mod auto_attack;
pub mod messages;
mod spells;
mod vitals;

use avian3d::prelude::*;
use bevy::prelude::*;

pub use messages::*;
pub use spells::{Abilities, Casting};
pub use vitals::EntityDeath;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Validate and begin combat actions (spell casts, start/stop attack).
    ProcessActions,
    /// Tick ongoing combat state (swing timers, cast bars, cooldowns).
    Tick,
    /// Apply resolved effects (spell damage).
    ApplyEffects,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CastSpellActionMessage>();
        app.add_message::<ApplySpellEffectMessage>();
        app.add_message::<StartAttackMessage>();
        app.add_message::<StopAttackMessage>();

        app.add_systems(
            FixedPreUpdate,
            (
                spells::process_spell_casts,
                auto_attack::process_start_attack,
                auto_attack::process_stop_attack,
            )
                .in_set(CombatSet::ProcessActions),
        );

        app.add_systems(
            FixedUpdate,
            (
                vitals::on_vitals_changed,
                spells::tick_casting,
                spells::tick_ability_cooldowns,
                auto_attack::tick_auto_attack,
                auto_attack::cancel_auto_attack_on_death,
                vitals::tick_corpse_despawn_timers,
            )
                .in_set(CombatSet::Tick),
        );

        app.add_systems(
            FixedPostUpdate,
            spells::apply_spell_effect
                .in_set(CombatSet::ApplyEffects)
                .after(PhysicsSystems::Last),
        );

        app.add_observer(vitals::on_entity_death);
    }
}
