# Server-Side AI Design

## Overview

Server-side AI for NPCs and monsters in the game-server crate. The design follows Bevy ECS patterns, reuses existing combat and movement systems, and is fully data-driven through RON configuration.

### Principles

1. **AI is only decision-making** — it decides *what* to do, not *how*. Execution flows through shared systems.
2. **Same rules as players** — mobs cast spells through the same pipeline, move with the same physics, die through the same vitals system.
3. **Abilities are an actor concern** — the `Abilities` component lives on any entity (player or mob), decoupled from AI.
4. **Data-driven** — behavior parameters come from `monsters.ron`, abilities from `spells.ron`.

## Architecture

```
crates/game-server/src/
├── ai/
│   ├── mod.rs              - AiPlugin, module declarations
│   ├── components.rs       - AI-specific components (AiConfig, ThreatTable, LeashAnchor)
│   ├── state.rs            - AiState enum + transition logic
│   ├── threat.rs           - Threat/aggro detection and tracking
│   ├── decision.rs         - Ability selection, target picking
│   ├── movement.rs         - AI movement (chase, patrol, return)
│   └── leash.rs            - Leash range enforcement and reset
```

## Layering

```
Abilities (data)          — what an actor CAN do (shared, any actor)
AI (behavior)             — what a mob DECIDES to do (AI-controlled entities only)
Cast pipeline (execution) — HOW it happens (validation, timing, effects — shared)
```

## Abilities System (Shared, Not AI-Specific)

### Component

```rust
#[derive(Component)]
pub struct Abilities {
    pub known: Vec<Ability>,
}

pub struct Ability {
    pub spell_id: u32,
    pub cooldown: Timer,
}
```

Lives on **every actor** that can use spells — players and mobs alike.

- **Players**: populated from the database on login (learned spells).
- **Mobs**: populated from `monsters.ron` at spawn time.

### Validation in `process_spell_casts`

The existing spell cast pipeline gains two additional checks:

```rust
// Caster must know the spell
let Some(ability) = abilities.known.iter().find(|a| a.spell_id == msg.spell_id) else {
    continue;
};

// Spell must be off cooldown
if !ability.cooldown.finished() {
    continue;
}
```

These apply identically to player and AI casts.

### Cooldown Reset

When a cast completes in `tick_casting`, the ability's cooldown timer is reset:

```rust
if let Some(ability) = abilities.known.iter_mut().find(|a| a.spell_id == cast.spell_id) {
    ability.cooldown.reset();
}
```

## AI State Machine

### AiState Component

```rust
#[derive(Component, Default)]
pub enum AiState {
    #[default]
    Idle,
    Patrol { waypoint_index: usize },
    Chase { target: Entity },
    Combat { target: Entity },
    Returning,
    Evading,
}
```

### Transitions

```
Idle/Patrol  -> Chase       : threat acquired (player enters aggro radius or deals damage)
Chase        -> Combat      : target within ability range
Combat       -> Chase       : target moves out of ability range
Chase/Combat -> Returning   : target lost (dead, disconnected, or threat decayed)
Chase/Combat -> Evading     : leash distance exceeded
Returning    -> Idle        : arrived at leash anchor
Evading      -> Idle        : arrived at leash anchor, HP reset
```

## Threat / Aggro System

### Components

```rust
#[derive(Component, Default)]
pub struct ThreatTable {
    pub entries: Vec<ThreatEntry>,
}

pub struct ThreatEntry {
    pub entity: Entity,
    pub threat: f32,
}

#[derive(Component)]
pub struct AggroRadius(pub f32);
```

### Systems

- **`detect_players`** — For mobs in `Idle`/`Patrol` state, queries the existing `SpatialGrid` for players within `AggroRadius`. Adds initial threat on proximity detection.
- **`update_threat`** — Listens to `ApplySpellEffectMessage`. When a mob takes damage, adds threat equal to damage dealt. Healing a mob's target generates 0.5x threat.
- **`select_target`** — Picks the highest-threat entry as the active target. Requires 10% threat differential to switch targets (hysteresis prevents flickering).
- **`decay_threat`** — Removes entries for dead or disconnected entities. Drops entries with no updates for 10 seconds.

## Leashing

### Component

```rust
#[derive(Component)]
pub struct LeashAnchor {
    pub position: Vec3,
    pub max_range: f32,
}
```

Set to the mob's spawn position on creation.

### Behavior

When distance from `LeashAnchor.position` exceeds `max_range`:

1. Transition to `Evading` state
2. Clear threat table
3. Move back to anchor at 2x movement speed
4. Regenerate HP to full over the walk back
5. Become untargetable (remove `Vitals` temporarily, re-add on arrival)
6. On arrival, transition to `Idle`

## AI Movement

Reuses the existing `character_move_step()` from `game-core`. AI entities already have `RigidBody::Kinematic`, capsule colliders, and `CharacterVelocityY` — the same physics setup as players.

### Component

```rust
#[derive(Component)]
pub struct AiMovement {
    pub target_position: Option<Vec3>,
    pub stop_distance: f32,
}
```

### Systems

- **`compute_ai_movement`** — For mobs in `Chase`/`Combat`/`Returning`/`Evading`, calculates a `MoveInput` direction toward `target_position`. Skips movement if the entity has a `Casting` component with `!castable_while_moving` (same rule as players).
- **`apply_ai_movement`** — Calls `character_move_step()` with the computed direction and `MovementSpeedComponent`. Handles gravity, ground snapping, and collision automatically.

Movement changes to `Transform` are picked up by the existing `sync_movement` system and broadcast to clients with no additional work.

### Patrol Behavior

For mobs in `Patrol` state:

