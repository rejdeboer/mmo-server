use crate::{
    assets::{MonsterBlueprint, MonsterLibrary, MonsterLibraryHandle},
    components::{MobSpawner, SpawnedMob, Vitals},
    systems::ActorBundle,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

pub fn spawn_mobs(
    mut commands: Commands,
    time: Res<Time>,
    mut q_spawners: Query<(Entity, &mut MobSpawner, &Transform)>,
    q_mobs: Query<&SpawnedMob>,
    library_handle: Res<MonsterLibraryHandle>,
    assets: Res<Assets<MonsterLibrary>>,
) {
    let Some(library) = assets.get(&library_handle.0) else {
        tracing::info!("still waiting for monsters library to load");
        return;
    };

    for (spawner_entity, mut spawner, transform) in q_spawners.iter_mut() {
        spawner.timer.tick(time.delta());
        if spawner.timer.just_finished() {
            let current_count = q_mobs
                .iter()
                .filter(|m| m.spawner == spawner_entity)
                .count();

            if current_count < spawner.max_mobs {
                let blueprint = library
                    .types
                    .get(&spawner.mob_id)
                    .expect("mob_id should match id in monsters.ron");

                let mut rng = rand::thread_rng();
                let x = rng.gen_range(-spawner.spawn_radius..spawner.spawn_radius);
                let z = rng.gen_range(-spawner.spawn_radius..spawner.spawn_radius);
                let level = rng.gen_range(spawner.level_range.clone());
                let spawn_transform = transform.with_translation(Vec3::new(x, 0.0, z));

                spawn_monster_entity(
                    &mut commands,
                    blueprint,
                    spawner_entity,
                    spawn_transform,
                    level,
                );
            }
        }
    }
}

fn spawn_monster_entity(
    commands: &mut Commands,
    blueprint: &MonsterBlueprint,
    spawner: Entity,
    transform: Transform,
    level: i32,
) {
    let vitals = Vitals {
        hp: blueprint.hp,
        max_hp: blueprint.hp,
    };
    let actor_bundle = ActorBundle::new(&blueprint.name, transform, vitals, level);
    commands.spawn((
        SpawnedMob { spawner },
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        LockedAxes::ROTATION_LOCKED,
    ));
}
