use crate::Transform;
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
