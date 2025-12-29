use crate::components::{Dead, MovementSpeedComponent, Vitals};
use bevy::prelude::*;
use std::time::Duration;

const CORPSE_DESPAWN_DURATION: Duration = Duration::from_secs(150);

#[derive(EntityEvent)]
pub struct EntityDeath(pub Entity);

#[derive(Bundle)]
/// Components that living world actors have
struct LivingBundle {
    vitals: Vitals,
    movement_speed: MovementSpeedComponent,
}

pub fn on_entity_death(event: On<EntityDeath>, mut commands: Commands) {
    let entity = event.0;
    commands
        .entity(entity)
        .remove::<LivingBundle>()
        .insert(Dead {
            despawn_timer: Timer::new(CORPSE_DESPAWN_DURATION, TimerMode::Once),
        });
}
