use crate::{components::ClientIdComponent, events::IncomingChatMessage};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer};
use flatbuffers::root;
use schemas::game as schema;

pub fn process_client_actions(
    mut server: ResMut<RenetServer>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
) {
    for (entity, client_id) in clients.iter() {
        if let Some(message) = server.receive_message(client_id.0, DefaultChannel::Unreliable) {
            process_message(entity, message, &mut commands);
        } else if let Some(message) =
            server.receive_message(client_id.0, DefaultChannel::ReliableOrdered)
        {
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
                        commands.send_event(IncomingChatMessage {
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
                    _ => {
                        warn!("unhandled event data type");
                    }
                }
            }
        }
        Err(error) => {
            error!(?error, "message does not follow event schema");
        }
    }
}

fn process_player_move_action(
    entity: Entity,
    action: schema::PlayerMoveAction,
    commands: &mut Commands,
) {
    let fb_transform = action.transform().unwrap();
    let pos = fb_transform.position();
    let transform = Transform::from_xyz(pos.x(), pos.y(), pos.z())
        .with_rotation(Quat::from_rotation_y(fb_transform.yaw()));
    commands.entity(entity).insert(transform);
}
