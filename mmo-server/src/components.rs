use std::sync::Arc;

use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;

use crate::assets::ContentId;

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

#[derive(Debug, Component, Clone)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Component)]
pub struct LevelComponent(pub i32);

#[derive(Debug, Component, Clone)]
pub struct MovementSpeedComponent(pub f32);

#[derive(Component)]
pub struct GroundedComponent;

#[derive(Component)]
pub struct MobSpawner {
    pub mob_id: ContentId,
    pub max_mobs: usize,
    pub timer: Timer,
    pub spawn_radius: f32,
    pub level_range: std::ops::Range<i32>,
}

#[derive(Component)]
pub struct SpawnedMob {
    pub spawner: Entity,
}

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
