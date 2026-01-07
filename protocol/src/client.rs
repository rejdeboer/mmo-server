use crate::models::ChatChannel;
use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
pub enum PlayerAction {
    Movement {
        yaw: u16,
        forward: i8,
        sideways: i8,
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
