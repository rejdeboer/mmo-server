use crate::primitives::{MovementSpeed, Transform};
use bitcode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub enum ActorAttributes {
    Player {
        character_id: i32,
        guild_name: Option<String>,
    },
    Npc {
        asset_id: u32,
    },
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Encode, Decode)]
pub struct ItemDrop {
    pub item_id: u32,
    pub quantity: u16,
}

#[derive(Encode, Decode, Debug)]
pub struct Actor {
    /// The entity ID assigned by bevy
    pub id: u64,
    pub attributes: ActorAttributes,
    pub name: String,
    pub transform: Transform,
    pub vitals: Vitals,
    pub level: u8,
    pub movement_speed: MovementSpeed,
}

#[repr(u8)]
#[derive(Encode, Decode, Debug, Clone)]
pub enum ChatChannel {
    Say,
    Yell,
    Zone,
}
