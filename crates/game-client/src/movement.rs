use crate::application::{NetworkIdMapping, PlayerComponent};
use crate::camera::ThirdPersonCamera;
use crate::input::Movement;
use crate::tick_sync::TickSync;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use game_core::character_controller::{self, CharacterVelocityY, FIXED_DT};
use game_core::components::{GroundedComponent, MovementSpeedComponent, NetworkId};
use game_core::movement::MoveInput;
use protocol::{
    client::MoveAction, primitives::YAW_QUANTIZATION_FACTOR, server::ServerMovementPayload,
};
use std::collections::VecDeque;
use std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The distance threshold beyond which a server correction triggers reconciliation.
/// Below this, the client's prediction is trusted entirely.
const RECONCILIATION_THRESHOLD: f32 = 0.1;

/// Number of server snapshots to buffer for remote entity interpolation.
const REMOTE_SNAPSHOT_BUFFER_SIZE: usize = 8;

// ---------------------------------------------------------------------------
// Components — Local player prediction
// ---------------------------------------------------------------------------

/// Snapshot of a predicted position + rotation at a given tick.
#[derive(Debug, Clone)]
pub struct PredictedState {
    pub position: Vec3,
    pub yaw: f32,
    pub velocity_y: f32,
    pub grounded: bool,
}

/// Input and state history for client-side prediction and server reconciliation.
///
/// Each tick, the client records the `MoveAction` it sent and (after the movement
/// step) the resulting `PredictedState`. When the server sends an authoritative
/// state tagged with `last_client_tick`, the client:
/// 1. Discards acknowledged history
/// 2. Compares predicted vs server position at that tick
/// 3. If they diverge, snaps to the server state and **replays** all
///    unacknowledged inputs through the real `character_move_step` function
///    with full collision detection
///
/// Avian3d's built-in `TransformInterpolation` handles visual smoothing
/// between fixed ticks — this component only handles prediction/reconciliation.
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

    /// Returns an iterator over the remaining unacknowledged inputs.
    /// Used during input replay after a server correction.
    pub fn unacknowledged_inputs(&self) -> impl Iterator<Item = &MoveAction> {
        self.input_buffer.iter()
    }

    /// Clear all predicted states (but keep inputs) — used before replaying
    /// inputs after a server correction.
    pub fn clear_states(&mut self) {
        self.state_buffer.clear();
    }

    /// Whether the most recent input in the buffer has movement.
    fn latest_has_movement(&self) -> bool {
        self.input_buffer.back().map_or(false, |a| a.has_movement())
    }

    /// Whether the second-to-last input in the buffer had movement.
    /// Used to detect the moving -> idle transition for the stop packet.
    fn previous_had_movement(&self) -> bool {
        self.input_buffer
            .iter()
            .rev()
            .nth(1)
            .map_or(false, |a| a.has_movement())
    }
}

// ---------------------------------------------------------------------------
// Components — Remote entity interpolation
// ---------------------------------------------------------------------------

/// A timestamped position snapshot for remote entities.
#[derive(Debug, Clone)]
struct RemoteSnapshot {
    position: Vec3,
    yaw: f32,
    timestamp: f64,
}

/// Interpolation buffer for remote (non-local) actors.
///
/// Remote entities receive position updates at the server tick rate (20Hz).
/// Without interpolation, they would visually teleport between positions.
/// This buffer stores recent snapshots and interpolates between them,
/// introducing one tick of render delay for smooth movement.
///
/// Remote entities use `NoTransformEasing` to disable avian's built-in
/// interpolation, since their transforms are driven entirely by server updates
/// rather than physics simulation.
#[derive(Component, Debug)]
pub struct RemoteInterpolation {
    snapshots: VecDeque<RemoteSnapshot>,
}

impl Default for RemoteInterpolation {
    fn default() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(REMOTE_SNAPSHOT_BUFFER_SIZE),
        }
    }
}

impl RemoteInterpolation {
    /// Push a new server snapshot into the buffer.
    pub fn push(&mut self, position: Vec3, yaw: f32, timestamp: f64) {
        self.snapshots.push_back(RemoteSnapshot {
            position,
            yaw,
            timestamp,
        });

        while self.snapshots.len() > REMOTE_SNAPSHOT_BUFFER_SIZE {
            self.snapshots.pop_front();
        }
    }

