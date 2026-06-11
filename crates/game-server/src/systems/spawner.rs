use crate::{
    ai::{
        AggroRadius, AiAbilityConfig, AiBehavior, AiBrain, AiMovement, LeashAnchor, ThreatTable,
        Wander,
    },
    assets::{
        AiBehaviorDef, ContentId, MonsterDef, MonsterLibrary, MonsterLibraryHandle, SpellLibrary,
        SpellLibraryHandle,
    },
    components::{Abilities, AssetIdComponent, MobSpawner, MonsterId, Spawned},
    systems::ActorBundle,
};
use bevy::prelude::*;
use game_core::components::Vitals;
use rand::Rng;
use std::time::Duration;

pub fn setup_spawners(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0., 0., 0.),
        MobSpawner {
            mob_id: ContentId::from("skeleton-warrior"),
            max_mobs: 10,
            spawn_radius: 25.,
            level_range: std::ops::Range { start: 1, end: 4 },
            timer: Timer::new(Duration::from_secs(5), TimerMode::Repeating),
        },
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_mobs(
    mut commands: Commands,
    time: Res<Time>,
    mut q_spawners: Query<(Entity, &mut MobSpawner, &Transform)>,
    q_mobs: Query<&Spawned>,
    library_handle: Res<MonsterLibraryHandle>,
    monster_assets: Res<Assets<MonsterLibrary>>,
    spell_library_handle: Res<SpellLibraryHandle>,
    spell_assets: Res<Assets<SpellLibrary>>,
) {
    let Some(library) = monster_assets.get(&library_handle.0) else {
        tracing::info!("still waiting for monsters library to load");
        return;
    };

    let spell_library = spell_assets.get(&spell_library_handle.0);

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
                    &spawner.mob_id,
                    blueprint,
                    spawner_entity,
                    spawn_transform,
                    level,
                    spell_library,
                );
                tracing::info!(
                    name = %blueprint.name,
                    postition = %spawn_transform.translation,
                    %level,
                    "spawning mob"
                );
            }
        }
    }
}

fn spawn_monster_entity(
    commands: &mut Commands,
    monster_id: &ContentId,
    blueprint: &MonsterDef,
    spawner: Entity,
    transform: Transform,
    level: i32,
    spell_library: Option<&SpellLibrary>,
) {
    let vitals = Vitals {
        hp: blueprint.hp,
        max_hp: blueprint.hp,
    };
    let actor_bundle = ActorBundle::new(&blueprint.name, transform, vitals, level);

    // Build spell cooldown map from the spell library
    let spell_cooldowns: std::collections::HashMap<u32, f32> = spell_library
        .map(|lib| {
            blueprint
                .abilities
                .iter()
                .filter_map(|&spell_id| {
                    lib.spells.get(&spell_id).map(|spell| (spell_id, spell.cooldown))
                })
                .collect()
        })
        .unwrap_or_default();

    let abilities = Abilities::new(&blueprint.abilities, &spell_cooldowns);

    let mut entity_commands = commands.spawn((
        MonsterId(*monster_id),
        Spawned { spawner },
        actor_bundle,
        AssetIdComponent(blueprint.asset_id),
        abilities,
    ));

    // Attach AI components if AI is configured
    if let Some(ai_def) = &blueprint.ai {
        let behavior = match ai_def.behavior {
            AiBehaviorDef::Aggressive => AiBehavior::Aggressive,
            AiBehaviorDef::Neutral => AiBehavior::Neutral,
        };

        entity_commands.insert((
            AiBrain {
                behavior,
                ..default()
            },
            ThreatTable::default(),
            AggroRadius(ai_def.aggro_radius),
            LeashAnchor {
                position: transform.translation,
                max_range: ai_def.leash_range,
            },
            AiAbilityConfig {
                priorities: ai_def.ability_priorities.clone(),
            },
            AiMovement::default(),
        ));

        if let Some(wander_def) = &ai_def.wander {
            entity_commands.insert(Wander::new(wander_def.radius, wander_def.pause_duration));
        }
    }
}
