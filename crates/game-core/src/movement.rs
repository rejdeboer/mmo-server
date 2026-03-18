use bevy::math::Vec3;
use protocol::{
    client::MoveAction,
    primitives::{MOVEMENT_QUANTIZATION_FACTOR, YAW_QUANTIZATION_FACTOR},
};
use std::f32::consts::TAU;

pub struct MoveInput {
    pub yaw: f32,
    pub forward: f32,
    pub sideways: f32,
}

impl MoveInput {
    pub fn direction(&self) -> Vec3 {
        let forward_dir = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right_dir = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        forward_dir * self.forward + right_dir * self.sideways
    }

    pub fn target_velocity(&self, movement_speed: f32) -> Vec3 {
        self.direction().normalize_or_zero() * movement_speed
    }
}

impl From<MoveAction> for MoveInput {
    fn from(value: MoveAction) -> Self {
        Self {
            yaw: (value.yaw as f32 / YAW_QUANTIZATION_FACTOR) * TAU,
            forward: value.forward as f32 / MOVEMENT_QUANTIZATION_FACTOR,
            sideways: value.sideways as f32 / MOVEMENT_QUANTIZATION_FACTOR,
        }
    }
}
