use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    messages::{CastSpellActionMessage, OutgoingMessage},
};
use bevy::prelude::*;

pub fn process_spell_casts(
    mut reader: MessageReader<CastSpellActionMessage>,
    mut writer: MessageWriter<OutgoingMessage>,
    mut q_caster: Query<(&Transform)>,
    q_target: Query<(&Transform)>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        tracing::info!("still waiting for spells library to load");
        return;
    };

    for msg in reader.read() {
        let Ok(caster_transform) = q_caster.get(msg.player_entity) else {
            tracing::warn!(
                caster = ?msg.player_entity,
                "unable to find spell caster entity"
            );
            continue;
        };

        let Ok(target_transform) = q_target.get(msg.target_entity) else {
            tracing::debug!(caster = ?msg.player_entity, target = ?msg.target_entity, "caster selected invalid target");
            continue;
        };

        let Some(spell) = library.spells.get(&msg.spell_id) else {
            tracing::debug!(caster = ?msg.player_entity, spell_id = %msg.spell_id, "caster used invalid spell ID");
            continue;
        };

        if caster_transform
            .translation
            .distance(target_transform.translation)
            > spell.range
        {
            tracing::debug!(caster = ?msg.player_entity, ?spell, "target is out of range");
            continue;
        }
    }
}
