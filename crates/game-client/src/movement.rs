use crate::application::{NetworkIdMapping, PlayerComponent};
use crate::input::Movement;
use crate::tick_sync::TickSync;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use game_core::components::{MovementSpeedComponent, NetworkId};
use game_core::movement::MoveInput;
use protocol::{
    client::MoveAction, primitives::YAW_QUANTIZATION_FACTOR, server::ServerMovementPayload,
};
use std::collections::VecDeque;
use std::f32::consts::TAU;

/// Snapshot of a predicted position + rotation at a given tick, used for reconciliation.
#[derive(Debug, Clone)]
pub struct PredictedState {
    pub position: Vec3,
    pub yaw: f32,
}

/// Per-entity component that stores the input and state history for client-side prediction.
///
/// Inspired by lightyear's prediction approach:
/// - Each tick, the client records the input it sent and the resulting predicted state.
/// - When the server sends back an authoritative state tagged with `last_client_tick`,
///   the client discards all history up to that tick, checks if the server state
///   diverges from the predicted state, and if so re-simulates from the server state
///   by replaying all unconfirmed inputs.
///
/// ## Visual vs. Simulation State
///
/// The `predicted_position` and `predicted_yaw` fields track the authoritative predicted
/// state advanced at FixedUpdate rate (one tick at a time). These are used for reconciliation.
///
/// The entity's `Transform` is the *visual* state, updated every render frame by the
/// `Fire<Movement>` observer for instant responsiveness. On reconciliation, both the
/// predicted state and Transform are snapped to the corrected position.
#[derive(Component, Debug)]
pub struct PredictionHistory {
    /// Inputs that have been sent but not yet confirmed by the server.
    input_buffer: VecDeque<MoveAction>,
    /// Predicted states corresponding to the ticks in the input buffer.
    state_buffer: VecDeque<PredictedState>,
    /// Maximum number of ticks to keep in the buffers.
    max_buffer_size: usize,
    /// The current predicted position, advanced each FixedUpdate tick.
    pub predicted_position: Vec3,
    /// The current predicted yaw, advanced each FixedUpdate tick.
    pub predicted_yaw: f32,
    /// Input stashed by the `Fire<Movement>` observer for the next FixedUpdate tick.
    /// `None` means no movement input arrived since the last tick (player idle or released).
    pub pending_input: Option<Vec2>,
    /// Whether the previous tick had movement input. Used to detect the transition
    /// from moving → idle so we can send one final zero-velocity packet to the server.
    had_input_last_tick: bool,
    /// Set by `predict_player_movement`, read by `send_player_input`.
    /// True when we need to send the latest input to the server.
    should_send: bool,
}

impl PredictionHistory {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            input_buffer: VecDeque::new(),
            state_buffer: VecDeque::new(),
            max_buffer_size: 256,
            predicted_position: position,
            predicted_yaw: yaw,
            pending_input: None,
            had_input_last_tick: false,
            should_send: false,
        }
    }

    pub fn push(&mut self, input: MoveAction, predicted_state: PredictedState) {
        self.input_buffer.push_back(input);
        self.state_buffer.push_back(predicted_state);

        while self.input_buffer.len() > self.max_buffer_size {
            self.input_buffer.pop_front();
            self.state_buffer.pop_front();
        }
    }

    /// Discard all history up to and including the given tick.
    /// Returns the predicted state at that tick (if it existed), so the caller
    /// can compare it with the server's authoritative state.
    pub fn acknowledge_up_to(&mut self, tick: u32) -> Option<PredictedState> {
        let mut confirmed_state = None;

        while let Some(front) = self.input_buffer.front() {
            if front.tick <= tick {
                confirmed_state = self.state_buffer.pop_front();
                self.input_buffer.pop_front();
            } else {
                break;
            }
        }

        confirmed_state
    }

    pub fn unconfirmed_inputs(&self) -> impl Iterator<Item = &MoveAction> {
        self.input_buffer.iter()
    }
}

/// The distance threshold beyond which a server correction triggers a full rollback
/// Below this threshold we assume the prediction was close enough
pub const RECONCILIATION_THRESHOLD: f32 = 0.1;

/// Observer that applies movement to the visual `Transform` every render frame
/// and stashes the input for the next FixedUpdate tick.
///
/// `Fire<Movement>` fires every frame the action is in `TriggerState::Fired` (i.e.,
/// the player is holding a movement key). This gives instant visual feedback at display
/// framerate, independent of the 20Hz FixedUpdate tick.
///
/// The stashed `pending_input` is consumed by `predict_player_movement` in FixedUpdate,
/// ensuring the tick simulation always sees the input even if the player releases the
/// key between FixedUpdate ticks.
pub fn apply_movement(
    event: On<Fire<Movement>>,
    time: Res<Time>,
    mut q_player: Query<
        (
            &mut Transform,
            &MovementSpeedComponent,
            &mut PredictionHistory,
        ),
        With<PlayerComponent>,
    >,
) {
    let Ok((mut transform, speed, mut history)) = q_player.get_mut(event.context) else {
        return;
    };

    let movement_value: Vec2 = event.value;
    let (_, yaw_rad, _) = transform.rotation.to_euler(EulerRot::YXZ);

    let input = MoveInput {
        yaw: yaw_rad,
        forward: movement_value.y,
        sideways: movement_value.x,
    };

    let velocity = input.target_velocity(speed.0);
    let dt = time.delta_secs();

    transform.translation += velocity * dt;
    history.pending_input = Some(movement_value);
}

