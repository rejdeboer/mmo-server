use crate::primitives::Transform;
use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
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
    item_id: u32,
    quantity: u16,
}

#[derive(Encode, Decode)]
pub struct Actor {
    /// The entity ID assigned by bevy
    id: u64,
    attributes: ActorAttributes,
    name: String,
    transform: Transform,
    vitals: Vitals,
    level: i32,
    movement_speed: u16,
}

#[repr(u8)]
#[derive(Encode, Decode)]
pub enum ChatChannel {
    Say,
    Yell,
    Zone,
}
