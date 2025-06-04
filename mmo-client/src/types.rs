// TODO: Do we need these intermediate structs?
#[derive(Debug, Clone)]
pub struct Character {
    pub name: String,
    pub hp: i32,
    pub level: i32,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
}

impl Into<Transform> for &schemas::mmo::Transform {
    fn into(self) -> Transform {
        Transform {
            position: self.position().into(),
        }
    }
}

impl Into<Vec3> for &schemas::mmo::Vec3 {
    fn into(self) -> Vec3 {
        Vec3 {
            x: self.x() as f32,
            y: self.y() as f32,
            z: self.z() as f32,
        }
    }
}
