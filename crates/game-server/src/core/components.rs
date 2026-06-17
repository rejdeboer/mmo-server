use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;
use std::sync::Arc;

use crate::telemetry::SERVER_TICK_METRIC;

#[derive(Resource, Debug, Default)]
pub struct ServerTick(pub u32);

impl ServerTick {
    pub fn advance(&mut self) -> u32 {
        let tick = self.0;
        metrics::counter!(SERVER_TICK_METRIC).increment(1);
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
pub struct Dead {
    pub despawn_timer: Timer,
}

#[derive(Component)]
pub struct Tapped {
    pub owner_id: ClientId,
}
