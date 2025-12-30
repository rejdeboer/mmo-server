use crate::{
    assets::{LootDb, LootTable},
    components::{
        ClientIdComponent, Dead, InterestedClients, Loot, LootEntry, MonsterId,
        MovementSpeedComponent, Tapped, Vitals,
    },
    messages::{OutgoingMessage, OutgoingMessageData},
};
use bevy::prelude::*;
use rand::{Rng, thread_rng};
use std::{collections::HashMap, time::Duration};

const CORPSE_DESPAWN_DURATION: Duration = Duration::from_secs(150);

#[derive(EntityEvent)]
pub struct EntityDeath(pub Entity);

#[derive(Bundle)]
/// Components that living world actors have
struct LivingBundle {
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
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
            despawn_timer: Timer::new(CORPSE_DESPAWN_DURATION, TimerMode::Once),
        });

    let Ok((interested, victim_client_id)) = q_victim.get(entity) else {
        return tracing::error!(?entity, "could not retrieve victim components");
    };

    let outgoing_msg = OutgoingMessageData::Death { entity };
    outgoing_msg.broadcast(&interested.clients, &mut writer);
    if let Some(victim_client_id) = victim_client_id {
        writer.write(OutgoingMessage {
            client_id: victim_client_id.0,
            data: outgoing_msg,
        });
    }
}

pub fn reward_kill(
    event: On<EntityDeath>,
    mut commands: Commands,
    q_victim: Query<(Option<&MonsterId>, Option<&Tapped>)>,
    loot_db: LootDb,
    mut writer: MessageWriter<OutgoingMessage>,
) {
    let entity = event.0;
    let Ok((monster_id, tapped)) = q_victim.get(entity) else {
        return tracing::error!(?entity, "could not retrieve victim components");
    };

    let Some(killer_client_id) = tapped.map(|t| t.owner_id) else {
        return tracing::debug!("entity was not killed by a player");
    };

    let Some(monster_id) = monster_id else {
        return tracing::debug!("killed entity is not a monster");
    };

    let Some(loot_tables) = loot_db.get_monster_loot_tables(monster_id) else {
        return tracing::error!(?monster_id, "failed to get monster loot tables");
    };

    let loot_entries = generate_loot(loot_tables);
    commands.entity(entity).insert(Loot {
        entries: loot_entries.clone(),
        owner_id: killer_client_id,
    });

    writer.write(OutgoingMessage {
        client_id: killer_client_id,
        data: OutgoingMessageData::KillReward {
            victim: entity,
            loot: loot_entries,
        },
    });
}

fn generate_loot<'a>(tables: impl Iterator<Item = &'a LootTable>) -> Vec<LootEntry> {
    let mut rng = thread_rng();
    let mut loot: HashMap<u32, u16> = HashMap::new();

    for table in tables {
        for entry in table {
            if rng.gen_bool(entry.chance as f64) {
                let count = rng.gen_range(entry.min..=entry.max);
                if count > 0 {
                    *loot.entry(entry.item_id).or_insert(0) += count;
                }
            }
        }
    }

    loot.into_iter()
        .map(|(item_id, quantity)| LootEntry { item_id, quantity })
        .collect()
}
