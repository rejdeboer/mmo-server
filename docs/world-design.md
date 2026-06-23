# World Creation Workflow

## Overview

The game world is authored in Blender, exported as GLB files, and loaded by both
the game client and game server from a shared asset directory. A zone definition
file (RON) acts as the single source of truth for prop placement and monster
spawner configuration, replacing any hardcoded spawn logic in Rust.

### Principles

1. **Single source of truth** -- one set of assets, one zone definition. The
   client and server must never disagree about terrain geometry or prop positions.
2. **Blender as the level editor** -- all world authoring happens in Blender.
   A Python export script produces the GLB and RON files consumed by the engine.
3. **Server skips visuals, not files** -- the server loads the same GLB files as
   the client but skips materials and textures via
   `load_materials = RenderAssetUsages::empty()`. No separate "server-only"
   exports are needed.
4. **Data-driven spawning** -- monster spawners, player spawn points, and prop
   placements are defined in the zone RON file, not in Rust code.

## Asset Management

Licensed and authored assets live in a **private Git repository** tracked with
**Git LFS** for binary files. This repo is added to the main project as a
**Git submodule** at the `assets/` directory.

### Why Git LFS + Submodule

- Version pinning is automatic: the submodule commit hash ties assets to a
  specific code revision.
- Single command setup: `git submodule update --init`.
- Works offline after initial clone.
- CI/CD just needs `git submodule update` in the build step.
- Asset sizes for a single 500x500 zone are well within Git LFS limits (~1-2 GB).

### When to Reconsider

If the asset repo exceeds ~5 GB or binary churn becomes expensive on LFS
hosting, migrate to an S3 bucket with a download script and lock file. The
directory structure and symlink setup described below remain the same regardless
of the storage backend.

## Directory Structure

```
mmo-server/
  assets/                              # Git submodule (private repo, Git LFS)
    world/
      terrain/
        starting_meadow.glb            # Terrain mesh for the zone
      props/
        tree_oak.glb                   # Individual prop, modeled at origin
        rock_large.glb
        house_tavern.glb
      zones/
        starting_meadow.ron            # Zone definition (exported from Blender)
    sounds/
      whisper-received.ogg
      party-invite.ogg
    icons/
      3.jpg
      4.jpg
    textures/
      ...

  crates/
    game-client/assets/
      world   -> ../../../assets/world    # Symlink into submodule
      sounds  -> ../../../assets/sounds   # Symlink into submodule
      icons   -> ../../../assets/icons    # Symlink into submodule
      spells.ron                          # Client-specific, stays here

    game-server/assets/
      world   -> ../../../assets/world    # Symlink into submodule
      monsters.ron                        # Server-specific, stays here
      items.ron
      loot_tables.ron
      spells.ron

  zones/                                 # Blender source files (not shipped)
    starting_meadow.blend

  tools/
    blender_export_zone.py               # Blender Python export script
    setup_assets.sh                      # Creates symlinks after submodule init
```

### Symlinks

Both `game-client` and `game-server` access shared assets through symlinks
pointing into the submodule. This lets Bevy's `AssetPlugin` (which serves a
single directory) resolve paths like `world/terrain/starting_meadow.glb`
identically from both crates.

Git tracks symlinks natively on Unix. The `tools/setup_assets.sh` script creates
them after `git submodule update --init` for convenience. On Windows, developer
mode or `mklink` is required (not a concern for the Linux-based K8s deployment).

## Blender Workflow

### Zone File (.blend)

One `.blend` file per zone, stored in `zones/` (source files, not shipped). The
scene contains:

| Object Type | Blender Representation | Custom Properties |
|-------------|----------------------|-------------------|
| Terrain | Mesh object | `type = "terrain"` |
| Prop | Empty (or linked collection for visual preview) | `type = "prop"`, `prop_id = "tree_oak"`, `collision = "trimesh"` / `"convex"` / `"none"` |
| Spawner | Empty | `type = "spawner"`, `monster_id`, `max_count`, `radius`, `level_min`, `level_max`, `respawn_secs` |
| Player spawn | Empty | `type = "player_spawn"` |

Organize objects with Blender collections (e.g., `Terrain`, `Props/Trees`,
`Props/Rocks`, `Spawners`).

### Prop Files (.glb)

Each prop is its own `.blend` and exported `.glb`:

- Modeled at the origin, facing -Z (Blender/glTF convention).
- Optionally includes a low-poly child mesh named `collision` for server-side
  physics (e.g., a cylinder for a tree trunk instead of the full foliage mesh).
- Exported to `assets/world/props/<prop_id>.glb`.

### Export Script

`tools/blender_export_zone.py` runs from the command line:

