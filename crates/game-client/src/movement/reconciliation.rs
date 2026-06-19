use super::interpolation::RemoteInterpolation;
use super::prediction::{PredictedState, PredictionHistory};
use crate::core::PlayerComponent;
use avian3d::prelude::*;
use bevy::prelude::*;
use game_core::character_controller::{self, CharacterVelocityY};
use game_core::components::{GroundedComponent, MovementSpeedComponent};
use game_core::movement::MoveInput;
use game_core::networking::NetworkId;
use game_core::networking::NetworkIdMapping;
use protocol::{
    client::MoveAction, primitives::YAW_QUANTIZATION_FACTOR, server::ServerMovementPayload,
};
use std::f32::consts::TAU;

const RECONCILIATION_THRESHOLD: f32 = 0.1;

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
                    tracing::info!("need correction");
                    let mut replay_pos = server_position;
                    let mut replay_yaw = server_yaw;
                    let mut replay_vy = predicted_at_tick
                        .as_ref()
                        .map(|p| p.velocity_y)
                        .unwrap_or(0.0);
                    let mut replay_grounded = predicted_at_tick
                        .as_ref()
                        .map(|p| p.grounded)
                        .unwrap_or(true);

                    let inputs: Vec<MoveAction> =
                        history.unacknowledged_inputs().cloned().collect();

                    history.clear_states();

                    for input in &inputs {
                        let move_input = MoveInput::from(input.clone());
                        let result = character_controller::character_move_step(
                            replay_pos,
                            replay_vy,
                            &move_input,
                            speed.0,
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
            } else if let Ok((mut transform, mut remote_interp)) = q_remote.get_mut(entity) {
                remote_interp.push(server_position, server_yaw, elapsed);

                transform.translation = server_position;
                transform.rotation = Quat::from_rotation_y(server_yaw);
            }
        }
    }
}
