use bevy::prelude::*;
use bevy_renet::renet::ClientId;

use crate::economy::LootEntry;

#[derive(Message, Debug)]
pub struct VisibilityChangedMessage {
    pub client_id: ClientId,
    pub added: Vec<Entity>,
    pub removed: Vec<Entity>,
}

#[derive(Message, Debug, Clone)]
pub struct OutgoingMessage {
    pub recipients: Vec<ClientId>,
    pub data: OutgoingMessageData,
}

impl OutgoingMessage {
    pub fn new(client_ids: Vec<ClientId>, data: OutgoingMessageData) -> Self {
        Self {
            recipients: client_ids,
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutgoingMessageData {
    ChatMessage {
        channel: protocol::models::ChatChannel,
        sender_name: String,
        text: String,
    },
    Death {
        entity: Entity,
    },
    DespawnCorpse(Entity),
    KillReward {
        victim: Entity,
        loot: Vec<LootEntry>,
    },
    StartCasting {
        entity: Entity,
        spell_id: u32,
    },
    SpellImpact {
        target_entity: Entity,
        spell_id: u32,
        impact_amount: i32,
    },
}

impl From<OutgoingMessageData> for protocol::server::ServerEvent {
    fn from(value: OutgoingMessageData) -> Self {
        match value {
            OutgoingMessageData::Death { entity } => {
                protocol::server::ServerEvent::ActorDeath(entity.to_bits())
            }
            OutgoingMessageData::DespawnCorpse(entity) => {
                protocol::server::ServerEvent::ActorDespawn(entity.to_bits())
            }
            OutgoingMessageData::KillReward { victim, loot } => {
                protocol::server::ServerEvent::KillReward {
                    victim_id: victim.to_bits(),
                    loot: loot
                        .into_iter()
                        .map(protocol::models::ItemDrop::from)
                        .collect(),
                }
            }
            OutgoingMessageData::ChatMessage {
                channel,
                sender_name,
                text,
            } => protocol::server::ServerEvent::Chat {
                channel,
                sender_name,
                text,
            },
            OutgoingMessageData::StartCasting { entity, spell_id } => {
                protocol::server::ServerEvent::StartCasting {
                    actor_id: entity.to_bits(),
                    spell_id,
                }
            }
            OutgoingMessageData::SpellImpact {
                target_entity,
                spell_id,
                impact_amount,
            } => protocol::server::ServerEvent::SpellImpact {
                target_id: target_entity.to_bits(),
                spell_id,
                impact_amount,
            },
        }
    }
}
