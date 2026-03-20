use crate::application::{NetworkIdMapping, PlayerComponent};
use crate::input::Movement;
use crate::tick_sync::TickSync;
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use game_core::components::{MovementSpeedComponent, NetworkId};
use game_core::movement::MoveInput;
use protocol::{
    client::MoveAction, primitives::YAW_QUANTIZATION_FACTOR, server::ServerMovementPayload,
};
use std::collections::VecDeque;
use std::f32::consts::TAU;

/// Stores the previous tick's position and yaw for visual interpolation.
#[derive(Component, Debug)]
pub struct InterpolationState {
    pub previous_position: Vec3,
    pub previous_yaw: f32,
}

impl InterpolationState {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            previous_position: position,
            previous_yaw: yaw,
        }
    }
}

/// Snapshot of a predicted position + rotation at a given tick.
#[derive(Debug, Clone)]
pub struct PredictedState {
    pub position: Vec3,
    pub yaw: f32,
}

/// The distance threshold beyond which a server correction triggers a snap.
pub const RECONCILIATION_THRESHOLD: f32 = 0.1;

/// Input and state history for client-side prediction and server reconciliation.
///
/// Each tick, the client records the `MoveAction` it sent and (after physics)
/// the resulting `PredictedState`. When the server sends an authoritative state
/// tagged with `last_client_tick`, the client discards history up to that tick
/// and compares the predicted vs. server position. If they diverge beyond
/// `RECONCILIATION_THRESHOLD`, the client snaps to the server state.
#[derive(Component, Debug)]
pub struct PredictionHistory {
    input_buffer: VecDeque<MoveAction>,
    state_buffer: VecDeque<PredictedState>,
    max_buffer_size: usize,
}

impl Default for PredictionHistory {
    fn default() -> Self {
        Self {
            input_buffer: VecDeque::new(),
            state_buffer: VecDeque::new(),
            max_buffer_size: 256,
        }
    }
}

impl PredictionHistory {
    pub fn push_input(&mut self, input: MoveAction) {
        self.input_buffer.push_back(input);

        while self.input_buffer.len() > self.max_buffer_size {
            self.input_buffer.pop_front();
            if !self.state_buffer.is_empty() {
                self.state_buffer.pop_front();
            }
        }
    }

    pub fn push_state(&mut self, state: PredictedState) {
        self.state_buffer.push_back(state);
    }

    /// Discard all history up to and including the given tick.
    /// Returns the predicted state at that tick (if it existed), so the caller
    /// can compare it with the server's authoritative state.
    pub fn acknowledge_up_to(&mut self, tick: u32) -> Option<PredictedState> {
        let mut confirmed_state = None;

        while let Some(front) = self.input_buffer.front() {
            if front.tick <= tick {
                self.input_buffer.pop_front();
                confirmed_state = self.state_buffer.pop_front();
            } else {
                break;
            }
        }

        confirmed_state
    }

    /// Whether the most recent input in the buffer has movement.
    fn latest_has_movement(&self) -> bool {
        self.input_buffer.back().map_or(false, |a| a.has_movement())
    }