```
blender zones/starting_meadow.blend --background \
  --python tools/blender_export_zone.py -- \
  --output assets/world/
```

The script:

1. Finds objects tagged `type = "terrain"`, exports them as a single
   `terrain/<zone_name>.glb`.
2. Iterates empties tagged `type = "prop"`, collects their transforms and
   `prop_id` values.
3. Iterates empties tagged `type = "spawner"`, collects spawn configuration.
4. Finds the `type = "player_spawn"` empty.
5. Writes `zones/<zone_name>.ron` with all collected data.
6. Validates that every referenced `prop_id` has a corresponding `.glb` in the
   props directory.

## Zone Definition Format

The zone RON file is the glue between Blender and the engine:

```ron
ZoneDef(
    id: "starting_meadow",
    terrain: "world/terrain/starting_meadow.glb",
    player_spawn: (0.0, 5.0, 0.0),
    props: [
        PropInstance(
            asset: "world/props/tree_oak.glb",
            transform: (
                translation: (10.0, 0.0, -5.0),
                rotation: (0.0, 0.7071, 0.0, 0.7071),
                scale: (1.0, 1.0, 1.0),
            ),
            collision: TrimeshFromMesh,
        ),
        PropInstance(
            asset: "world/props/rock_large.glb",
            transform: (
                translation: (25.0, 0.0, 12.0),
                rotation: (0.0, 0.0, 0.0, 1.0),
                scale: (2.0, 2.0, 2.0),
            ),
            collision: ConvexHull,
        ),
    ],
    spawn_points: [
        SpawnPoint(
            id: "skeleton_camp_01",
            position: (50.0, 0.0, 30.0),
            radius: 25.0,
            monster_id: "skeleton-warrior",
            max_count: 10,
            level_range: (1, 3),
            respawn_secs: 5.0,
        ),
    ],
)
```

## Client vs. Server Loading

Both crates load the same zone RON and GLB files. They differ in what they do
with the data:

| Aspect | Client | Server |
|--------|--------|--------|
| Terrain GLB | Full materials, rendered | `load_materials = empty()`, trimesh collider only |
| Prop GLBs | Full materials, rendered | Only loads props where `collision != None`, collider only |
| Zone RON | Reads terrain path + prop placements | Reads terrain path + prop placements + spawn points |
| Spawn points | Ignored | Creates `MobSpawner` entities |
| Player spawn | Used for initial camera position | Used for character spawn on first login |

### Shared Types

The `ZoneDef`, `PropInstance`, and `SpawnPoint` types live in `game-core` along
with the RON asset loader registration, since both crates need to deserialize the
same format.

Each crate has its own zone loading system that interprets the deserialized data
according to its role (client spawns visuals, server spawns colliders and
spawners).

## Props and Collision

Props should have efficient collision representations:

- **Decorative only** (`collision: None`): grass, flowers, small ground clutter.
  Client renders them; server ignores them entirely.
- **Convex hull** (`collision: ConvexHull`): rocks, crates, simple shapes. Fast
  collision, good enough for most solid objects.
- **Trimesh** (`collision: TrimeshFromMesh`): terrain, buildings, complex
  walkable geometry. More expensive but accurate.
- **Dedicated collision mesh**: For props like trees where the visual mesh is
  complex (5k+ triangles) but the collision should be a simple cylinder, add a
  low-poly child mesh named `collision` in Blender. The server uses only that
  child mesh; the client uses both.

## Instancing

Bevy instances mesh data automatically. Spawning 200 entities that reference
`world/props/tree_oak.glb` loads the mesh once and shares it. The zone RON file
can reference the same prop hundreds of times with different transforms at
negligible memory cost.

## Iteration Loop

1. Edit terrain or prop placement in Blender.
2. Run the export script (one command).
3. Restart the game server/client. With `AssetPlugin` file watching enabled,
   some changes can be picked up without a restart.

## Future Considerations

### World Streaming / Chunking

Not needed at 500x500 units. The existing spatial grid (128-unit cells) can
inform chunk boundaries later. If the world grows beyond ~2000x2000, split
terrain and prop placements along grid boundaries and load/unload chunks based on
player position.

### Zone Transitions

Listed as a TODO: handoff between game-server instances. When implemented, each
zone is a separate `ZoneDef` served by its own game-server instance. The
web-server coordinates the handoff.

### Heightmap Terrain

Not planned. Bevy has no built-in heightmap terrain system. A sculpted mesh
exported from Blender gives full artistic control and integrates directly with
the existing `ColliderConstructorHierarchy` setup. If terrain scale demands it
later, evaluate `bevy_terrain` or a custom heightmap solution.
