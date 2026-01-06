use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
pub struct Transform {
    position: glam::Vec3,
    yaw: u16,
}
