use schemas::game as schema;

#[derive(Debug, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub yaw: f32,
}

impl From<&schema::Transform> for Transform {
    fn from(val: &schema::Transform) -> Self {
        Transform {
            position: val.position().into(),
            yaw: val.yaw(),
        }
    }
}

impl From<&schema::Vec3> for Vec3 {
    fn from(val: &schema::Vec3) -> Self {
        Vec3 {
            x: val.x(),
            y: val.y(),
            z: val.z(),
        }
    }
}