    /// Whether the second-to-last input in the buffer had movement.
    /// Used to detect the moving → idle transition for the stop packet.
    fn previous_had_movement(&self) -> bool {
        self.input_buffer
            .iter()
            .rev()
            .nth(1)
            .map_or(false, |a| a.has_movement())
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// System that sets `LinearVelocity` and rotation from the latest input.
///
/// Runs in `FixedPreUpdate` (before physics), mirroring the server's
/// `process_move_action_messages`. Reads the current `Action<Movement>` value
/// directly — no intermediate stash needed because `bevy_enhanced_input`
/// persists the action value while the trigger state is `Fired`.
pub fn predict_player_movement(
    tick_sync: Res<TickSync>,
    movement_action: Single<(&Action<Movement>, &TriggerState), With<Action<Movement>>>,
    mut q_player: Query<
        (
            &Transform,
            &MovementSpeedComponent,
            &mut LinearVelocity,
            &mut PredictionHistory,
        ),
        With<PlayerComponent>,
    >,
) {
    let current_tick = tick_sync.tick;

    let Ok((transform, speed, mut velocity, mut history)) = q_player.single_mut() else {
        return;
    };

    // Read current input directly from the action. Zero when not fired.
    let (action_value, trigger_state) = *movement_action;
    let movement_value = match trigger_state {
        TriggerState::Fired => **action_value,
        _ => Vec2::ZERO,
    };

    // Use the visual yaw for input quantization (camera orientation).
    let (_, yaw_rad, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let action = MoveAction::from_f32(yaw_rad, movement_value.y, movement_value.x, current_tick);

    // Set LinearVelocity, mirroring the server's process_move_action_messages.
    let move_input = MoveInput::from(action.clone());
    let target_velocity = move_input.target_velocity(speed.0);
    velocity.x = target_velocity.x;
    velocity.z = target_velocity.z;
    // Leave velocity.y untouched — gravity is handled by avian3d.

    // Record the input. The corresponding state will be recorded in record_predicted_state
    // after physics has stepped.
    history.push_input(action);
}

/// System that snapshots the post-physics position into the prediction history
/// and updates interpolation state.
///
/// Runs in `FixedPostUpdate` after `PhysicsSystems::Last`, so `Transform` reflects
/// the resolved physics position (with collisions, gravity, etc.).
pub fn record_predicted_state(
    mut q_player: Query<
        (&Transform, &mut PredictionHistory, &mut InterpolationState),
        With<PlayerComponent>,
    >,
) {
    let Ok((transform, mut history, mut interp)) = q_player.single_mut() else {
        return;
    };

    let (_, yaw, _) = transform.rotation.to_euler(EulerRot::YXZ);

    history.push_state(PredictedState {
        position: transform.translation,
        yaw,
    });

    interp.previous_position = transform.translation;
    interp.previous_yaw = yaw;
}

/// System that sends the latest predicted input to the server.
///
/// Sends when the player is actively moving, plus one final zero-velocity
/// packet when they stop (so the server zeros velocity).
pub fn send_player_input(
    mut client: ResMut<bevy_renet::RenetClient>,
    query: Query<&PredictionHistory, With<PlayerComponent>>,
) {
    let Ok(history) = query.single() else {
        return;
    };

    let Some(action) = history.input_buffer.back() else {
        return;
    };

    // Send if moving, or if this is the first idle tick after movement (stop packet).
    let should_send = history.latest_has_movement() || history.previous_had_movement();
    if !should_send {
        return;
    }

    let encoded = bitcode::encode(action);
    client.send_message(bevy_renet::renet::DefaultChannel::Unreliable, encoded);
}

/// System that interpolates the player's visual `Transform` between the previous
/// and current tick positions using `Time<Fixed>::overstep_fraction()`.
///
/// This produces smooth movement at display framerate despite the 20Hz tick rate.
/// The visual position is always between two known physics states — no extrapolation.
pub fn interpolate_player(
    time: Res<Time<Fixed>>,
    mut q_player: Query<(&mut Transform, &InterpolationState), With<PlayerComponent>>,
) {
    let Ok((mut transform, interp)) = q_player.single_mut() else {
        return;
    };

    let t = time.overstep_fraction();

    transform.translation = interp.previous_position.lerp(transform.translation, t);
    let prev_rot = Quat::from_rotation_y(interp.previous_yaw);
    let curr_rot = transform.rotation;
    transform.rotation = prev_rot.slerp(curr_rot, t);
}

/// System that receives authoritative transform updates from the server
/// and performs server reconciliation for the local player.
///
/// For the local player: compares the server position at `last_client_tick` against
/// our predicted state at that tick. If the error exceeds the threshold, snaps
/// the player to the server state.
///
/// For remote actors: directly applies the server position.
pub fn reconcile_with_server(
    mut client: ResMut<bevy_renet::RenetClient>,
    network_id_mapping: Res<NetworkIdMapping>,
    player_network_id: Query<Entity, With<PlayerComponent>>,
    mut q_actors: Query<(
        &mut Transform,
        Option<&mut PredictionHistory>,
        Option<&mut InterpolationState>,
    )>,
) {
    let Ok(player_entity) = player_network_id.single() else {
        return;
    };

    while let Some(message) = client.receive_message(bevy_renet::renet::DefaultChannel::Unreliable)
    {
        let payload = match bitcode::decode::<ServerMovementPayload>(&message) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("received invalid ServerMovementPayload: {}", e);
                continue;
            }
        };

        let last_client_tick = payload.last_client_tick;

        for update in &payload.updates {
            let net_id = NetworkId(update.actor_id);
            let Some(&entity) = network_id_mapping.0.get(&net_id) else {
                continue;
            };

            let Ok((mut transform, prediction_history, interpolation_state)) =
                q_actors.get_mut(entity)
            else {
                continue;
            };

            let server_position = update.transform.position;
            let server_yaw = (update.transform.yaw as f32 / YAW_QUANTIZATION_FACTOR) * TAU;

            if entity == player_entity {
                if let Some(mut history) = prediction_history {
                    let predicted_at_tick = history.acknowledge_up_to(last_client_tick);

                    let needs_correction = predicted_at_tick
                        .map(|predicted| {
                            predicted.position.distance(server_position) > RECONCILIATION_THRESHOLD
                        })
                        .unwrap_or(true);

                    if needs_correction {
                        transform.translation = server_position;
                        transform.rotation = Quat::from_rotation_y(server_yaw);

                        if let Some(mut interp) = interpolation_state {
                            interp.previous_position = server_position;
                            interp.previous_yaw = server_yaw;
                        }

                        tracing::debug!(
                            ?server_position,
                            tick = last_client_tick,
                            "server correction applied"
                        );
                    }
                }
            } else {
                transform.translation = server_position;
                transform.rotation = Quat::from_rotation_y(server_yaw);
            }
        }
    }
}
