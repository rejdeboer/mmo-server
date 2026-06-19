use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use game_core::networking::NetworkId;

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
        network_id: NetworkId,
    },
    DespawnCorpse(NetworkId),
    KillReward {
        victim_network_id: NetworkId,
        loot: Vec<LootEntry>,
    },
    StartCasting {
        network_id: NetworkId,
        spell_id: u32,
    },
    SpellImpact {
        target_network_id: NetworkId,
        spell_id: u32,
        impact_amount: i32,
    },
}

impl From<OutgoingMessageData> for protocol::server::ServerEvent {
    fn from(value: OutgoingMessageData) -> Self {
        match value {
            OutgoingMessageData::Death { network_id } => {
                protocol::server::ServerEvent::ActorDeath(network_id.0)
            }
            OutgoingMessageData::DespawnCorpse(network_id) => {
                protocol::server::ServerEvent::ActorDespawn(network_id.0)
            }
            OutgoingMessageData::KillReward {
                victim_network_id,
                loot,
            } => protocol::server::ServerEvent::KillReward {
                victim_id: victim_network_id.0,
                loot: loot
                    .into_iter()
                    .map(protocol::models::ItemDrop::from)
                    .collect(),
            },
            OutgoingMessageData::ChatMessage {
                channel,
                sender_name,
                text,
            } => protocol::server::ServerEvent::Chat {
                channel,
                sender_name,
                text,
            },
            OutgoingMessageData::StartCasting {
                network_id,
                spell_id,
            } => protocol::server::ServerEvent::StartCasting {
                actor_id: network_id.0,
                spell_id,
            },
            OutgoingMessageData::SpellImpact {
                target_network_id,
                spell_id,
                impact_amount,
            } => protocol::server::ServerEvent::SpellImpact {
                target_id: target_network_id.0,
                spell_id,
                impact_amount,
            },
        }
    }
}
