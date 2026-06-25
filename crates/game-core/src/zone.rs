use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct PropInstance {
    pub asset: String,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl PropInstance {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::from(self.translation),
            rotation: Quat::from_xyzw(
                self.rotation[0],
                self.rotation[1],
                self.rotation[2],
                self.rotation[3],
            ),
            scale: Vec3::from(self.scale),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpawnPoint {
    pub id: String,
    pub position: [f32; 3],
    pub radius: f32,
    pub monster_id: String,
    pub max_count: usize,
    pub level_range: (i32, i32),
    pub respawn_secs: f32,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct ZoneDef {
    pub id: String,
    pub terrain: String,
    #[serde(default)]
    pub skydome: Option<String>,
    #[serde(default)]
    pub skydome_scale: Option<f32>,
    pub player_spawn: [f32; 3],
    pub props: Vec<PropInstance>,
    #[serde(default)]
    pub spawn_points: Vec<SpawnPoint>,
}

#[derive(Resource)]
pub struct ZoneDefHandle(pub Handle<ZoneDef>);
