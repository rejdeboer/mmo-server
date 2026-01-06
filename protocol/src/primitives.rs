use bitcode::{Decode, Encode};

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
