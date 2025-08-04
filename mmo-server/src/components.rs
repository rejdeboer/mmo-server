use std::sync::Arc;

use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;

#[derive(Debug, Component)]
pub struct ClientIdComponent(pub ClientId);

#[derive(Debug, Component)]
pub struct CharacterIdComponent(pub i32);

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
