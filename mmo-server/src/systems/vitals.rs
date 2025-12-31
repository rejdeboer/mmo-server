use crate::{
    components::{ClientIdComponent, Dead, Vitals},
    observers::EntityDeath,
};
use bevy::prelude::*;

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

pub fn tick_corpse_despawn_timers(
    mut commands: Commands,
    mut q_dead: Query<(Entity, &mut Dead)>,
    time: Res<Time>,
) {
    for (entity, mut dead) in q_dead.iter_mut() {
        dead.despawn_timer.tick(time.delta());
        if dead.despawn_timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
