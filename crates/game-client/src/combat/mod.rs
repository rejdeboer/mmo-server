mod auto_attack;
pub mod action_bar;
pub mod cast_bar;
mod feedback;

pub use auto_attack::AttackTarget;

use bevy::prelude::*;

/// Tracks whether the local player is currently auto-attacking.
#[derive(Resource, Default)]
pub struct IsAttacking(pub bool);

/// The spell IDs the player's character knows, received from the server on login.
#[derive(Resource)]
pub struct KnownAbilities(pub Vec<u32>);

use crate::application::{AppState, EnterGame};
use crate::networking::NetworkingSet;
use action_bar::AbilityCooldowns;
use game_core::spells::{SpellLibrary, SpellLibraryHandle};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatSet {
    ProcessInput,
    Feedback,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IsAttacking::default());

        app.add_observer(auto_attack::on_attack_target);
        app.add_observer(auto_attack::on_escape);
        app.add_observer(on_enter_game);

        app.add_systems(
            Update,
            (
                action_bar::spawn_action_bar,
                action_bar::handle_ability_input,
                action_bar::handle_ability_click,
                action_bar::tick_cooldowns,
                action_bar::update_slot_visuals,
                cast_bar::manage_cast_bar,
                cast_bar::update_cast_bar,
            )
                .in_set(CombatSet::ProcessInput)
                .after(NetworkingSet::Receive)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                feedback::handle_combat_hits,
                feedback::update_floating_combat_text,
                feedback::update_hit_flash,
                feedback::log_combat_hits,
            )
                .in_set(CombatSet::Feedback)
                .after(CombatSet::ProcessInput)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn on_enter_game(event: On<EnterGame>, mut commands: Commands, assets: Res<AssetServer>) {
    let response = &event.0;
    commands.insert_resource(KnownAbilities(response.known_abilities.clone()));
    commands.insert_resource(AbilityCooldowns::default());

    let spells_handle = assets.load::<SpellLibrary>("spells.ron");
    commands.insert_resource(SpellLibraryHandle(spells_handle));
}
