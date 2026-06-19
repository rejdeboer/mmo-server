use crate::{
    combat::{CastSpellActionMessage, StartAttackMessage, StopAttackMessage},
    core::{ClientIdComponent, LastClientTick, ServerTick},
    social::IncomingChatMessage,
    telemetry::{NETWORK_BYTES_TOTAL_METRIC, NETWORK_PACKETS_TOTAL_METRIC},
    world::{JumpActionMessage, MoveActionMessage},
};
use bevy::prelude::*;
use bevy_renet::{RenetServer, renet::DefaultChannel};
use game_core::networking::NetworkId;
use game_core::{components::MovementSpeedComponent, networking::NetworkIdMapping};
use protocol::client::{MoveAction, PlayerAction};
use protocol::server::ServerEvent;

pub fn process_client_actions(
    mut server: ResMut<RenetServer>,
    server_tick: Res<ServerTick>,
    net_entity_map: Res<NetworkIdMapping>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
    mut encode_buffer: Local<bitcode::Buffer>,
) {
    for (entity, client_id) in clients.iter() {
        while let Some(message) =
            server.receive_message(client_id.0, DefaultChannel::ReliableOrdered)
        {
            metrics::counter!(
                NETWORK_PACKETS_TOTAL_METRIC,
                "direction" => "incoming",
                "channel" => "reliable"
            )
            .increment(1);
            metrics::counter!(
                NETWORK_BYTES_TOTAL_METRIC,
                "direction" => "incoming",
                "channel" => "reliable"
            )
            .increment(message.len() as u64);

            match bitcode::decode::<PlayerAction>(&message) {
                Ok(action) => {
                    process_player_action(
                        entity,
                        client_id.0,
                        action,
                        &mut commands,
                        &mut server,
                        server_tick.0,
                        &mut encode_buffer,
                        &net_entity_map,
                    );
                }
                Err(error) => {
                    tracing::error!(?error, "action message does not follow action schema");
                }
            }
        }
    }
}

pub fn process_client_movements(
    mut server: ResMut<RenetServer>,
    mut clients: Query<
        (Entity, &ClientIdComponent, &mut LastClientTick),
        With<MovementSpeedComponent>,
    >,
    mut writer: MessageWriter<MoveActionMessage>,
) {
    for (entity, client_id, mut last_client_tick) in clients.iter_mut() {
        let mut latest_action: Option<MoveAction> = None;

        while let Some(message) = server.receive_message(client_id.0, DefaultChannel::Unreliable) {
            metrics::counter!(
                NETWORK_PACKETS_TOTAL_METRIC,
                "direction" => "incoming",
                "channel" => "unreliable"
            )
            .increment(1);
            metrics::counter!(
                NETWORK_BYTES_TOTAL_METRIC,
                "direction" => "incoming",
                "channel" => "unreliable"
            )
            .increment(message.len() as u64);

            match bitcode::decode::<MoveAction>(&message) {
                Ok(action) => {
                    latest_action = Some(action);
                }
                Err(error) => {
                    tracing::error!(?error, "movement message does not follow action schema");
                }
            }
        }

        // NOTE: We only handle the latest move action to prevent flooding
        if let Some(action) = latest_action {
            last_client_tick.0 = action.tick;
            writer.write(MoveActionMessage { entity, action });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_player_action(
    entity: Entity,
    client_id: u64,
    action: PlayerAction,
    commands: &mut Commands,
    server: &mut ResMut<RenetServer>,
    current_server_tick: u32,
    encode_buffer: &mut Local<bitcode::Buffer>,
    net_entity_map: &NetworkIdMapping,
) {
    match action {
        PlayerAction::Chat { channel, text } => {
            commands.write_message(IncomingChatMessage {
                author: entity,
                channel,
                text,
            });
        }
        PlayerAction::Jump => {
            commands.write_message(JumpActionMessage { entity });
        }
        PlayerAction::CastSpell {
            spell_id,
            target_network_id,
        } => {
            let Some(target_entity) = net_entity_map.0.get(&NetworkId(target_network_id)).copied()
            else {
                tracing::warn!(
                    %target_network_id,
                    "client sent CastSpell with unknown network ID"
                );
                return;
            };
            commands.write_message(CastSpellActionMessage {
                caster_entity: entity,
                target_entity,
                spell_id,
            });
        }
        PlayerAction::StartAttack { target_network_id } => {
            let Some(target_entity) = net_entity_map.0.get(&NetworkId(target_network_id)).copied()
            else {
                tracing::warn!(
                    %target_network_id,
                    "client sent StartAttack with unknown network ID"
                );
                return;
            };
            commands.write_message(StartAttackMessage {
                attacker_entity: entity,
                target_entity,
            });
        }
        PlayerAction::StopAttack => {
            commands.write_message(StopAttackMessage {
                attacker_entity: entity,
            });
        }
        PlayerAction::Ping { client_tick } => {
            let pong = ServerEvent::Pong {
                client_tick,
                server_tick: current_server_tick,
            };
            let data = encode_buffer.encode(&pong);
            server.send_message(client_id, DefaultChannel::ReliableOrdered, data.to_vec());
        }
    }
}