    /// Sample the interpolated position at the given render time.
    pub fn sample(&self, render_time: f64) -> Option<(Vec3, f32)> {
        if self.snapshots.len() < 2 {
            return self.snapshots.back().map(|s| (s.position, s.yaw));
        }

        // Find the two snapshots that bracket `render_time`.
        for i in 0..self.snapshots.len() - 1 {
            let from = &self.snapshots[i];
            let to = &self.snapshots[i + 1];

            if render_time >= from.timestamp && render_time <= to.timestamp {
                let duration = to.timestamp - from.timestamp;
                if duration < f64::EPSILON {
                    return Some((to.position, to.yaw));
                }
                let t = ((render_time - from.timestamp) / duration) as f32;
                let t = t.clamp(0.0, 1.0);
                let pos = from.position.lerp(to.position, t);
                let yaw = lerp_angle(from.yaw, to.yaw, t);
                return Some((pos, yaw));
            }
        }

        // Past all snapshots — extrapolate from the last two.
        let from = &self.snapshots[self.snapshots.len() - 2];
        let to = &self.snapshots[self.snapshots.len() - 1];
        let duration = to.timestamp - from.timestamp;
        if duration < f64::EPSILON {
            return Some((to.position, to.yaw));
        }
        let t = ((render_time - from.timestamp) / duration) as f32;
        let t = t.clamp(0.0, 2.0);
        let pos = from.position.lerp(to.position, t);
        let yaw = lerp_angle(from.yaw, to.yaw, t);
        Some((pos, yaw))
    }
}

/// Lerp between two angles, taking the shortest path.
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut diff = (to - from) % TAU;
    if diff > std::f32::consts::PI {
        diff -= TAU;
    } else if diff < -std::f32::consts::PI {
        diff += TAU;
    }
    from + diff * t
}

// ---------------------------------------------------------------------------
// Systems — Local player prediction (FixedPreUpdate, before physics)
// ---------------------------------------------------------------------------

