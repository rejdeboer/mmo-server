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

#[derive(Encode, Decode)]
pub enum ServerEvent {
    ActorMove {
        entity_id: u64,
        position: Vec3,
        yaw: u16,
    },
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

#[repr(u8)]
#[derive(Encode, Decode)]
pub enum ChatChannel {
    Say,
    Yell,
    Zone,
}

#[derive(Encode, Decode)]
pub enum PlayerAction {
    Movement {
        yaw: u16,
        forward: u8,
        sideways: u8,
    },
    Jump,
    CastSpell {
        spell_id: u32,
        target_entity_id: u64,
    },
    Chat {
        channel: ChatChannel,
        text: String,
    },
}

#[derive(Encode, Decode)]
pub struct EnterGameResponse {
    player_actor: Actor,
}
