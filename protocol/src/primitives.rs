use bitcode::{Decode, Encode};

pub const YAW_QUANTIZATION_FACTOR: f32 = 65535.0;
pub const MOVEMENT_QUANTIZATION_FACTOR: f32 = 127.0;

#[derive(Encode, Decode, Clone)]
pub struct Transform {
    position: glam::Vec3,
    yaw: u16,
}

impl Transform {
    pub fn from_glam(pos: glam::Vec3, rot: glam::Quat) -> Self {
        let (yaw_rad, _, _) = rot.to_euler(glam::EulerRot::YXZ);
        let yaw_norm =
            (yaw_rad % std::f32::consts::TAU + std::f32::consts::TAU) % std::f32::consts::TAU;
        let yaw_u16 = (yaw_norm / std::f32::consts::TAU * YAW_QUANTIZATION_FACTOR) as u16;

        Self {
            position: pos,
            yaw: yaw_u16,
        }
    }

    pub fn get_quat(&self) -> glam::Quat {
        let yaw_rad = (self.yaw as f32 / YAW_QUANTIZATION_FACTOR) * std::f32::consts::TAU;
        glam::Quat::from_rotation_y(yaw_rad)
    }
}
