use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schemas::mmo::ChannelType;

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
    ChatMessage(ChannelType, String, String),
    Despawn(Entity),
    Movement(Entity, Transform),
    Spawn(Entity, Transform),
}

impl OutgoingMessageData {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Event<'a>> {
        match self {
            Self::ChatMessage(channel, author, msg) => {
                let fb_author = builder.create_string(author);
                let fb_msg = builder.create_string(msg);
                let event_data = schemas::mmo::ServerChatMessage::create(
                    builder,
                    &schemas::mmo::ServerChatMessageArgs {
                        channel: *channel,
                        author_name: Some(fb_author),
                        text: Some(fb_msg),
                    },
                );
                schemas::mmo::Event::create(
                    builder,
                    &schemas::mmo::EventArgs {
                        data_type: schemas::mmo::EventData::mmo_ServerChatMessage,
                        data: Some(event_data.as_union_value()),
                    },
                )
            }
            Self::Movement(id, transform) => {
                let pos = transform.translation;
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                let fb_transform = schemas::mmo::Transform::new(
                    &schemas::mmo::Vec3::new(pos.x, pos.y, pos.z),
                    yaw,
                );
                let event_data = schemas::mmo::EntityMoveEvent::create(
                    builder,
                    &schemas::mmo::EntityMoveEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                );
                schemas::mmo::Event::create(
                    builder,
                    &schemas::mmo::EventArgs {
                        data_type: schemas::mmo::EventData::EntityMoveEvent,
                        data: Some(event_data.as_union_value()),
                    },
                )
            }
            Self::Spawn(id, transform) => {
                let pos = transform.translation;
                let fb_transform = schemas::mmo::Transform::new(
                    &schemas::mmo::Vec3::new(pos.x, pos.y, pos.z),
                    transform.rotation.y,
                );
                let event_data = schemas::mmo::EntitySpawnEvent::create(
                    builder,
                    &schemas::mmo::EntitySpawnEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                );
                schemas::mmo::Event::create(
                    builder,
                    &schemas::mmo::EventArgs {
                        data_type: schemas::mmo::EventData::EntitySpawnEvent,
                        data: Some(event_data.as_union_value()),
                    },
                )
            }
            Self::Despawn(id) => {
                let event_data = schemas::mmo::EntityDespawnEvent::create(
                    builder,
                    &schemas::mmo::EntityDespawnEventArgs {
                        entity_id: id.to_bits(),
                    },
                );
                schemas::mmo::Event::create(
                    builder,
                    &schemas::mmo::EventArgs {
                        data_type: schemas::mmo::EventData::EntityDespawnEvent,
                        data: Some(event_data.as_union_value()),
                    },
                )
            }
        }
    }
}
