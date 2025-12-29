use crate::{
    components::{ClientIdComponent, Vitals},
    observers::EntityDeath,
};
use bevy::prelude::*;

pub fn on_vitals_changed(
    mut commands: Commands,
    q_actors: Query<(Entity, &Vitals, Option<&ClientIdComponent>), Changed<Vitals>>,
) {
    for (entity, vitals, client_id) in q_actors.iter() {
        if vitals.hp <= 0 {
            commands.entity(entity).trigger(EntityDeath);
            continue;
        }
    }
}
