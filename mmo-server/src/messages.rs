use crate::{
    components::{LootEntry, NameComponent, Vitals},
    systems::{EntityAttributes, serialize_entity},
};
use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use protocol::{models::ChatChannel, server::ServerEvent};
use schema::ChannelType;
use schemas::game as schema;
use std::sync::Arc;

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

#[derive(Message, Debug)]
pub struct OutgoingMessage {
    pub client_id: ClientId,
    pub data: OutgoingMessageData,
}

impl OutgoingMessage {
    pub fn new(client_id: ClientId, data: OutgoingMessageData) -> Self {
        Self { client_id, data }
    }
}

#[derive(Debug, Clone)]
// TODO: Box large enum variants?
pub enum OutgoingMessageData {
    ChatMessage {
        channel: ChatChannel,
        sender_name: NameComponent,
        text: String,
    },
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

impl OutgoingMessageData {
    pub fn broadcast(
        &self,
        recipients: &HashSet<ClientId>,
        writer: &mut MessageWriter<OutgoingMessage>,
    ) {
        writer.write_batch(recipients.iter().map(|client_id| OutgoingMessage {
            client_id: *client_id,
            data: self.clone(),
        }));
    }
}

impl From<OutgoingMessageData> for ServerEvent {
    fn from(value: OutgoingMessageData) -> Self {
        match value {
            OutgoingMessageData::Spawn {
                entity,
                attributes,
                name,
                transform,
                level,
                vitals,
                movement_speed,
            } => Self::ActorSpawn(()),
        }
    }
}
