pub mod components;
mod decision;
mod leash;
mod movement;
mod state;
mod threat;
mod wander;

use bevy::prelude::*;

pub use components::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                (
                    threat::detect_players,
                    threat::update_threat_on_damage,
                    threat::cleanup_threat_tables,
                )
                    .chain(),
                state::ai_state_transitions,
                (wander::wander, decision::ai_select_ability),
                (movement::apply_ai_movement, leash::reset_evading_mobs),
            )
                .chain(),
        );
    }
}
