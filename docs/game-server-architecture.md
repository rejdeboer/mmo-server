# Game Server Modular Architecture

## Overview

The game server is organized into domain-specific Bevy plugins, each owning its
systems, messages, observers, and system sets. Cross-plugin ordering is
configured in `application.rs` using the sets each plugin exposes.

## Module Layout

```
crates/game-server/src/
├── assets/                  Game data definitions and loading
│   ├── mod.rs               ContentPlugin, setup_content, re-exports
│   ├── content_id.rs        ContentId (hashed string identifier)
│   ├── monsters.rs          MonsterId, MonsterDef, MonsterLibrary
│   ├── spells.rs            SpellDef, SpellLibrary
│   ├── items.rs             ItemDef, ItemLibrary
│   └── loot.rs              LootTableEntry, LootTableLibrary, LootDb SystemParam
├── core/                    Shared actor types and cross-cutting components
│   ├── mod.rs
│   ├── actor.rs             ActorBundle, CharacterBundle
│   └── components.rs        ServerTick, GridCell, InterestedClients, etc.
├── ai/                      AI behaviors
│   ├── mod.rs               AiPlugin
│   └── ...
├── networking/              Network I/O, transport, interest management, state sync
│   ├── mod.rs               NetworkingPlugin(ServerSettings), NetworkingSet, renet/netcode bootstrap
│   ├── messages.rs          OutgoingMessage, VisibilityChangedMessage
│   ├── action.rs            Client packet dispatch
│   ├── connection.rs        Connect/disconnect lifecycle
│   ├── sync.rs              Movement, event, and visibility sync
│   └── visibility.rs        Interest management (who sees what)
├── combat/                  Damage dealing and receiving
│   ├── mod.rs               CombatPlugin, CombatSet
│   ├── messages.rs          CastSpellAction, ApplySpellEffect, Start/StopAttack
│   ├── auto_attack.rs       Melee swing loop, AutoAttack component
│   ├── spells.rs            Cast, tick, apply, cooldowns, Casting/Abilities components
│   └── vitals.rs            Health change detection, death, corpse despawn
├── world/                   Physical simulation and spatial indexing
│   ├── mod.rs               WorldPlugin, WorldSet, SpatialGrid
│   ├── messages.rs          MoveActionMessage, JumpActionMessage
│   ├── movement.rs          Character controller, ground check, server tick
│   ├── spatial_grid.rs      Grid rebuild system (update_spatial_grid)
│   └── spawner.rs           MobSpawner, Spawned, mob spawning
├── social/                  Player communication
│   ├── mod.rs               SocialPlugin, SocialSet, IncomingChatMessage
│   ├── chat.rs              Proximity/channel chat
│   └── party.rs             NATS party updates, PartySubscription
├── economy/                 Items, loot, future inventory
│   ├── mod.rs               EconomyPlugin
│   └── loot.rs              reward_kill observer, LootEntry, Loot component
├── observability/           Telemetry
│   ├── mod.rs               ObservabilityPlugin
│   └── metrics.rs           Prometheus gauge/histogram updates
├── application.rs           Assembles plugins, app runner (headless/debug), cross-plugin ordering
├── main.rs
└── lib.rs
```

## Per-Plugin System Sets

Each plugin defines its own system set enum. Plugins register their systems
into their own sets, and `application.rs` configures ordering *between* sets.

### NetworkingSet

| Variant           | Schedule         | Systems                                       |
|-------------------|------------------|-----------------------------------------------|
| `ReceiveInput`    | FixedPreUpdate   | `process_client_actions`, `process_client_movements` |
| `UpdateVisibility`| FixedPostUpdate  | `update_player_visibility`, `sync_visibility` |
| `Sync`            | FixedPostUpdate  | `sync_server_events`, `sync_movement`         |

### CombatSet

| Variant          | Schedule        | Systems                                         |
|------------------|-----------------|--------------------------------------------------|
| `ProcessActions` | FixedPreUpdate  | `process_spell_casts`, `process_start_attack`, `process_stop_attack` |
| `Tick`           | FixedUpdate     | `on_vitals_changed`, `tick_casting`, `tick_ability_cooldowns`, `tick_auto_attack`, `cancel_auto_attack_on_death`, `tick_corpse_despawn_timers` |
| `ApplyEffects`   | FixedPostUpdate | `apply_spell_effect`                            |

### WorldSet

| Variant           | Schedule       | Systems                                        |
|-------------------|----------------|------------------------------------------------|
| `Tick`            | FixedPreUpdate | `increment_server_tick`                        |
| `PreProcess`      | FixedPreUpdate | `check_ground_status`                          |
| `ProcessMovement` | FixedPreUpdate | `process_move_action_messages`, `process_jump_action_messages` |

### SocialSet

| Variant          | Schedule       | Systems                    |
|------------------|----------------|----------------------------|
| `ReceiveUpdates` | FixedPreUpdate | `process_party_updates`    |
| `ProcessChat`    | FixedPreUpdate | `process_incoming_chat`    |

## The `core` Module

Contains types that define what an entity *is*, used across multiple plugins:

- `ActorBundle` / `CharacterBundle` - entity archetypes
- `ServerTick` - global tick counter resource
- `GridCell` - spatial partition membership
- `InterestedClients` - which clients observe an entity
- `VisibleEntities` - which entities a player can see
- Identity: `ClientIdComponent`, `CharacterIdComponent`, `AssetIdComponent`,
  `NameComponent`
- State markers: `Dead`, `Tapped`

## The `assets` Module

Owns all game data definitions loaded from RON files, plus the `ContentPlugin`
that registers RON asset loaders and spawns the world scene:

- `ContentId` - hashed string identifier for asset content
- `MonsterId` - component wrapping a `ContentId` for monsters
- `MonsterDef` / `MonsterLibrary` - monster type definitions
- `SpellDef` / `SpellLibrary` - spell definitions
- `ItemDef` / `ItemLibrary` - item definitions
- `LootTableEntry` / `LootTableLibrary` - loot tables
- `LootDb` - `SystemParam` for querying monster loot tables

Domain-specific components live in their owning plugin:
- `AutoAttack`, `Casting`, `Abilities` → combat
- `MobSpawner`, `Spawned`, `SpatialGrid` → world
- `Loot`, `LootEntry` → economy
