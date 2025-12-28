use crate::components::{ClientIdComponent, Dead, Vitals};
use bevy::prelude::*;
use std::time::Duration;

const CORPSE_DESPAWN_DURATION: Duration = Duration::from_secs(150);

pub fn on_vitals_changed(
    mut commands: Commands,
    q_actors: Query<(Entity, &Vitals, Option<&ClientIdComponent>), Changed<Vitals>>,
) {
    for (entity, vitals, client_id) in q_actors.iter() {
        if vitals.hp <= 0 {
            commands.entity(entity).insert(Dead {
                despawn_timer: Timer::new(CORPSE_DESPAWN_DURATION, TimerMode::Once),
            });
            continue;
        }
    }
}
