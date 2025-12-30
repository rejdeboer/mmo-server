use crate::{
    components::{LootEntry, NameComponent, Vitals},
    systems::{EntityAttributes, serialize_entity},
};
use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
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
    pub channel: ChannelType,
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
    ChatMessage(ChannelType, NameComponent, String),
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

    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Event<'a>> {
        let data_type;
        let data = match self {
            Self::ChatMessage(channel, author, msg) => {
                data_type = schema::EventData::game_ServerChatMessage;
                let fb_author = builder.create_string(&author.0);
                let fb_msg = builder.create_string(msg);
                schema::ServerChatMessage::create(
                    builder,
                    &schema::ServerChatMessageArgs {
                        channel: *channel,
                        sender_name: Some(fb_author),
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::Movement(id, transform) => {
                data_type = schema::EventData::EntityMoveEvent;
                let pos = transform.translation;
                let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                let fb_transform =
                    schema::Transform::new(&schema::Vec3::new(pos.x, pos.y, pos.z), yaw);
                schema::EntityMoveEvent::create(
                    builder,
                    &schema::EntityMoveEventArgs {
                        entity_id: id.to_bits(),
                        transform: Some(&fb_transform),
                    },
                )
                .as_union_value()
            }
            Self::StartCasting { entity, spell_id } => {
                data_type = schema::EventData::StartCastingEvent;
                schema::StartCastingEvent::create(
                    builder,
                    &schema::StartCastingEventArgs {
                        entity_id: entity.to_bits(),
                        spell_id: *spell_id,
                    },
                )
                .as_union_value()
            }
            Self::SpellImpact {
                target_entity,
                spell_id,
                impact_amount,
            } => {
                data_type = schema::EventData::SpellImpactEvent;
                schema::SpellImpactEvent::create(
                    builder,
                    &schema::SpellImpactEventArgs {
                        target_id: target_entity.to_bits(),
                        spell_id: *spell_id,
                        impact_amount: *impact_amount,
                    },
                )
                .as_union_value()
            }
            Self::Spawn {
                entity,
                attributes,
                name,
                transform,
                level,
                vitals,
                movement_speed,
            } => {
                data_type = schema::EventData::EntitySpawnEvent;
                let fb_entity = serialize_entity(
                    builder,
                    *entity,
                    attributes,
                    name,
                    transform,
                    vitals,
                    *level,
                    *movement_speed,
                );
                schema::EntitySpawnEvent::create(
                    builder,
                    &schema::EntitySpawnEventArgs {
                        entity: Some(fb_entity),
                    },
                )
                .as_union_value()
            }
            Self::Despawn(id) => {
                data_type = schema::EventData::EntityDespawnEvent;
                schema::EntityDespawnEvent::create(
                    builder,
                    &schema::EntityDespawnEventArgs {
                        entity_id: id.to_bits(),
                    },
                )
                .as_union_value()
            }
            Self::Death { entity } => {
                data_type = schema::EventData::EntityDeathEvent;
                schema::EntityDeathEvent::create(
                    builder,
                    &schema::EntityDeathEventArgs {
                        entity_id: entity.to_bits(),
                    },
                )
                .as_union_value()
            }
            Self::KillReward { victim, loot } => {
                data_type = schema::EventData::KillRewardEvent;
                builder.start_vector::<schema::ItemDrop>(loot.len());
                for entry in loot.iter.rev() {
                    builder.push(schema::ItemDrop::new(entry.item_id, entry.quantity));
                }
                let fb_loot = builder.end_vector(loot.len());
                schema::KillRewardEvent::create(
                    builder,
                    &schema::KillRewardEventArgs {
                        victim_id: victim.to_bits(),
                        loot: Some(fb_loot),
                    },
                )
                .as_union_value()
            }
        };

        schema::Event::create(
            builder,
            &schema::EventArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}
