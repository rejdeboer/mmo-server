use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct ItemDef {
    pub name: String,
    pub stack_size: u16,
    pub asset_id: u32,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct ItemLibrary {
    pub items: HashMap<u32, ItemDef>,
}

#[derive(Resource)]
pub struct ItemLibraryHandle(pub Handle<ItemLibrary>);
