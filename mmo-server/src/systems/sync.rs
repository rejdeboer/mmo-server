use crate::{
    components::{ClientIdComponent, InterestedClients},
    messages::{OutgoingMessage, OutgoingMessageData},
    telemetry::Metrics,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use protocol::primitives::Transform as NetTransform;
use protocol::server::{ActorTransformUpdate, ServerEvent};

pub fn sync_movement(
    mut server: ResMut<RenetServer>,
    mut encode_buffer: Local<bitcode::Buffer>,
    mut updates_per_client: Local<HashMap<u64, Vec<ActorTransformUpdate>>>,
    q_movement: Query<
        (
            Entity,
            &Transform,
            &InterestedClients,
            Option<&ClientIdComponent>,
        ),
        Changed<Transform>,
    >,
) {
    for clients in updates_per_client.values_mut() {
        clients.clear();
    }

    for (entity, transform, interested, moved_client_id) in q_movement.iter() {
        if interested.clients.is_empty() {
            continue;
        }

        // TODO: Use custom network ID u32
        let update = ActorTransformUpdate {
            actor_id: entity.to_bits(),
            transform: NetTransform::from_glam(transform.translation, transform.rotation),
        };

        for &client_id in &interested.clients {
            updates_per_client
                .entry(client_id)
                .or_default()
                .push(update.clone());
        }

        if let Some(client_id) = moved_client_id {
            updates_per_client
                .entry(client_id.0)
                .or_default()
                .push(update);
        }
    }

    for (client_id, updates) in updates_per_client.iter() {
        if updates.is_empty() {
            continue;
        }

        let data = encode_buffer.encode(updates);
        server.send_message(*client_id, DefaultChannel::Unreliable, data.to_vec());
    }
}

pub fn sync_players(
    mut server: ResMut<RenetServer>,
    mut ev_msg: MessageReader<OutgoingMessage>,
    metrics: Res<Metrics>,
) {
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

    for (client_id, messages) in client_messages {
        let mut player_messages = Vec::<ServerEvent>::with_capacity(messages.len());

        let metric_labels = &["outgoing", "reliable"];
        metrics
            .network_packets_total
            .with_label_values(metric_labels)
            .inc();
        metrics
            .network_bytes_total
            .with_label_values(metric_labels)
            .inc_by(data.len() as u64);

        server.send_message(client_id, DefaultChannel::ReliableOrdered, data);
        builder.reset();
    }
}
