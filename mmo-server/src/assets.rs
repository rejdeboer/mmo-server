use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::components::MonsterId;

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
    #[serde(default)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct LootTableEntry {
    pub item_id: u32,
    pub chance: f32,
    pub min: u16,
    pub max: u16,
}

pub type LootTable = Vec<LootTableEntry>;

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct LootTableLibrary {
    pub tables: HashMap<ContentId, LootTable>,
}

#[derive(Resource)]
pub struct LootTableLibraryHandle(pub Handle<LootTableLibrary>);

#[derive(SystemParam)]
pub struct LootDb<'w> {
    monster_handle: Res<'w, MonsterLibraryHandle>,
    monsters: Res<'w, Assets<MonsterLibrary>>,
    loot_handle: Res<'w, LootTableLibraryHandle>,
    loot_tables: Res<'w, Assets<LootTableLibrary>>,
}

impl<'w> LootDb<'w> {
    pub fn get_monster_loot_tables<'a>(
        &'a self,
        monster_id: &MonsterId,
    ) -> Option<impl Iterator<Item = &'a LootTable>> {
        let monster_lib = self.monsters.get(&self.monster_handle.0)?;
        let monster_def = monster_lib.types.get(&monster_id.0)?;
        let loot_lib = self.loot_tables.get(&self.loot_handle.0)?;

        Some(monster_def.loot_tables.iter().filter_map(|table_id| {
            let table = loot_lib.tables.get(table_id);

            if table.is_none() {
                tracing::warn!(
                    monster_id = ?monster_id.0,
                    ?table_id,
                    "monster references missing loot table ID",
                );
            }

            table
        }))
    }
}
