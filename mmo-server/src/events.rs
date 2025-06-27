use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::components::EntityId;

#[derive(Event)]
pub struct EntityMoveEvent {
    pub entity: Entity,
    pub transform: Transform,
}

#[derive(Event)]
pub struct OutgoingMessage {
    pub client_id: ClientId,
    pub data: OutgoingMessageData,
}

pub enum OutgoingMessageData {
    Movement(EntityId, Transform),
    Spawn(EntityId),
    Despawn(EntityId),
}

impl OutgoingMessageData {
    pub fn encode<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Event<'a>> {
        match self {
            Self::Movement(id, transform) => {
                let pos = transform.translation;
                let event_data = schemas::mmo::EntityMoveEvent::create(
                    builder,
                    &schemas::mmo::EntityMoveEventArgs {
                        entity_id: id.0,
                        position: Some(&schemas::mmo::Vec3::new(pos.x, pos.y, pos.z)),
                        direction: Some(&schemas::mmo::Vec2::new(0., 0.)),
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
            Self::Spawn(id) => {
                let event_data = schemas::mmo::EntitySpawnEvent::create(
                    builder,
                    &schemas::mmo::EntitySpawnEventArgs { entity_id: id.0 },
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
                    &schemas::mmo::EntityDespawnEventArgs { entity_id: id.0 },
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
