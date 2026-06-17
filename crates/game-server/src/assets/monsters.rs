use super::ContentId;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Component, Clone, Copy, Debug)]
pub struct MonsterId(pub ContentId);

#[derive(Deserialize, Debug, Clone, Copy, Default)]
pub enum AiBehaviorDef {
    #[default]
    Aggressive,
    Neutral,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WanderDef {
    pub radius: f32,
    pub pause_duration: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AiDef {
    #[serde(default)]
    pub behavior: AiBehaviorDef,
    pub aggro_radius: f32,
    pub leash_range: f32,
    #[serde(default)]
    pub ability_priorities: HashMap<u32, u8>,
    pub wander: Option<WanderDef>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MonsterDef {
    pub name: String,
    pub hp: i32,
    pub speed: f32,
    pub asset_id: u32,
    #[serde(default)]
    pub loot_tables: Vec<ContentId>,
    #[serde(default)]
    pub abilities: Vec<u32>,
    pub ai: Option<AiDef>,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct MonsterLibrary {
    pub types: HashMap<ContentId, MonsterDef>,
}

#[derive(Resource)]
pub struct MonsterLibraryHandle(pub Handle<MonsterLibrary>);
