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
pub struct Transform {
    #[bitcode(with_serde)]
    position: glam::Vec3,
    yaw: u16,
}

#[derive(Encode, Decode)]
pub struct Actor {
    /// The entity ID assigned by bevy
    id: u64,
    attributes: ActorAttributes,
    name: String,
    transform: Transform,
}