- **Wander**: pick a random point within a radius of the leash anchor, walk there, pause, repeat.
- **Waypoints** (future): follow a predefined path of positions.

### Pathfinding (Phase 3)

Initial implementation uses direct-line movement (sufficient for open areas). Future phases add:

- Navigation mesh generated from `world.gltf` collision geometry at startup
- A* path queries with path caching per entity
- Path invalidation on obstruction

## AI Ability Selection

The AI system reads the shared `Abilities` component and applies priority-based decision-making.

### AI-Specific Config

```rust
#[derive(Component)]
pub struct AiAbilityConfig {
    pub priorities: HashMap<u32, u8>,  // spell_id -> priority
}
```

### System

```rust
pub fn ai_select_ability(
    q_mobs: Query<(Entity, &AiState, &Transform, &Abilities, &AiAbilityConfig, Option<&Casting>)>,
    q_targets: Query<&Transform>,
    library_handle: Res<SpellLibraryHandle>,
    assets: Res<Assets<SpellLibrary>>,
) {
    for (entity, state, transform, abilities, config, casting) in q_mobs.iter() {
        let AiState::Combat { target } = state else { continue };

        // Respect the same "one cast at a time" rule as players
        if casting.is_some() { continue; }

        let Ok(target_transform) = q_targets.get(*target) else { continue };
        let distance = transform.translation.distance(target_transform.translation);

        // Pick best ability: off cooldown, in range, highest priority
        let best = abilities.known.iter()
            .filter(|a| a.cooldown.finished())
            .filter(|a| {
                let spell = library.spells.get(&a.spell_id).unwrap();
                distance <= spell.range
            })
            .max_by_key(|a| config.priorities.get(&a.spell_id).copied().unwrap_or(0));

        if let Some(ability) = best {
            commands.write_message(CastSpellActionMessage {
                caster_entity: entity,
                target_entity: *target,
                spell_id: ability.spell_id,
            });
        }
    }
}
```

## Data Configuration

### `monsters.ron`

```ron
(
    monsters: {
        "skeleton-warrior": (
            hp: 50,
            movement_speed: 5.0,
            asset_id: 1,
            loot_tables: ["common-loot"],
            ai: (
                aggro_radius: 12.0,
                leash_range: 40.0,
                behavior: Aggressive,
                patrol: None,
                ability_priorities: { 100: 1 },
            ),
            abilities: [100],
        ),
        "goblin": (
            hp: 50,
            movement_speed: 7.0,
            asset_id: 2,
            loot_tables: ["common-loot"],
            ai: (
                aggro_radius: 18.0,
                leash_range: 50.0,
                behavior: Aggressive,
                patrol: Some(Wander(radius: 10.0, pause: 3.0)),
                ability_priorities: { 101: 1, 102: 3 },
            ),
            abilities: [101, 102],
        ),
    }
)
```

### `spells.ron`

```ron
(
    spells: {
        // Player spells
        0: ( name: "Smite", damage: 10, range: 30.0, cooldown: 1.5, casting_duration: 0.0, castable_while_moving: false, visual_id: 0 ),

        // Mob abilities (same system, same validation)
        100: ( name: "Skeleton Strike", damage: 8, range: 3.0, cooldown: 2.0, casting_duration: 0.0, castable_while_moving: false, visual_id: 10 ),
        101: ( name: "Goblin Slash", damage: 5, range: 2.5, cooldown: 1.5, casting_duration: 0.0, castable_while_moving: false, visual_id: 11 ),
        102: ( name: "Poison Spit", damage: 12, range: 15.0, cooldown: 8.0, casting_duration: 1.0, castable_while_moving: false, visual_id: 12 ),
    }
)
```

## System Execution Order

AI systems slot into the existing `FixedUpdate` schedule:

```
FixedUpdate:
    [existing: spawn_mobs, tick_casting, tick_corpse_despawn_timers, ...]

    // AI (new)
    detect_players
    update_threat
    decay_threat
    select_target
    check_leash
    ai_state_transitions
    ai_select_ability
    compute_ai_movement
    apply_ai_movement

FixedPostUpdate:
    [existing: sync_movement, sync_server_events, sync_visibility]
    // No changes — AI Transform updates are synced automatically
```

## Integration With Existing Systems

| Existing System | AI Integration |
|---|---|
| `process_spell_casts` | Validates AI casts (range, cooldown, known spell) — same as players |
| `tick_casting` | Ticks AI cast timers, cancels on movement — same as players |
| `apply_spell_effect` | Applies AI spell damage — same pipeline |
| `sync_movement` | Detects `Changed<Transform>` — AI movement synced automatically |
| `sync_server_events` | `StartCasting` / `SpellImpact` events sent to clients — players see mob casts |
| `visibility` | AI entities already have `InterestedClients` — no changes needed |
| `SpatialGrid` | AI uses it for player detection (already updated each tick) |
| `on_entity_death` | Mobs die through the same vitals/death pipeline |
| `character_move_step()` | AI uses the same physics-aware movement as players |

## Implementation Phases

| Phase | Scope |
|---|---|
| **1 - Core** | `AiState`, `ThreatTable`, `AggroRadius`, `LeashAnchor`, `Abilities` component, ability selection. Direct-line chase + melee attack through shared cast pipeline. |
| **2 - Polish** | Patrol/wander behavior, multi-ability priority, facing/rotation, evade invulnerability, threat from healing. |
| **3 - Pathfinding** | Navmesh generation from `world.gltf`, A* queries, path caching and smoothing. |
| **4 - Advanced** | Group AI (linked packs), boss scripted phases, flee behavior, call-for-help radius. |
