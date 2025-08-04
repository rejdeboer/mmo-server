use crate::events::{OutgoingMessage, OutgoingMessageData};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schemas::game as schema;

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
