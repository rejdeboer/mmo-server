use crate::{
    core::{ClientIdComponent, Dead, InterestedClients},
    networking::{OutgoingMessage, OutgoingMessageData},
    telemetry::PLAYER_DEATHS_TOTAL_METRIC,
};
use bevy::prelude::*;
use game_core::components::{MovementSpeedComponent, Vitals};

#[derive(EntityEvent)]
pub struct EntityDeath(pub Entity);

#[derive(Bundle)]
struct LivingBundle {
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
}

pub fn on_vitals_changed(
    mut commands: Commands,
    q_actors: Query<(Entity, &Vitals, Option<&ClientIdComponent>), Changed<Vitals>>,
) {
    for (entity, vitals, _client_id) in q_actors.iter() {
        if vitals.hp <= 0 {
            commands.entity(entity).trigger(EntityDeath);
            continue;
        }
    }
}

pub fn on_entity_death(
    event: On<EntityDeath>,
    mut commands: Commands,
    q_victim: Query<(&InterestedClients, Option<&ClientIdComponent>)>,
    mut writer: MessageWriter<OutgoingMessage>,
) {
    let entity = event.0;
    commands
        .entity(entity)
        .remove::<LivingBundle>()
        .insert(Dead {
            despawn_timer: Timer::new(std::time::Duration::from_secs(150), TimerMode::Once),
        });

    let Ok((interested, victim_client_id)) = q_victim.get(entity) else {
        return tracing::error!(?entity, "could not retrieve victim components");
    };

    let mut recipients = Vec::with_capacity(interested.clients.len() + 1);
    recipients.extend(interested.clients.iter().copied());

    if let Some(victim_client_id) = victim_client_id {
        recipients.push(victim_client_id.0);
        metrics::counter!(PLAYER_DEATHS_TOTAL_METRIC).increment(1);
        tracing::info!(?entity, client_id = %victim_client_id.0, "player died");
    }

    let outgoing_msg = OutgoingMessageData::Death { entity };
    writer.write(OutgoingMessage {
        recipients,
        data: outgoing_msg,
    });
}

pub fn tick_corpse_despawn_timers(
    mut commands: Commands,
    mut q_dead: Query<(Entity, &mut Dead, &InterestedClients)>,
    time: Res<Time>,
    mut writer: MessageWriter<OutgoingMessage>,
) {
    for (entity, mut dead, interested) in q_dead.iter_mut() {
        dead.despawn_timer.tick(time.delta());
        if dead.despawn_timer.is_finished() {
            let outgoing_msg = OutgoingMessageData::DespawnCorpse(entity);
            let recipients = interested.clients.iter().copied().collect();
            writer.write(OutgoingMessage {
                recipients,
                data: outgoing_msg,
            });
            commands.entity(entity).despawn();
        }
    }
}
