use crate::components::{MobSpawner, SpawnedMob};
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

pub fn spawn_mobs(
    mut commands: Commands,
    time: Res<Time>,
    mut q_spawners: Query<(Entity, &mut MobSpawner, &Transform)>,
    q_mobs: Query<&SpawnedMob>,
) {
    for (spawner_entity, mut spawner, transform) in q_spawners.iter_mut() {
        spawner.timer.tick(time.delta());
        if spawner.timer.just_finished() {
            let current_count = q_mobs
                .iter()
                .filter(|m| m.spawner == spawner_entity)
                .count();

            if current_count < spawner.max_mobs {
                let mut rng = rand::thread_rng();
                let x = rng.gen_range(-spawner.spawn_radius..spawner.spawn_radius);
                let z = rng.gen_range(-spawner.spawn_radius..spawner.spawn_radius);
                let spawn_pos = transform.translation + Vec3::new(x, 0.0, z);
                spawn_monster_entity(&mut commands, spawner_entity, spawn_pos);
            }
        }
    }
}

// TODO: Refactor and use character bundle
fn spawn_monster_entity(commands: &mut Commands, spawner: Entity, position: Vec3) {
    commands.spawn((
        SpawnedMob { spawner },
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        LockedAxes::ROTATION_LOCKED,
    ));
}