/// System that performs one tick of predicted character movement.
///
/// Runs in `FixedPreUpdate`. Uses the shared `character_move_step` function
/// with full collision detection (shape casts), producing identical results
/// to the server. The result is written directly to `Transform` — no
/// `LinearVelocity` is used because this is a kinematic character controller.
///
/// After the movement step, the input and resulting state are recorded in
/// `PredictionHistory` for later server reconciliation.
pub fn predict_player_movement(
    tick_sync: Res<TickSync>,
    movement_action: Single<(&Action<Movement>, &TriggerState), With<Action<Movement>>>,
    spatial_query: SpatialQuery,
    q_camera: Query<&ThirdPersonCamera>,
    mut q_player: Query<
        (
            Entity,
            &mut Transform,
            &Collider,
            &MovementSpeedComponent,
            &mut CharacterVelocityY,
            &mut PredictionHistory,
            Has<GroundedComponent>,
        ),
        With<PlayerComponent>,
    >,
    mut commands: Commands,
) {
    let current_tick = tick_sync.tick;

    let Ok((entity, mut transform, collider, speed, mut vel_y, mut history, grounded)) =
        q_player.single_mut()
    else {
        return;
    };

    // Read current input. Zero when not fired.
    let (action_value, trigger_state) = *movement_action;
    let movement_value = match trigger_state {
        TriggerState::Fired => **action_value,
        _ => Vec2::ZERO,
    };

    // Use the camera's yaw as the movement direction reference.
    // WASD moves relative to where the camera is facing.
    let yaw_rad = q_camera
        .single()
        .map(|cam| cam.yaw)
        .unwrap_or_else(|_| transform.rotation.to_euler(EulerRot::YXZ).1);
    let action = MoveAction::from_f32(yaw_rad, movement_value.y, movement_value.x, current_tick);

    // Run the shared movement step with full collision detection.
    let move_input = MoveInput::from(action.clone());
    let result = character_controller::character_move_step(
        transform.translation,
        vel_y.0,
        &move_input,
        speed.0,
        grounded,
        collider,
        entity,
        &spatial_query,
    );

    // Apply the result.
    transform.translation = result.position;
    transform.rotation = Quat::from_rotation_y(result.yaw);
    vel_y.0 = result.velocity_y;

    // Update grounded status.
    if result.grounded {
        commands.entity(entity).insert(GroundedComponent);
    } else {
        commands.entity(entity).remove::<GroundedComponent>();
    }

    // Record input and state for reconciliation.
    history.push_input(action);
    history.push_state(PredictedState {
        position: result.position,
        yaw: result.yaw,
        velocity_y: result.velocity_y,
        grounded: result.grounded,
    });
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

// ---------------------------------------------------------------------------
// Systems — Server reconciliation with full resimulation (Update)
// ---------------------------------------------------------------------------

/// System that receives authoritative transform updates from the server
/// and performs server reconciliation for the local player.
///
/// For the local player: compares the server position at `last_client_tick` against
/// our predicted state at that tick. If they diverge beyond the threshold:
/// 1. Snaps to the server's authoritative position
/// 2. **Replays all unacknowledged inputs** through the real `character_move_step`
///    function with full collision detection — producing an identical result to
///    what the server will compute
///
/// Writing to `Transform` outside of the fixed timestep schedules triggers
/// avian3d's teleport detection, which resets the interpolation easing state.
/// This means the correction appears instantly without lerping from the old
/// (wrong) position. Normal interpolation resumes on the next fixed tick.
///
/// For remote actors: pushes the server position into the interpolation buffer.
pub fn reconcile_with_server(
    mut client: ResMut<bevy_renet::RenetClient>,
    network_id_mapping: Res<NetworkIdMapping>,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    player_network_id: Query<Entity, With<PlayerComponent>>,
    mut q_player: Query<
        (
            Entity,
            &mut Transform,
            &Collider,
            &mut PredictionHistory,
            &MovementSpeedComponent,
            &mut CharacterVelocityY,
        ),
        With<PlayerComponent>,
    >,
    mut q_remote: Query<(&mut Transform, &mut RemoteInterpolation), Without<PlayerComponent>>,
    mut commands: Commands,
) {
    let Ok(player_entity) = player_network_id.single() else {
        return;
    };

    let elapsed = time.elapsed_secs_f64();

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

            let server_position = update.transform.position;
            let server_yaw = (update.transform.yaw as f32 / YAW_QUANTIZATION_FACTOR) * TAU;

            if entity == player_entity {
                // --- Local player reconciliation with full resimulation ---
                let Ok((player_ent, mut transform, collider, mut history, speed, mut vel_y)) =
                    q_player.single_mut()
                else {
                    continue;
                };

                let predicted_at_tick = history.acknowledge_up_to(last_client_tick);

                let needs_correction = predicted_at_tick
                    .as_ref()
                    .map(|predicted| {
                        predicted.position.distance(server_position) > RECONCILIATION_THRESHOLD
                    })
                    .unwrap_or(true);

                if needs_correction {
                    // Start from the server's authoritative state.
                    let mut replay_pos = server_position;
                    let mut replay_yaw = server_yaw;
                    // Use the predicted velocity_y at the acknowledged tick if available,
                    // otherwise start from the server position (assume standing).
                    let mut replay_vy = predicted_at_tick
                        .as_ref()
                        .map(|p| p.velocity_y)
                        .unwrap_or(0.0);
                    let mut replay_grounded = predicted_at_tick
                        .as_ref()
                        .map(|p| p.grounded)
                        .unwrap_or(true);

                    // Collect unacknowledged inputs for replay.
                    let inputs: Vec<MoveAction> =
                        history.unacknowledged_inputs().cloned().collect();

                    // Clear old predicted states — we'll rebuild them from replay.
                    history.clear_states();

                    // Replay each unacknowledged input through the real movement function
                    // with full collision detection.
                    for input in &inputs {
                        let move_input = MoveInput::from(input.clone());
                        let result = character_controller::character_move_step(
                            replay_pos,
                            replay_vy,
                            &move_input,
                            speed.0,
                            replay_grounded,
                            collider,
                            player_ent,
                            &spatial_query,
                        );

                        replay_pos = result.position;
                        replay_yaw = result.yaw;
                        replay_vy = result.velocity_y;
                        replay_grounded = result.grounded;

                        history.push_state(PredictedState {
                            position: result.position,
                            yaw: result.yaw,
                            velocity_y: result.velocity_y,
                            grounded: result.grounded,
                        });
                    }

                    // Apply the corrected predicted position.
                    transform.translation = replay_pos;
                    transform.rotation = Quat::from_rotation_y(replay_yaw);
                    vel_y.0 = replay_vy;

                    if replay_grounded {
                        commands.entity(player_ent).insert(GroundedComponent);
                    } else {
                        commands.entity(player_ent).remove::<GroundedComponent>();
                    }

                    tracing::debug!(
                        ?server_position,
                        ?replay_pos,
                        remaining_inputs = inputs.len(),
                        tick = last_client_tick,
                        "server correction with resimulation"
                    );
                }
            } else {
                // --- Remote entity: push into interpolation buffer ---
                if let Ok((mut transform, mut remote_interp)) = q_remote.get_mut(entity) {
                    remote_interp.push(server_position, server_yaw, elapsed);

                    // Also update the physics transform so collision queries
                    // against remote entities have a reasonable position.
                    transform.translation = server_position;
                    transform.rotation = Quat::from_rotation_y(server_yaw);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Systems — Remote entity visual interpolation (Update)
// ---------------------------------------------------------------------------

/// System that interpolates remote entities between buffered server snapshots.
///
/// Remote actors are rendered one tick behind the latest server update,
/// providing a smooth interpolation window.
pub fn interpolate_remote_actors(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &RemoteInterpolation), Without<PlayerComponent>>,
) {
    let render_time = time.elapsed_secs_f64() - FIXED_DT as f64;

    for (mut transform, remote_interp) in query.iter_mut() {
        if let Some((pos, yaw)) = remote_interp.sample(render_time) {
            transform.translation = pos;
            transform.rotation = Quat::from_rotation_y(yaw);
        }
    }
}
