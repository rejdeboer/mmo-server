use bevy::prelude::Component;
use protocol::models::Vitals as NetVitals;

#[derive(Component, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkId(pub u64);

#[derive(Debug, Component, Clone)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
}

impl From<Vitals> for NetVitals {
    fn from(value: Vitals) -> Self {
        Self {
            hp: value.hp,
            max_hp: value.max_hp,
        }
    }
}

impl From<NetVitals> for Vitals {
    fn from(value: NetVitals) -> Self {
        Self {
            hp: value.hp,
            max_hp: value.max_hp,
        }
    }
}

#[derive(Debug, Component)]
pub struct LevelComponent(pub i32);

#[derive(Debug, Component, Clone)]
pub struct MovementSpeedComponent(pub f32);

#[derive(Component)]
pub struct GroundedComponent;
