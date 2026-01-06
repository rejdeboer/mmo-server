use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Encode, Decode)]
pub struct Transform {
    position: Vec3,
    yaw: u16,
}
