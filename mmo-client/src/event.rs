use crate::Transform;
use flatbuffers::{InvalidFlatbuffer, root};
use renet::Bytes;
use schema::ChannelType;
use schemas::game as schema;

#[derive(Debug, Clone)]
pub enum GameEvent {
    Chat {
        channel: ChannelType,
        sender_name: String,
        text: String,
    },
    MoveEntity {
        entity_id: u64,
        transform: Transform,
    },
    SpawnEntity {
        entity_id: u64,
        transform: Transform,
    },
    DespawnEntity {
        entity_id: u64,
    },
}

pub fn read_event_batch(
    events: &mut Vec<GameEvent>,
    bytes: Bytes,
) -> Result<(), InvalidFlatbuffer> {
    let batch = root::<schema::BatchedEvents>(&bytes)?;
    let Some(fb_events) = batch.events() else {
        return Ok(());
    };

    for event in fb_events {
        match event.data_type() {
            schema::EventData::game_ServerChatMessage => {
                let fb_event = event
                    .data_as_game_server_chat_message()
                    .expect("event should be some");
                events.push(GameEvent::Chat {
                    channel: fb_event.channel(),
                    sender_name: fb_event.sender_name().to_string(),
                    text: fb_event.text().to_string(),
                })
            }
            schema::EventData::EntityMoveEvent => {
                let fb_event = event
                    .data_as_entity_move_event()
                    .expect("event should be some");
                let transform = fb_event.transform().expect("transform should be some");
                let pos = transform.position();
                events.push(GameEvent::MoveEntity {
                    entity_id: fb_event.entity_id(),
                    transform: Transform {
                        position: Vec3::new(pos.x(), pos.y(), pos.z()),
                        yaw: transform.yaw(),
                    },
                })
            }
            schema::EventData::EntitySpawnEvent => {
                let fb_event = event
                    .data_as_entity_spawn_event()
                    .expect("event should be entity spawn event");
                let transform = fb_event.transform().expect("transform should be some");
                let pos = transform.position();
                events.push(GameEvent::SpawnEntity {
                    entity_id: fb_event.entity_id(),
                    transform: Transform {
                        position: Vec3::new(pos.x(), pos.y(), pos.z()),
                        yaw: transform.yaw(),
                    },
                })
            }
            schema::EventData::EntityDespawnEvent => events.push(GameEvent::DespawnEntity {
                entity_id: event.data_as_entity_despawn_event().unwrap().entity_id(),
            }),
            event_type => {
                tracing::warn!(?event_type, "unhandled event type");
            }
        }
    }

    Ok(())
}
