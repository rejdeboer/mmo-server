use crate::{
    components::{ClientIdComponent, LastClientTick, ServerTick},
    messages::{CastSpellActionMessage, IncomingChatMessage, JumpActionMessage, MoveActionMessage},
    telemetry::Metrics,
};
use bevy::prelude::*;
use bevy_renet::{RenetServer, renet::DefaultChannel};
use protocol::client::{MoveAction, PlayerAction};
use protocol::server::ServerEvent;

pub fn process_client_actions(
    mut server: ResMut<RenetServer>,
    server_tick: Res<ServerTick>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
    metrics: Res<Metrics>,
    mut encode_buffer: Local<bitcode::Buffer>,
) {
    for (entity, client_id) in clients.iter() {
        while let Some(message) =
            server.receive_message(client_id.0, DefaultChannel::ReliableOrdered)
        {
            metrics
                .network_packets_total
                .with_label_values(&["incoming", "reliable"])
                .inc();
            metrics
                .network_bytes_total
                .with_label_values(&["incoming", "reliable"])
                .inc_by(message.len() as u64);

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
    mut clients: Query<(Entity, &ClientIdComponent, &mut LastClientTick)>,
    metrics: Res<Metrics>,
    mut writer: MessageWriter<MoveActionMessage>,
) {
    for (entity, client_id, mut last_client_tick) in clients.iter_mut() {
        let mut latest_action: Option<MoveAction> = None;

        while let Some(message) = server.receive_message(client_id.0, DefaultChannel::Unreliable) {
            metrics
                .network_packets_total
                .with_label_values(&["incoming", "unreliable"])
                .inc();
            metrics
                .network_bytes_total
                .with_label_values(&["incoming", "unreliable"])
                .inc_by(message.len() as u64);

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
            process_player_movement(entity, action, &mut writer);
        }
    }
}

fn process_player_action(
    entity: Entity,
    client_id: u64,
    action: PlayerAction,
    commands: &mut Commands,
    server: &mut ResMut<RenetServer>,
    current_server_tick: u32,
    encode_buffer: &mut Local<bitcode::Buffer>,
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
            target_entity_id,
        } => {
            commands.write_message(CastSpellActionMessage {
                caster_entity: entity,
                target_entity: Entity::from_bits(target_entity_id),
                spell_id,
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

fn process_player_movement(
    entity: Entity,
    action: MoveAction,
    writer: &mut MessageWriter<MoveActionMessage>,
) {
    writer.write(MoveActionMessage { entity, action });
}
