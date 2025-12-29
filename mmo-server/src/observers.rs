use crate::components::Dead;
use bevy::prelude::*;
use std::time::Duration;

const CORPSE_DESPAWN_DURATION: Duration = Duration::from_secs(150);

#[derive(EntityEvent)]
pub struct EntityDeath(pub Entity);

pub fn on_entity_death(event: On<EntityDeath>, mut commands: Commands) {
    commands.entity(event.0).insert(Dead {
        despawn_timer: Timer::new(CORPSE_DESPAWN_DURATION, TimerMode::Once),
    });
}
