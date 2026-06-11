use super::components::{AiAbilityConfig, AiBrain, AiState};
use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    components::{Abilities, Casting},
    messages::CastSpellActionMessage,
};
use bevy::prelude::*;

/// AI selects and casts the best available ability against its current target.
#[allow(clippy::type_complexity)]
pub fn ai_select_ability(
    q_mobs: Query<(
        Entity,
        &AiBrain,
        &Transform,
        &Abilities,
        &AiAbilityConfig,
        Option<&Casting>,
    )>,
    q_targets: Query<&Transform, Without<AiBrain>>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
    mut writer: MessageWriter<CastSpellActionMessage>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        return;
    };

    for (entity, brain, transform, abilities, config, casting) in q_mobs.iter() {
        // Only act in Combat state
        let AiState::Combat { target } = &brain.state else {
            continue;
        };

        // Already casting — respect the same "one cast at a time" rule as players
        if casting.is_some() {
            continue;
        }

        let Ok(target_transform) = q_targets.get(*target) else {
            continue;
        };

        let distance = transform.translation.distance(target_transform.translation);

        // Pick best available ability: off cooldown, in range, highest priority
        let best = abilities
            .known
            .iter()
            .filter(|a| a.cooldown.is_finished())
            .filter(|a| {
                library
                    .spells
                    .get(&a.spell_id)
                    .is_some_and(|spell| distance <= spell.range)
            })
            .max_by_key(|a| config.priorities.get(&a.spell_id).copied().unwrap_or(0));

        if let Some(ability) = best {
            writer.write(CastSpellActionMessage {
                caster_entity: entity,
                target_entity: *target,
                spell_id: ability.spell_id,
            });
        }
    }
}
