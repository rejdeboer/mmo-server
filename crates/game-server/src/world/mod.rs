pub mod messages;
mod movement;
mod spatial_grid;
mod spawner;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub use messages::*;

#[derive(Debug, Resource, Default)]
pub struct SpatialGrid {
    pub cells: HashMap<IVec2, Vec<Entity>>,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorldSet {
    /// Advance the server tick counter.
    Tick,
    /// Ground detection and physics preprocessing.
    PreProcess,
    /// Apply player movement and jump inputs.
    ProcessMovement,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MoveActionMessage>();
        app.add_message::<JumpActionMessage>();

        app.insert_resource(SpatialGrid::default());

        app.add_systems(
            FixedPreUpdate,
            movement::increment_server_tick.in_set(WorldSet::Tick),
        );
        app.add_systems(
            FixedPreUpdate,
            movement::check_ground_status.in_set(WorldSet::PreProcess),
        );
        app.add_systems(
            FixedPreUpdate,
            (
                movement::process_move_action_messages,
                movement::process_jump_action_messages,
            )
                .in_set(WorldSet::ProcessMovement),
        );

        app.add_systems(Startup, spawner::setup_spawners);
        app.add_systems(FixedUpdate, (spawner::spawn_mobs, spatial_grid::update_spatial_grid));
    }
}
