use crate::{
    assets::{LootDb, LootTable, MonsterId},
    combat::EntityDeath,
    core::Tapped,
    networking::OutgoingMessage,
    networking::OutgoingMessageData,
    telemetry::{LOOT_ITEMS_GENERATED_TOTAL_METRIC, MOB_KILLS_TOTAL_METRIC},
};
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use protocol::models::ItemDrop;
use rand::{Rng, thread_rng};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct LootEntry {
    pub item_id: u32,
    pub quantity: u16,
}

impl From<LootEntry> for ItemDrop {
    fn from(value: LootEntry) -> Self {
        Self {
            item_id: value.item_id,
            quantity: value.quantity,
        }
    }
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Loot {
    pub entries: Vec<LootEntry>,
    pub owner_id: ClientId,
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

    metrics::counter!(MOB_KILLS_TOTAL_METRIC).increment(1);
    metrics::counter!(LOOT_ITEMS_GENERATED_TOTAL_METRIC).increment(loot_entries.len() as u64);
    tracing::debug!(
        ?monster_id,
        killer = %killer_client_id,
        item_count = loot_entries.len(),
        "loot generated from kill"
    );

    commands.entity(entity).insert(Loot {
        entries: loot_entries.clone(),
        owner_id: killer_client_id,
    });

    writer.write(OutgoingMessage {
        recipients: vec![killer_client_id],
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
