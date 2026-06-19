use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::renet::ClientId;
use game_core::networking::NetworkId;
use std::sync::Arc;

use crate::telemetry::SERVER_TICK_METRIC;

/// Monotonically incrementing counter for assigning unique network IDs to entities.
#[derive(Resource, Debug, Default)]
pub struct NetworkIdCounter(u32);

impl NetworkIdCounter {
    /// NOTE: wraps after ~4 billion allocations. If the server ever runs long enough
    /// for this to happen, add collision-skipping logic
    pub fn allocate(&mut self) -> NetworkId {
        let id = self.0;
        self.0 = self.0.wrapping_add(1);
        NetworkId(id)
    }
}

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
