use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct MonsterDef {
    pub name: String,
    pub hp: i32,
    pub speed: f32,
    pub asset_id: u32,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct MonsterLibrary {
    pub types: HashMap<String, MonsterDef>,
}

#[derive(Resource)]
pub struct MonsterLibraryHandle(pub Handle<MonsterLibrary>);

#[derive(Deserialize, Debug, Clone)]
pub struct SpellDef {
    pub name: String,
    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
    pub casting_duration: f32,
    #[serde(default)]
    pub castable_while_moving: bool,
    pub visual_id: u32,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct SpellLibrary {
    pub spells: HashMap<u32, SpellDef>,
}

#[derive(Resource)]
pub struct SpellLibraryHandle(pub Handle<SpellLibrary>);
