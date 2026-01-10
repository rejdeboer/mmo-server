use crate::{
    models::ChatChannel,
    primitives::{MOVEMENT_QUANTIZATION_FACTOR, YAW_QUANTIZATION_FACTOR},
};
use bitcode::{Decode, Encode};
use std::f32::consts::TAU;

#[derive(Encode, Decode)]
pub struct MoveAction {
    pub yaw: u16,
    pub forward: i8,
    pub sideways: i8,
}

impl MoveAction {
    pub fn from_f32(yaw: f32, forward: f32, sideways: f32) -> Self {
        let quantized_forward =
            (forward.clamp(-1.0, 1.0) * MOVEMENT_QUANTIZATION_FACTOR).round() as i8;
        let quantized_sideways =
            (sideways.clamp(-1.0, 1.0) * MOVEMENT_QUANTIZATION_FACTOR).round() as i8;

        let normalized_yaw = yaw.rem_euclid(TAU) / TAU;
        let quantized_yaw = (normalized_yaw * YAW_QUANTIZATION_FACTOR).round() as u16;

        Self {
            yaw: quantized_yaw,
            forward: quantized_forward,
            sideways: quantized_sideways,
        }
    }
}

#[derive(Encode, Decode)]
pub enum PlayerAction {
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
