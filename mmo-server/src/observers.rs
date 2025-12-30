use crate::{
    assets::{ContentId, LootTableLibrary},
    components::{ClientIdComponent, Dead, InterestedClients, MovementSpeedComponent, Vitals},
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

// TODO: Return loot
fn generate_loot(table_ids: &[ContentId], library: &LootTableLibrary) {
    let mut rng = thread_rng();
    let mut loot: HashMap<u32, u16> = HashMap::new();

    for table_id in table_ids {
        let Some(table) = library.tables.get(table_id) else {
            tracing::debug!(?table_id, "failed to get loot table");
            continue;
        };

        for entry in table {
            if rng.gen_bool(entry.chance as f64) {
                let count = rng.gen_range(entry.min..=entry.max);
                if count > 0 {
                    *loot.entry(entry.item_id).or_insert(0) += count;
                }
            }
        }
    }
}
