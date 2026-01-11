use crate::components::{LootEntry, NameComponent};
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use protocol::{models::ChatChannel, server::ServerEvent};

#[derive(Message, Debug)]
pub struct MoveActionMessage {
    pub entity: Entity,
    pub yaw: u16,
    pub forward: i8,
    pub sideways: i8,
}

#[derive(Message, Debug)]
pub struct JumpActionMessage {
    pub entity: Entity,
}

#[derive(Message, Debug)]
pub struct CastSpellActionMessage {
    pub caster_entity: Entity,
    pub target_entity: Entity,
    pub spell_id: u32,
}

#[derive(Message, Debug)]
pub struct IncomingChatMessage {
    pub author: Entity,
    pub channel: ChatChannel,
    pub text: String,
}

#[derive(Message, Debug)]
pub struct ApplySpellEffectMessage {
    pub caster_entity: Entity,
    pub caster_client_id: Option<ClientId>,
    pub target_entity: Entity,
    pub spell_id: u32,
}

#[derive(Message, Debug)]
pub struct VisibilityChangedMessage {
    pub client_id: ClientId,
    pub added: Vec<Entity>,
    pub removed: Vec<Entity>,
}

#[derive(Message, Debug, Clone)]
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
// TODO: Box large enum variants?
pub enum OutgoingMessageData {
    ChatMessage {
        channel: ChatChannel,
        sender_name: NameComponent,
        text: String,
    },
    Death {
        entity: Entity,
    },
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

impl From<OutgoingMessageData> for ServerEvent {
    fn from(value: OutgoingMessageData) -> Self {
        match value {
            OutgoingMessageData::Death { entity } => ServerEvent::ActorDeath(entity.to_bits()),
            OutgoingMessageData::KillReward { victim, loot } => ServerEvent::KillReward {
                victim_id: victim.to_bits(),
                loot: loot.into_iter().map(|e| e.into()).collect(),
            },
            OutgoingMessageData::ChatMessage {
                channel,
                sender_name,
                text,
            } => ServerEvent::Chat {
                channel,
                sender_name: sender_name.0.to_string(),
                text,
            },
            OutgoingMessageData::StartCasting { entity, spell_id } => ServerEvent::StartCasting {
                actor_id: entity.to_bits(),
                spell_id,
            },
            OutgoingMessageData::SpellImpact {
                target_entity,
                spell_id,
                impact_amount,
            } => ServerEvent::SpellImpact {
                target_id: target_entity.to_bits(),
                spell_id,
                impact_amount,
            },
        }
    }
}
