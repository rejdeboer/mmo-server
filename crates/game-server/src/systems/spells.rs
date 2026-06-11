use crate::{
    assets::{SpellLibrary, SpellLibraryHandle},
    components::{Abilities, Casting, ClientIdComponent, InterestedClients, Tapped},
    messages::{
        ApplySpellEffectMessage, CastSpellActionMessage, OutgoingMessage, OutgoingMessageData,
    },
};
use bevy::prelude::*;
use game_core::components::Vitals;

#[allow(clippy::type_complexity)]
pub fn process_spell_casts(
    mut commands: Commands,
    mut reader: MessageReader<CastSpellActionMessage>,
    mut writer: MessageWriter<OutgoingMessage>,
    mut q_caster: Query<(
        Option<&ClientIdComponent>,
        &Transform,
        &InterestedClients,
        Option<&Casting>,
        &mut Abilities,
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
        let Ok((caster_client_id, caster_transform, interested, casting, mut abilities)) =
            q_caster.get_mut(msg.caster_entity)
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

        let Some(ability) = abilities.known.iter().find(|a| a.spell_id == msg.spell_id) else {
            tracing::debug!(caster = ?msg.caster_entity, spell_id = %msg.spell_id, "caster does not know this spell");
            continue;
        };

        if !ability.cooldown.is_finished() {
            tracing::debug!(caster = ?msg.caster_entity, spell_id = %msg.spell_id, "spell is on cooldown");
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

        commands.entity(msg.caster_entity).insert(Casting {
            spell_id: msg.spell_id,
            target: msg.target_entity,
            timer: Timer::from_seconds(spell.casting_duration, TimerMode::Once),
            castable_while_moving: spell.castable_while_moving,
        });

        // Reset the ability cooldown
        if let Some(ability) = abilities
            .known
            .iter_mut()
            .find(|a| a.spell_id == msg.spell_id)
        {
            ability.cooldown.reset();
        }

        let outgoing_msg = OutgoingMessageData::StartCasting {
            entity: msg.caster_entity,
            spell_id: msg.spell_id,
        };

        let mut recipients = Vec::with_capacity(interested.clients.len() + 1);
        recipients.extend(interested.clients.iter().copied());
        if let Some(client_id) = caster_client_id {
            recipients.push(client_id.0);
        }

        writer.write(OutgoingMessage {
            recipients,
            data: outgoing_msg,
        });
    }
}

pub fn tick_casting(
    mut commands: Commands,
    time: Res<Time>,
    mut q_casting: Query<(
        Entity,
        &mut Casting,
        Ref<Transform>,
        Option<&ClientIdComponent>,
    )>,
    mut writer: MessageWriter<ApplySpellEffectMessage>,
) {
    for (entity, mut cast, transform, client_id) in q_casting.iter_mut() {
        // Cancel non-movable casts if the caster's Transform changed this tick
        // (i.e. they moved). With kinematic bodies we no longer have LinearVelocity,
        // so change detection on Transform is the reliable signal.
        if transform.is_changed() && !cast.castable_while_moving {
            commands.entity(entity).remove::<Casting>();
            tracing::debug!(?entity, "caster moved while casting stationary spell");
            continue;
        }

        cast.timer.tick(time.delta());
        if cast.timer.is_finished() {
            writer.write(ApplySpellEffectMessage {
                caster_entity: entity,
                caster_client_id: client_id.map(|c| c.0),
                target_entity: cast.target,
                spell_id: cast.spell_id,
            });
            commands.entity(entity).remove::<Casting>();
        }
    }
}

pub fn apply_spell_effect(
    mut commands: Commands,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
    mut reader: MessageReader<ApplySpellEffectMessage>,
    mut q_target: Query<(
        &mut Vitals,
        &InterestedClients,
        Option<&ClientIdComponent>,
        Option<&Tapped>,
    )>,
    mut writer: MessageWriter<OutgoingMessage>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        return;
    };

    for msg in reader.read() {
        let Some(spell) = library.spells.get(&msg.spell_id) else {
            tracing::warn!(spell_id = %msg.spell_id, "tried to apply invalid spell");
            continue;
        };

        let Ok((mut target_vitals, interested, target_client_id, tapped)) =
            q_target.get_mut(msg.target_entity)
        else {
            tracing::debug!(entity_id = ?msg.target_entity, "tried to apply spell to invalid entity");
            continue;
        };

        let outgoing_msg = OutgoingMessageData::SpellImpact {
            target_entity: msg.target_entity,
            spell_id: msg.spell_id,
            impact_amount: spell.damage,
        };

        let mut recipients = Vec::with_capacity(interested.clients.len() + 1);
        recipients.extend(interested.clients.iter().copied());

        if let Some(caster_client_id) = msg.caster_client_id {
            // NOTE: caster's own ID is not within the interested set
            if msg.caster_entity == msg.target_entity {
                recipients.push(caster_client_id);
            }

            // TODO: Healing something should not tap it
            // TODO: Notify interested clients of tap
            if target_client_id.is_none() && tapped.is_none() {
                commands.entity(msg.target_entity).insert(Tapped {
                    owner_id: caster_client_id,
                });
            }
        }

        writer.write(OutgoingMessage {
            recipients,
            data: outgoing_msg,
        });

        target_vitals.hp -= spell.damage;
    }
}

pub fn tick_ability_cooldowns(time: Res<Time>, mut q_abilities: Query<&mut Abilities>) {
    for mut abilities in q_abilities.iter_mut() {
        for ability in abilities.known.iter_mut() {
            ability.cooldown.tick(time.delta());
        }
    }
}
