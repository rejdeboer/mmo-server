use std::sync::Arc;

use crate::{
    components::{
        AssetIdComponent, CharacterIdComponent, ClientIdComponent, InterestedClients,
        LevelComponent, MovementSpeedComponent, NameComponent, Vitals,
    },
    messages::{OutgoingMessage, OutgoingMessageData, VisibilityChangedMessage},
    telemetry::Metrics,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use protocol::{
    models::Actor,
    server::{ActorTransformUpdate, ServerEvent},
};
use protocol::{
    models::{ActorAttributes, Vitals as NetVitals},
    primitives::Transform as NetTransform,
};

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

type SpawnableComponents<'a> = (
    &'a NameComponent,
    &'a Transform,
    &'a Vitals,
    &'a LevelComponent,
    &'a MovementSpeedComponent,
    Option<&'a CharacterIdComponent>,
    Option<&'a AssetIdComponent>,
);

pub fn sync_visibility(
    mut server: ResMut<RenetServer>,
    q_spawnables: Query<SpawnableComponents>,
    mut reader: MessageReader<VisibilityChangedMessage>,
    mut encode_buffer: Local<bitcode::Buffer>,
    mut spawn_cache: Local<HashMap<Entity, Vec<u8>>>,
) {
    spawn_cache.clear();

    for msg in reader.read() {
        for &entity in &msg.removed {
            let data = encode_buffer.encode(&ServerEvent::ActorDespawn(entity.to_bits()));
            server.send_message(
                msg.client_id,
                DefaultChannel::ReliableOrdered,
                data.to_vec(),
            );
        }

        for &entity in &msg.added {
            if let Some(cached_spawn) = spawn_cache.get(&entity) {
                server.send_message(
                    msg.client_id,
                    DefaultChannel::ReliableOrdered,
                    cached_spawn.clone(),
                );
                continue;
            }

            if let Ok((name, transform, vitals, level, speed, char_id, asset_id)) =
                q_spawnables.get(entity)
            {
                let attributes = if let Some(cid) = char_id {
                    ActorAttributes::Player {
                        character_id: cid.0,
                        // TODO: Correctly handle guild
                        guild_name: None,
                    }
                } else if let Some(aid) = asset_id {
                    ActorAttributes::Npc { asset_id: aid.0 }
                } else {
                    tracing::warn!(name = %name.0, "failed to create entity attributes");
                    continue;
                };

                let actor = Actor {
                    id: entity.to_bits(),
                    attributes,
                    name: name.0.to_string(),
                    transform: NetTransform::from_glam(transform.translation, transform.rotation),
                    vitals: NetVitals::from(vitals),
                    movement_speed: speed.0.into(),
                    level: level.0 as u8,
                };

                let data = encode_buffer
                    .encode(&ServerEvent::ActorSpawn(Box::new(actor)))
                    .to_vec();
                server.send_message(msg.client_id, DefaultChannel::ReliableOrdered, data.clone());

                spawn_cache.insert(entity, data);
            }
        }
    }
}
