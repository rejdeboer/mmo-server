use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    components::{Casting, ClientIdComponent, InterestedClients},
    messages::{
        ApplySpellEffectMessage, CastSpellActionMessage, OutgoingMessage, OutgoingMessageData,
    },
};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

pub fn process_spell_casts(
    mut commands: Commands,
    mut reader: MessageReader<CastSpellActionMessage>,
    mut writer: MessageWriter<OutgoingMessage>,
    q_caster: Query<(
        &ClientIdComponent,
        &Transform,
        &InterestedClients,
        Option<&Casting>,
    )>,
    q_target: Query<&Transform>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        tracing::info!("still waiting for spells library to load");
        return;
    };

    for msg in reader.read() {
        let Ok((caster_client_id, caster_transform, interested, casting)) =
            q_caster.get(msg.caster_entity)
        else {
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
            .distance_squared(target_transform.translation)
            > spell.range * spell.range
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

        let outgoing_msg = OutgoingMessageData::StartCasting {
            entity: msg.caster_entity,
            spell_id: msg.spell_id,
        };
        writer.write_batch(interested.clients.iter().map(|client_id| OutgoingMessage {
            client_id: *client_id,
            data: outgoing_msg.clone(),
        }));
        writer.write(OutgoingMessage {
            client_id: caster_client_id.0,
            data: outgoing_msg,
        });
    }
}

pub fn tick_casting(
    mut commands: Commands,
    time: Res<Time>,
    mut q_casting: Query<(Entity, &mut Casting, &LinearVelocity)>,
    mut writer: MessageWriter<ApplySpellEffectMessage>,
) {
    for (entity, mut cast, velocity) in q_casting.iter_mut() {
        if velocity.length_squared() > 0.1 && !cast.castable_while_moving {
            commands.entity(entity).remove::<Casting>();
            tracing::debug!(?entity, "caster moved while casting stationary spell");
            continue;
        }

        cast.timer.tick(time.delta());
        if cast.timer.is_finished() {
            writer.write(ApplySpellEffectMessage {
                caster_entity: entity,
                target_entity: cast.target,
                spell_id: cast.spell_id,
            });
            commands.entity(entity).remove::<Casting>();
        }
    }
}

pub fn apply_spell_effect(
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
    mut reader: MessageReader<ApplySpellEffectMessage>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        return;
    };

    for msg in reader.read() {}
}
