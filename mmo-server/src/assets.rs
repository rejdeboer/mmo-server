use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
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

#[derive(Default)]
pub struct MonsterLoader;

impl AssetLoader for MonsterLoader {
    type Asset = MonsterLibrary;
    type Settings = ();
    type Error = anyhow::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let custom_asset = ron::de::from_bytes::<MonsterLibrary>(&bytes)?;
        Ok(custom_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}
