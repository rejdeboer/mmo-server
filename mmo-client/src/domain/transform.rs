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

impl Into<Transform> for &schema::Transform {
    fn into(self) -> Transform {
        Transform {
            position: self.position().into(),
            yaw: self.yaw(),
        }
    }
}

impl Into<Vec3> for &schema::Vec3 {
    fn into(self) -> Vec3 {
        Vec3 {
            x: self.x(),
            y: self.y(),
            z: self.z(),
        }
    }
}
