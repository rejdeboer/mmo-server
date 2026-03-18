use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;
use protocol::models::ItemDrop;
use std::sync::Arc;

use crate::assets::ContentId;

#[derive(Resource, Debug, Default)]
pub struct ServerTick(pub u32);

impl ServerTick {
    pub fn next(&mut self) -> u32 {
        let tick = self.0;
        self.0 = self.0.wrapping_add(1);
        tick
    }
}

#[derive(Debug, Component, Default)]
pub struct LastClientTick(pub u32);

#[derive(Debug, Component)]
pub struct ClientIdComponent(pub ClientId);

#[derive(Debug, Component)]
pub struct CharacterIdComponent(pub i32);

#[derive(Debug, Component)]
pub struct AssetIdComponent(pub u32);

#[derive(Debug, Component)]
pub struct GridCell(pub IVec2);

#[derive(Debug, Component, Default)]
pub struct InterestedClients {
    pub clients: HashSet<ClientId>,
}

#[derive(Debug, Component, Clone)]
pub struct NameComponent(pub Arc<str>);

#[derive(Debug, Component, Default)]
pub struct VisibleEntities {
    pub entities: HashSet<Entity>,
}

#[derive(Component)]
pub struct MobSpawner {
    pub mob_id: ContentId,
    pub max_mobs: usize,
    pub timer: Timer,
    pub spawn_radius: f32,
    pub level_range: std::ops::Range<i32>,
}

#[derive(Component)]
pub struct Spawned {
    pub spawner: Entity,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct MonsterId(pub ContentId);

#[derive(Component)]
// TODO: Implement different types of castable actions
pub struct Casting {
    pub spell_id: u32,
    pub target: Entity,
    pub timer: Timer,
    pub castable_while_moving: bool,
}

#[derive(Component)]
pub struct Dead {
    pub despawn_timer: Timer,
}

#[derive(Component)]
// TODO: Taps can also be owned by groups
pub struct Tapped {
    pub owner_id: ClientId,
}

#[derive(Clone, Debug)]
pub struct LootEntry {
    pub item_id: u32,
    pub quantity: u16,
}

impl From<LootEntry> for ItemDrop {
    fn from(value: LootEntry) -> Self {
        Self {
            item_id: value.item_id,
            quantity: value.quantity,
        }
    }
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Loot {
    pub entries: Vec<LootEntry>,
    pub owner_id: ClientId,
}
