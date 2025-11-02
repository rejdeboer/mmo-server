use crate::messages::{OutgoingMessage, OutgoingMessageData};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schemas::game as schema;

// TODO: Maybe use change detection for transform changes
pub fn sync_players(mut server: ResMut<RenetServer>, mut ev_msg: MessageReader<OutgoingMessage>) {
    let mut client_messages: HashMap<ClientId, Vec<&OutgoingMessageData>> = HashMap::new();
    for event in ev_msg.read() {
        client_messages
            .entry(event.client_id)
            .or_default()
            .push(&event.data);
    }

    if client_messages.is_empty() {
        return;
    }

    let mut builder = FlatBufferBuilder::new();

    // TODO: Parallelism?
    for (client_id, messages) in client_messages {
        let mut player_messages = Vec::<WIPOffset<schema::Event>>::with_capacity(messages.len());
        let mut channel = DefaultChannel::Unreliable;

        for msg in messages {
            if !matches!(msg, OutgoingMessageData::Movement(_, _)) {
                channel = DefaultChannel::ReliableOrdered;
            }
            player_messages.push(msg.encode(&mut builder));
        }

        if player_messages.is_empty() {
            return;
        }

        let fb_events = builder.create_vector(player_messages.as_slice());
        let batch = schema::BatchedEvents::create(
            &mut builder,
            &schema::BatchedEventsArgs {
                events: Some(fb_events),
            },
        );

        builder.finish_minimal(batch);
        let data = builder.finished_data().to_vec();
        server.send_message(client_id, channel, data);
        builder.reset();
    }
}
