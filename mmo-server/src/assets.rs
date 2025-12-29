use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentId(pub u64);

impl ContentId {
    pub fn from(s: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContentKeyVisitor;

        impl<'de> serde::de::Visitor<'de> for ContentKeyVisitor {
            type Value = ContentId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a content key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ContentId::from(value))
            }
        }

        deserializer.deserialize_str(ContentKeyVisitor)
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentId({:#x})", self.0)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MonsterDef {
    pub name: String,
    pub hp: i32,
    pub speed: f32,
    pub asset_id: u32,
    pub loot_tables: Vec<ContentId>,
}

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct MonsterLibrary {
    pub types: HashMap<ContentId, MonsterDef>,
}

#[derive(Resource)]
pub struct MonsterLibraryHandle(pub Handle<MonsterLibrary>);

#[derive(Deserialize, Debug, Clone)]
pub struct SpellDef {
    pub name: String,
    pub damage: i32,
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