/// System that advances the predicted state by one tick in `FixedUpdate`.
pub fn predict_player_movement(
    tick_sync: Res<TickSync>,
    time: Res<Time<Fixed>>,
    mut q_player: Query<
        (&Transform, &MovementSpeedComponent, &mut PredictionHistory),
        With<PlayerComponent>,
    >,
) {
    let dt = time.delta_secs();
    let current_tick = tick_sync.tick;

    let Ok((transform, speed, mut history)) = q_player.single_mut() else {
        return;
    };

    // Consume the stashed input from the observer. If none arrived, player is idle.
    let movement_value = history.pending_input.take().unwrap_or(Vec2::ZERO);

    // Use the visual yaw for input quantization (camera orientation).
    let (_, yaw_rad, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let input = MoveAction::from_f32(yaw_rad, movement_value.y, movement_value.x, current_tick);

    // Track whether this tick had movement, for the stop-packet logic in send_player_input.
    let has_movement = input.has_movement();
    // Send if moving, or if this is the first idle tick (stop packet).
    history.should_send = has_movement || history.had_input_last_tick;
    history.had_input_last_tick = has_movement;

    // Advance the predicted state (not the visual Transform).
    let (new_position, new_yaw) =
        apply_movement_input(history.predicted_position, &input, speed.0, dt);

    history.predicted_position = new_position;
    history.predicted_yaw = new_yaw;

    // Record the input and the resulting predicted state.
    history.push(
        input,
        PredictedState {
            position: new_position,
            yaw: new_yaw,
        },
    );
}

/// System that sends the latest predicted input to the server.
///
/// Runs after `predict_player_movement` in FixedUpdate so we send the input
/// that was just recorded. Sends when the player is actively moving, plus
/// one final zero-velocity packet when they stop (so the server zeros velocity).
pub fn send_player_input(
    mut client: ResMut<bevy_renet::RenetClient>,
    query: Query<&mut PredictionHistory, With<PlayerComponent>>,
) {
    let Ok(history) = query.single() else {
        return;
    };

    if !history.should_send {
        return;
    }

    let Some(action) = history.input_buffer.back() else {
        return;
    };

    let encoded = bitcode::encode(action);
    client.send_message(bevy_renet::renet::DefaultChannel::Unreliable, encoded);
}

/// System that receives authoritative transform updates from the server
/// and performs server reconciliation for the local player.
///
/// For remote actors (non-player entities), it simply applies the server position directly.
pub fn reconcile_with_server(
    mut client: ResMut<bevy_renet::RenetClient>,
    time: Res<Time<Fixed>>,
    network_id_mapping: Res<NetworkIdMapping>,
    player_network_id: Query<Entity, With<PlayerComponent>>,
    mut transforms: Query<(
        &mut Transform,
        &MovementSpeedComponent,
        Option<&mut PredictionHistory>,
    )>,
) {
    let Ok(player_entity) = player_network_id.single() else {
        tracing::error!("could not retrieve player entity");
        return;
    };

    let dt = time.delta_secs();

    while let Some(message) = client.receive_message(bevy_renet::renet::DefaultChannel::Unreliable)
    {
        let payload = match bitcode::decode::<ServerMovementPayload>(&message) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("received invalid ServerMovementPayload: {}", e);
                continue;
            }
        };

        handle_movement_payload(
            &payload,
            &network_id_mapping,
            player_entity,
            &mut transforms,
            dt,
        );
    }
}

fn handle_movement_payload(
    payload: &ServerMovementPayload,
    network_id_mapping: &NetworkIdMapping,
    player_entity: Entity,
    transforms: &mut Query<(
        &mut Transform,
        &MovementSpeedComponent,
        Option<&mut PredictionHistory>,
    )>,
    dt: f32,
) {
    let last_client_tick = payload.last_client_tick;

    for update in &payload.updates {
        let net_id = NetworkId(update.actor_id);
        let Some(&entity) = network_id_mapping.0.get(&net_id) else {
            continue;
        };

        let Ok((mut transform, speed, prediction_history)) = transforms.get_mut(entity) else {
            continue;
        };

        if entity == player_entity {
            if let Some(mut history) = prediction_history {
                let server_position = update.transform.position;
                let server_yaw = (update.transform.yaw as f32 / YAW_QUANTIZATION_FACTOR) * TAU;

                let predicted_at_tick = history.acknowledge_up_to(last_client_tick);

                // Check if our prediction at that tick was accurate.
                let needs_correction = predicted_at_tick
                    .map(|predicted| {
                        predicted.position.distance(server_position) > RECONCILIATION_THRESHOLD
                    })
                    .unwrap_or(true);

                if needs_correction {
                    // Snap to server state and replay all unconfirmed inputs.
                    let mut pos = server_position;
                    let mut yaw = server_yaw;

                    for action in history.unconfirmed_inputs() {
                        let result = apply_movement_input(pos, action, speed.0, dt);
                        pos = result.0;
                        yaw = result.1;
                    }

                    // Snap both the predicted state and the visual Transform.
                    history.predicted_position = pos;
                    history.predicted_yaw = yaw;
                    transform.translation = pos;
                    transform.rotation = Quat::from_rotation_y(yaw);
                }
            }
        } else {
            transform.translation = update.transform.position;
            transform.rotation = update.transform.get_quat();
        }
    }
}

pub fn apply_movement_input(
    position: Vec3,
    action: &MoveAction,
    movement_speed: f32,
    dt: f32,
) -> (Vec3, f32) {
    let input = MoveInput::from(action.clone());
    let velocity = input.target_velocity(movement_speed);
    let new_position = position + velocity * dt;
    (new_position, input.yaw)
}
