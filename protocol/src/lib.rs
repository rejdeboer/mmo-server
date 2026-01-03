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

#[derive(Encode, Decode)]
pub struct Vec3 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Encode, Decode)]
pub struct Transform {
    position: Vec3,
    yaw: u16,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
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

#[derive(Encode, Decode)]
pub enum ServerEvent {
    ActorMoveEvent {
        actor_id: u64,
        position: Vec3,
        yaw: u16,
    },
    ActorSpawnEvent(Box<Actor>),
    ActorDespawnEvent(u64),
}
