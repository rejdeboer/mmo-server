use crate::{
    components::ClientIdComponent,
    messages::{CastSpellActionMessage, IncomingChatMessage, JumpActionMessage, MoveActionMessage},
    telemetry::Metrics,
};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};
use protocol::client::{MoveAction, PlayerAction};

pub fn process_client_actions(
    mut server: ResMut<RenetServer>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
    metrics: Res<Metrics>,
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
                    process_player_action(entity, action, &mut commands);
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
    clients: Query<(Entity, &ClientIdComponent)>,
    metrics: Res<Metrics>,
    mut writer: MessageWriter<MoveActionMessage>,
) {
    for (entity, client_id) in clients.iter() {
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
                    process_player_movement(entity, action, &mut writer);
                }
                Err(error) => {
                    tracing::error!(?error, "movement message does not follow action schema");
                }
            }
        }
    }
}

fn process_player_action(entity: Entity, action: PlayerAction, commands: &mut Commands) {
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
    }
}

fn process_player_movement(
    entity: Entity,
    action: MoveAction,
    writer: &mut MessageWriter<MoveActionMessage>,
) {
    writer.write(MoveActionMessage {
        entity,
        yaw: action.yaw,
        forward: action.forward,
        sideways: action.sideways,
    });
}
