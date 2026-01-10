use crate::{
    models::{Actor, ChatChannel, ItemDrop},
    primitives::Transform,
};
use bitcode::{Decode, Encode};

#[derive(Encode, Decode, Clone)]
pub struct ActorTransformUpdate {
    pub actor_id: u64,
    pub transform: Transform,
}

#[derive(Encode, Decode)]
pub enum ServerEvent {
    ActorSpawn(Box<Actor>),
    ActorDespawn(u64),
    ActorDeath(u64),
    KillReward {
        victim_id: u64,
        loot: Vec<ItemDrop>,
    },
    StartCasting {
        actor_id: u64,
        spell_id: u32,
    },
    SpellImpact {
        // TODO: How will we do spells that do not have a target, or do not deal damage?
        // We probably want to implement impact type, like damage, heal, etc...
        target_id: u64,
        spell_id: u32,
        impact_amount: i32,
    },
    Chat {
        channel: ChatChannel,
        sender_name: String,
        text: String,
    },
}

#[derive(Encode, Decode)]
pub struct EnterGameResponse {
    pub player_actor: Actor,
}

#[derive(Encode, Decode)]
pub struct TokenUserData {
    pub character_id: i32,
    pub traceparent: Option<String>,
}
