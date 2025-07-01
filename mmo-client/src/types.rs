// TODO: Do we need these intermediate structs?
#[derive(Debug, Clone)]
pub struct Character {
    pub entity_id: u64,
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

impl Into<Character> for schemas::mmo::EnterGameResponse<'_> {
    fn into(self) -> Character {
        let entity = self.character().unwrap().entity().unwrap();
        Character {
            entity_id: self.player_entity_id(),
            name: entity.name().to_string(),
            hp: entity.hp(),
            level: entity.level(),
            transform: entity.transform().into(),
        }
    }
}

impl Into<Transform> for &schemas::mmo::Transform {
    fn into(self) -> Transform {
        Transform {
            position: self.position().into(),
            yaw: self.yaw(),
        }
    }
}

impl Into<Vec3> for &schemas::mmo::Vec3 {
    fn into(self) -> Vec3 {
        Vec3 {
            x: self.x(),
            y: self.y(),
            z: self.z(),
        }
    }
}
