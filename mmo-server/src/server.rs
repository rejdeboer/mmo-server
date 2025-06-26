use bevy::prelude::*;
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use flatbuffers::{FlatBufferBuilder, WIPOffset, root};

use crate::components::{ClientIdComponent, EntityId};

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
}

impl OutgoingMessageData {
    pub fn encode<'a>(
        self,
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
        }
    }
}

// pub fn flush_messages(mut server: ResMut<RenetServer>, mut ev_msg: EventReader<OutgoingMessage>) {
//     ev_msg.re
// }

pub fn handle_server_messages(
    mut server: ResMut<RenetServer>,
    clients: Query<(Entity, &ClientIdComponent)>,
    mut commands: Commands,
) {
    for (entity, client_id) in clients.iter() {
        if let Some(message) = server.receive_message(client_id.0, DefaultChannel::Unreliable) {
            process_message(entity, message, &mut commands);
        }
    }
}

fn process_message(entity: Entity, message: bevy_renet::renet::Bytes, commands: &mut Commands) {
    match root::<schemas::mmo::BatchedEvents>(&message) {
        Ok(batch) => {
            for event in batch.events().unwrap() {
                match event.data_type() {
                    schemas::mmo::EventData::EntityMoveEvent => {
                        process_player_move_event(
                            entity,
                            event.data_as_entity_move_event().unwrap(),
                            commands,
                        );
                    }
                    _ => {
                        bevy::log::warn!("unhandled event data type");
                    }
                }
            }
        }
        Err(error) => {
            bevy::log::error!(?error, "message does not follow event schema");
        }
    }
}

fn process_player_move_event(
    entity: Entity,
    event: schemas::mmo::EntityMoveEvent,
    commands: &mut Commands,
) {
    let pos = event.position().unwrap();
    // TODO: Rotations
    let transform = Transform::from_xyz(pos.x(), pos.y(), pos.z());
    commands.entity(entity).insert(transform);
    // TODO: This way of writing events is not performant
    commands.send_event(EntityMoveEvent { entity, transform });
}

// TODO: Consider batching all events per client, and flushing them every server tick
pub fn handle_entity_move_events(
    mut ev_moves: EventReader<EntityMoveEvent>,
    mut server: ResMut<RenetServer>,
    q_entity_id: Query<&EntityId>,
) {
    let mut events = Vec::<WIPOffset<schemas::mmo::Event>>::new();
    let mut builder = FlatBufferBuilder::new();

    for event in ev_moves.read() {
        let entity_id = q_entity_id.get(event.entity).unwrap().0;
        let pos = event.transform.translation;
        let event_data = schemas::mmo::EntityMoveEvent::create(
            &mut builder,
            &schemas::mmo::EntityMoveEventArgs {
                entity_id,
                position: Some(&schemas::mmo::Vec3::new(pos.x, pos.y, pos.z)),
                direction: Some(&schemas::mmo::Vec2::new(0., 0.)),
            },
        );
        let fb_event = schemas::mmo::Event::create(
            &mut builder,
            &schemas::mmo::EventArgs {
                data_type: schemas::mmo::EventData::EntityMoveEvent,
                data: Some(event_data.as_union_value()),
            },
        );
        events.push(fb_event);
    }

    let fb_events = builder.create_vector(events.as_slice());
    let batch = schemas::mmo::BatchedEvents::create(
        &mut builder,
        &schemas::mmo::BatchedEventsArgs {
            events: Some(fb_events),
        },
    );
    builder.finish_minimal(batch);
    let data = builder.finished_data().to_vec();

    server.broadcast_message(DefaultChannel::Unreliable, data);
}
