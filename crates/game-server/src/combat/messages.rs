use bevy::prelude::*;
use bevy_renet::renet::ClientId;

#[derive(Message, Debug)]
pub struct CastSpellActionMessage {
    pub caster_entity: Entity,
    pub target_entity: Entity,
    pub spell_id: u32,
}

#[derive(Message, Debug)]
pub struct ApplySpellEffectMessage {
    pub caster_entity: Entity,
    pub caster_client_id: Option<ClientId>,
    pub target_entity: Entity,
    pub spell_id: u32,
}

#[derive(Message, Debug)]
pub struct StartAttackMessage {
    pub attacker_entity: Entity,
    pub target_entity: Entity,
}

#[derive(Message, Debug)]
pub struct StopAttackMessage {
    pub attacker_entity: Entity,
}
