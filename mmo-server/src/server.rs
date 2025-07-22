use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use flatbuffers::{FlatBufferBuilder, WIPOffset, root};
use schemas::game as schema;

use crate::{
    components::ClientIdComponent,
    events::{IncomingChatMessage, OutgoingMessage, OutgoingMessageData},
};

// TODO: Maybe use change detection for transform changes
pub fn sync_players(mut server: ResMut<RenetServer>, mut ev_msg: EventReader<OutgoingMessage>) {
    let mut client_events: HashMap<ClientId, Vec<&OutgoingMessageData>> = HashMap::new();
    for event in ev_msg.read() {
        client_events
            .entry(event.client_id)
            .or_default()
            .push(&event.data);
    }

    if client_events.is_empty() {
        return;
    }

    let mut builder = FlatBufferBuilder::new();

    // TODO: Parallelism?
    for (client_id, events) in client_events {
        let mut player_events = Vec::<WIPOffset<schema::Event>>::with_capacity(events.len());
        let mut can_be_unreliable = true;

        for event in events {
            if can_be_unreliable && !matches!(event, OutgoingMessageData::Movement(_, _)) {
                can_be_unreliable = false;
            }
            player_events.push(event.encode(&mut builder));
        }

        if player_events.is_empty() {
            return;
        }

        let fb_events = builder.create_vector(player_events.as_slice());
        let batch = schema::BatchedEvents::create(
            &mut builder,
            &schema::BatchedEventsArgs {
                events: Some(fb_events),
            },
        );
        builder.finish_minimal(batch);
        let data = builder.finished_data().to_vec();

        if can_be_unreliable {
            server.send_message(client_id, DefaultChannel::Unreliable, data);
        } else {
            server.send_message(client_id, DefaultChannel::ReliableOrdered, data);
        }

        builder.reset();
    }
}

pub fn handle_server_messages(
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
