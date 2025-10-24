use crate::{
    components::{NameComponent, Vitals},
    systems::{EntityAttributes, serialize_entity},
};
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schema::ChannelType;
use schemas::game as schema;
use std::sync::Arc;

#[derive(Event, Debug)]
pub struct IncomingChatMessage {
    pub author: Entity,
    pub channel: ChannelType,
    pub text: String,
}

#[derive(Event, Debug)]
pub struct OutgoingMessage {
    pub client_id: ClientId,
    pub data: OutgoingMessageData,
}

impl OutgoingMessage {
    pub fn new(client_id: ClientId, data: OutgoingMessageData) -> Self {
        Self { client_id, data }
    }
}

#[derive(Debug)]
pub enum OutgoingMessageData {
    ChatMessage(ChannelType, NameComponent, String),
    Despawn(Entity),
    Movement(Entity, Transform),
    Spawn {
        entity: Entity,
        attributes: EntityAttributes,
        name: Arc<str>,
        transform: Transform,
        level: i32,
        vitals: Vitals,
        movement_speed: f32,
    },
}

impl OutgoingMessageData {
    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Event<'a>> {
        let data_type;
        let data = match self {
            Self::ChatMessage(channel, author, msg) => {
                data_type = schema::EventData::game_ServerChatMessage;
                let fb_author = builder.create_string(&author.0);
                let fb_msg = builder.create_string(msg);
                schema::ServerChatMessage::create(
                    builder,
                    &schema::ServerChatMessageArgs {
                        channel: *channel,
                        sender_name: Some(fb_author),
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::Movement(id, transform) => {
                data_type = schema::EventData::EntityMoveEvent;
                let pos = transform.translation;
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                let fb_transform =
                    schema::Transform::new(&schema::Vec3::new(pos.x, pos.y, pos.z), yaw);
                schema::EntityMoveEvent::create(
                    builder,
                    &schema::EntityMoveEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                )
                .as_union_value()
            }
            Self::Spawn {
                entity,
                attributes,
                name,
                transform,
                level,
                vitals,
                movement_speed,
            } => {
                data_type = schema::EventData::EntitySpawnEvent;
                let fb_entity = serialize_entity(
                    builder,
                    *entity,
                    attributes,
                    name,
                    transform,
                    vitals,
                    *level,
                    *movement_speed,
                );
                schema::EntitySpawnEvent::create(
                    builder,
                    &schema::EntitySpawnEventArgs {
                        entity: Some(fb_entity),
                    },
                )
                .as_union_value()
            }
            Self::Despawn(id) => {
                data_type = schema::EventData::EntityDespawnEvent;
                schema::EntityDespawnEvent::create(
                    builder,
                    &schema::EntityDespawnEventArgs {
                        entity_id: id.to_bits(),
                    },
                )
                .as_union_value()
            }
        };

        schema::Event::create(
            builder,
            &schema::EventArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}
