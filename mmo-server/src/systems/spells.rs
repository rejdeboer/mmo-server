use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    components::Casting,
    messages::{CastSpellActionMessage, OutgoingMessage},
};
use bevy::prelude::*;

pub fn process_spell_casts(
    mut commands: Commands,
    mut reader: MessageReader<CastSpellActionMessage>,
    mut writer: MessageWriter<OutgoingMessage>,
    mut q_caster: Query<(&Transform, Option<&Casting>)>,
    q_target: Query<(&Transform)>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        tracing::info!("still waiting for spells library to load");
        return;
    };

    for msg in reader.read() {
        let Ok((caster_transform, casting)) = q_caster.get(msg.caster_entity) else {
            tracing::warn!(
                caster = ?msg.caster_entity,
                "unable to find spell caster entity"
            );
            continue;
        };

        if casting.is_some() {
            tracing::debug!(caster = ?msg.caster_entity, "caster tried to cast while already casting");
            continue;
        }

        let Ok(target_transform) = q_target.get(msg.target_entity) else {
            tracing::debug!(caster = ?msg.caster_entity, target = ?msg.target_entity, "caster selected invalid target");
            continue;
        };

        let Some(spell) = library.spells.get(&msg.spell_id) else {
            tracing::debug!(caster = ?msg.caster_entity, spell_id = %msg.spell_id, "caster used invalid spell ID");
            continue;
        };

        if caster_transform
            .translation
            .distance(target_transform.translation)
            > spell.range
        {
            tracing::debug!(caster = ?msg.caster_entity, ?spell, "target is out of range");
            continue;
        }

        // TODO: Spell cooldowns
        commands.entity(msg.caster_entity).insert(Casting {
            spell_id: msg.spell_id,
            target: msg.target_entity,
            timer: Timer::from_seconds(spell.casting_duration, TimerMode::Once),
            castable_while_moving: spell.castable_while_moving,
        });
    }
}
