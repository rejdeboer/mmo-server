use crate::{
    components::ClientIdComponent,
    messages::{CastSpellActionMessage, IncomingChatMessage, JumpActionMessage, MoveActionMessage},
    telemetry::Metrics,
};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};
use protocol::client::PlayerAction;

pub fn process_client_actions(
    mut server: ResMut<RenetServer>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
    metrics: Res<Metrics>,
) {
    for (entity, client_id) in clients.iter() {
        if let Some(message) = server.receive_message(client_id.0, DefaultChannel::Unreliable) {
            metrics
                .network_packets_total
                .with_label_values(&["incoming", "unreliable"])
                .inc();
            metrics
                .network_bytes_total
                .with_label_values(&["incoming", "unreliable"])
                .inc_by(message.len() as u64);
            process_message(entity, message, &mut commands);
        } else if let Some(message) =
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
            process_message(entity, message, &mut commands);
        }
    }
}

fn process_message(entity: Entity, message: bevy_renet::renet::Bytes, commands: &mut Commands) {
    match bitcode::decode::<Vec<PlayerAction>>(&message) {
        Ok(actions) => {
            for action in actions {
                process_player_action(entity, action, commands);
            }
        }
        Err(error) => {
            tracing::error!(?error, "message does not follow action schema");
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
        PlayerAction::Movement {
            yaw,
            forward,
            sideways,
        } => {
            commands.write_message(MoveActionMessage {
                entity,
                yaw,
                forward,
                sideways,
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
        _ => {
            tracing::warn!("unhandled event data type");
        }
    }
}
