use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schemas::mmo::ChannelType;

use crate::components::NameComponent;

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
    Spawn(Entity, Transform),
}

impl OutgoingMessageData {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Event<'a>> {
        let data_type;
        let data = match self {
            Self::ChatMessage(channel, author, msg) => {
                data_type = schemas::mmo::EventData::mmo_ServerChatMessage;
                let fb_author = builder.create_string(&author.0);
                let fb_msg = builder.create_string(msg);
                schemas::mmo::ServerChatMessage::create(
                    builder,
                    &schemas::mmo::ServerChatMessageArgs {
                        channel: *channel,
                        author_name: Some(fb_author),
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::Movement(id, transform) => {
                data_type = schemas::mmo::EventData::EntityMoveEvent;
                let pos = transform.translation;
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                let fb_transform = schemas::mmo::Transform::new(
                    &schemas::mmo::Vec3::new(pos.x, pos.y, pos.z),
                    yaw,
                );
                schemas::mmo::EntityMoveEvent::create(
                    builder,
                    &schemas::mmo::EntityMoveEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                )
                .as_union_value()
            }
            Self::Spawn(id, transform) => {
                data_type = schemas::mmo::EventData::EntitySpawnEvent;
                let pos = transform.translation;
                let fb_transform = schemas::mmo::Transform::new(
                    &schemas::mmo::Vec3::new(pos.x, pos.y, pos.z),
                    transform.rotation.y,
                );
                schemas::mmo::EntitySpawnEvent::create(
                    builder,
                    &schemas::mmo::EntitySpawnEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                )
                .as_union_value()
            }
            Self::Despawn(id) => {
                data_type = schemas::mmo::EventData::EntityDespawnEvent;
                schemas::mmo::EntityDespawnEvent::create(
                    builder,
                    &schemas::mmo::EntityDespawnEventArgs {
                        entity_id: id.to_bits(),
                    },
                )
                .as_union_value()
            }
        };

        schemas::mmo::Event::create(
            builder,
            &schemas::mmo::EventArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}
