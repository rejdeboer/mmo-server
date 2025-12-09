use crate::{
    components::ClientIdComponent,
    messages::{IncomingChatMessage, JumpActionMessage, MoveActionMessage},
    telemetry::Metrics,
};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};
use flatbuffers::root;
use schemas::game as schema;

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
    match root::<schema::BatchedActions>(&message) {
        Ok(batch) => {
            for action in batch.actions().unwrap() {
                match action.data_type() {
                    schema::ActionData::game_ClientChatMessage => {
                        let chat_message = action.data_as_game_client_chat_message().unwrap();
                        commands.write_message(IncomingChatMessage {
                            author: entity,
                            channel: chat_message.channel(),
                            text: chat_message.text().to_string(),
                        });
                    }
                    schema::ActionData::PlayerMoveAction => {
                        process_player_move_action(
                            entity,
                            action.data_as_player_move_action().unwrap(),
                            commands,
                        );
                    }
                    schema::ActionData::PlayerJumpAction => {
                        process_player_jump_action(entity, commands);
                    }
                    _ => {
                        tracing::warn!("unhandled event data type");
                    }
                }
            }
        }
        Err(error) => {
            tracing::error!(?error, "message does not follow event schema");
        }
    }
}

fn process_player_move_action(
    entity: Entity,
    action: schema::PlayerMoveAction,
    commands: &mut Commands,
) {
    commands.write_message(MoveActionMessage {
        entity,
        yaw: action.yaw(),
        forward: action.forward(),
        sideways: action.sideways(),
    });
}

fn process_player_jump_action(entity: Entity, commands: &mut Commands) {
    commands.write_message(JumpActionMessage { entity });
}
