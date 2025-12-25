use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct MonsterBlueprint {
    pub name: String,
    pub hp: i32,
    pub speed: f32,
    pub asset_id: u32,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct MonsterLibrary {
    pub types: HashMap<String, MonsterBlueprint>,
}

#[derive(Resource)]
pub struct MonsterLibraryHandle(pub Handle<MonsterLibrary>);
