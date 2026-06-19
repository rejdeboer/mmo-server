use crate::{
    combat::messages::{StartAttackMessage, StopAttackMessage},
    core::{ClientIdComponent, Dead, InterestedClients, Tapped},
    networking::{OutgoingMessage, OutgoingMessageData},
    telemetry::AUTO_ATTACKS_TOTAL_METRIC,
};
use bevy::prelude::*;
use game_core::components::Vitals;
use game_core::networking::NetworkId;
use protocol::server::AUTO_ATTACK_VISUAL_ID;

const MELEE_RANGE: f32 = 3.0;
const AUTO_ATTACK_SPEED: f32 = 2.0;
const AUTO_ATTACK_DAMAGE: i32 = 5;

#[derive(Component)]
pub struct AutoAttack {
    pub target: Entity,
    pub swing_timer: Timer,
}

#[allow(clippy::type_complexity)]
pub fn process_start_attack(
    mut commands: Commands,
    mut reader: MessageReader<StartAttackMessage>,
    q_attacker: Query<(Entity, Option<&AutoAttack>), (With<Vitals>, Without<Dead>)>,
    q_target: Query<Entity, (With<Vitals>, Without<Dead>)>,
) {
    for msg in reader.read() {
        let Ok((attacker_entity, existing_attack)) = q_attacker.get(msg.attacker_entity) else {
            tracing::debug!(
                attacker = ?msg.attacker_entity,
                "start_attack: attacker is dead or invalid"
            );
            continue;
        };

        if q_target.get(msg.target_entity).is_err() {
            tracing::debug!(
                target = ?msg.target_entity,
                "start_attack: target is dead or invalid"
            );
            continue;
        };

        if attacker_entity == msg.target_entity {
            tracing::debug!(attacker = ?attacker_entity, "cannot auto-attack self");
            continue;
        }

        // If already attacking this target, ignore
        if let Some(existing) = existing_attack
            && existing.target == msg.target_entity
        {
            continue;
        }

        // Start with a finished timer so the first swing fires immediately when in range
        let mut swing_timer = Timer::from_seconds(AUTO_ATTACK_SPEED, TimerMode::Repeating);
        swing_timer.tick(std::time::Duration::from_secs_f32(AUTO_ATTACK_SPEED));

        commands.entity(attacker_entity).insert(AutoAttack {
            target: msg.target_entity,
            swing_timer,
        });
    }
}

pub fn process_stop_attack(
    mut commands: Commands,
    mut reader: MessageReader<StopAttackMessage>,
    q_attacker: Query<Entity, With<AutoAttack>>,
) {
    for msg in reader.read() {
        if q_attacker.get(msg.attacker_entity).is_ok() {
            commands.entity(msg.attacker_entity).remove::<AutoAttack>();
        }
    }
}

pub fn tick_auto_attack(
    mut commands: Commands,
    time: Res<Time>,
    mut q_attackers: Query<(
        Entity,
        &mut AutoAttack,
        &Transform,
        Option<&ClientIdComponent>,
    )>,
    mut q_targets: Query<
        (
            &NetworkId,
            &Transform,
            &mut Vitals,
            &InterestedClients,
            Option<&Tapped>,
        ),
        Without<Dead>,
    >,
    mut writer: MessageWriter<OutgoingMessage>,
) {
    for (attacker_entity, mut auto_attack, attacker_transform, attacker_client_id) in
        q_attackers.iter_mut()
    {
        let Ok((target_network_id, target_transform, mut target_vitals, interested, tapped)) =
            q_targets.get_mut(auto_attack.target)
        else {
            // Target is dead or despawned, cancel auto-attack
            commands.entity(attacker_entity).remove::<AutoAttack>();
            continue;
        };

        let distance_sq = attacker_transform
            .translation
            .distance_squared(target_transform.translation);

        // Only tick the swing timer when in range (pause/resume behavior)
        if distance_sq > MELEE_RANGE * MELEE_RANGE {
            continue;
        }

        auto_attack.swing_timer.tick(time.delta());

        if !auto_attack.swing_timer.just_finished() {
            continue;
        }

        // Apply damage
        target_vitals.hp -= AUTO_ATTACK_DAMAGE;
        metrics::counter!(AUTO_ATTACKS_TOTAL_METRIC).increment(1);

        // Tap the target if this is the first hit from a player
        if let Some(client_id) = attacker_client_id
            && tapped.is_none()
        {
            commands.entity(auto_attack.target).insert(Tapped {
                owner_id: client_id.0,
            });
        }

        // Broadcast the hit to interested clients
        let mut recipients = Vec::with_capacity(interested.clients.len() + 1);
        recipients.extend(interested.clients.iter().copied());
        if let Some(client_id) = attacker_client_id {
            recipients.push(client_id.0);
        }

        writer.write(OutgoingMessage {
            recipients,
            data: OutgoingMessageData::SpellImpact {
                target_network_id: *target_network_id,
                spell_id: AUTO_ATTACK_VISUAL_ID,
                impact_amount: AUTO_ATTACK_DAMAGE,
            },
        });
    }
}

/// Removes the AutoAttack component from entities that have died.
pub fn cancel_auto_attack_on_death(
    mut commands: Commands,
    q_dead_attackers: Query<Entity, (With<AutoAttack>, With<Dead>)>,
) {
    for entity in q_dead_attackers.iter() {
        commands.entity(entity).remove::<AutoAttack>();
    }
}
