use bevy::prelude::*;
use game_core::networking::NetworkId;

/// Internal message for combat hit feedback, emitted by network event handling.
#[derive(Message)]
pub struct CombatHitMessage {
    pub target_entity: Entity,
    pub amount: i32,
}

#[derive(Message)]
pub struct ActorSpawnMessage(pub protocol::models::Actor);

#[derive(Message)]
pub struct ActorDespawnMessage(pub NetworkId);

#[derive(Message)]
pub struct SpellImpactMessage {
    pub target_id: u32,
    pub spell_id: u32,
    pub impact_amount: i32,
}

#[derive(Message)]
pub struct ActorDeathMessage(pub u32);

#[derive(Message)]
pub struct StartCastingMessage {
    pub actor_id: u32,
    pub spell_id: u32,
}

#[derive(Message)]
pub struct KillRewardMessage {
    pub victim_id: u32,
    pub loot: Vec<protocol::models::ItemDrop>,
}

#[derive(Message)]
pub struct ServerChatMessage {
    pub channel: protocol::models::ChatChannel,
    pub sender_name: String,
    pub text: String,
}
