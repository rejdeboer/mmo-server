use super::{ContentId, MonsterId, MonsterLibrary, MonsterLibraryHandle};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::Deserialize;
use std::collections::HashMap;

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
