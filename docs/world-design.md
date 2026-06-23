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
   A Python export script produces the terrain GLB and zone RON file.
3. **Paid props ship as-is** -- licensed GLB assets are not re-exported or
   modified. The export script only references them by path.
4. **Server skips visuals, not files** -- the server loads the same GLB files as
   the client but skips materials and textures via
   `load_materials = RenderAssetUsages::empty()`. No separate "server-only"
   exports are needed.
5. **Data-driven spawning** -- monster spawners, player spawn points, and prop
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
        starting_meadow.glb            # Terrain mesh (exported from Blender)
      props/
        tent.glb                       # Paid asset, shipped as-is
        rock_large.glb                 # Paid asset, shipped as-is
        pine_tree.glb                  # Paid asset, shipped as-is
      textures/
        tent_albedo.png                # Textures referenced by prop GLBs
        tent_normal.png
        rock_albedo.png
        ...
      zones/
        starting_meadow.ron            # Zone definition (exported from Blender)
    sounds/
      whisper-received.ogg
      party-invite.ogg
    icons/
      3.jpg
      4.jpg

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

### Texture Path Resolution

Paid GLB assets reference external textures using relative paths. Bevy resolves
these paths **relative to the GLB file's location**. The texture directory must
be placed so that the GLB's internal references resolve correctly.

For example, if `props/tent.glb` internally references `../textures/tent_albedo.png`,
then the directory layout must have `textures/` as a sibling of `props/`:

```
world/
  props/tent.glb          # references "../textures/tent_albedo.png"
  textures/tent_albedo.png
```

Check the paid assets' internal texture references and mirror the expected
directory structure. If the paths don't match, use a tool like `gltf-transform`
to rewrite texture paths in the GLBs:

```
npx @gltf-transform/cli inspect tent.glb    # check current paths
```

If textures load correctly in Bevy (no magenta/pink materials), the layout is
correct.

### Symlinks

Both `game-client` and `game-server` access shared assets through symlinks
pointing into the submodule. This lets Bevy's `AssetPlugin` (which serves a
single directory) resolve paths like `world/terrain/starting_meadow.glb`
identically from both crates.

Git tracks symlinks natively on Unix. The `tools/setup_assets.sh` script creates
them after `git submodule update --init` for convenience. On Windows, developer
mode or `mklink` is required (not a concern for the Linux-based K8s deployment).

## Blender Workflow

### Props (Paid Assets)

Props are **pre-made licensed GLB files**. They are not modeled or re-exported --
they ship to the engine untouched. The Blender workflow only involves importing
and placing them:

1. Import paid GLBs into the zone's `.blend` file (File > Import > glTF 2.0).
2. Place, duplicate (Shift+D), rotate, and scale them as needed.
3. Name instances using the source GLB filename as a prefix:
   `tent.001`, `tent.002`, `rock_large.001`, `rock_large.002`, etc.
4. The Blender `.001` suffix is for deduplication only -- the export script
   strips it to determine the prop_id (which maps to `<prop_id>.glb`).

This keeps the Blender workflow natural: you see the full world visually and
place things like normal scene building.

### Terrain

Terrain is the only mesh authored and exported from Blender:

- Sculpt terrain directly in the zone `.blend` file.
- Place it in a `Terrain` collection or name it `terrain`.
- The export script selects the terrain mesh and exports it as a standalone GLB.

### Spawners and Player Spawn

These are invisible game logic objects with no visual representation:

- **Spawners**: Place an Empty where a monster camp should be. Add custom
  properties: `type = "spawner"`, `monster_id`, `max_count`, `radius`,
  `level_min`, `level_max`, `respawn_secs`.
- **Player spawn**: Place an Empty. Add custom property `type = "player_spawn"`.

### Zone File (.blend) Summary

One `.blend` file per zone, stored in `zones/` (source files, not shipped):

| Object Type | Blender Representation | Identification |
|-------------|----------------------|----------------|
| Terrain | Sculpted mesh | In `Terrain` collection or named `terrain` |
| Prop | Imported GLB, duplicated and placed | Named `<prop_id>.001`, `<prop_id>.002`, etc. |
| Spawner | Empty | Custom property `type = "spawner"` + config properties |
| Player spawn | Empty | Custom property `type = "player_spawn"` |

### Export Script

`tools/blender_export_zone.py` runs from the command line:

```
blender zones/starting_meadow.blend --background \
  --python tools/blender_export_zone.py -- \
  --output assets/world/
```

The script:

1. Finds the terrain mesh, exports it as `terrain/<zone_name>.glb`.
2. Iterates all mesh objects that are not terrain. Strips the `.001` suffix from
   each object name to get the `prop_id`. Collects their world-space transforms.
3. Iterates empties with `type = "spawner"`, collects spawn configuration from
   custom properties.
4. Finds the empty with `type = "player_spawn"`.
5. Writes `zones/<zone_name>.ron` with all collected data.
6. Validates that every referenced `prop_id` has a corresponding `.glb` in the
   props directory.

The script does **not** export prop GLBs -- only the terrain and the zone RON.

## Zone Definition Format

The zone RON file is the glue between Blender and the engine:

```ron
ZoneDef(
    id: "starting_meadow",
    terrain: "world/terrain/starting_meadow.glb",
    player_spawn: (0.0, 5.0, 0.0),
    props: [
        PropInstance(
            asset: "world/props/tent.glb",
            transform: (
                translation: (10.0, 0.0, -5.0),
                rotation: (0.0, 0.7071, 0.0, 0.7071),
                scale: (1.0, 1.0, 1.0),
            ),
            collision: ConvexHull,
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

Since the paid assets are low-poly, collision can be computed directly from the
visual mesh in most cases:

- **Decorative only** (`collision: None`): grass, flowers, small ground clutter.
  Client renders them; server ignores them entirely.
- **Convex hull** (`collision: ConvexHull`): the default for most props. Avian3d
  computes a convex hull from the mesh at load time. Fast, zero authoring effort,
  works well for rocks, crates, tents, trees. Low-poly meshes make this cheap.
- **Trimesh** (`collision: TrimeshFromMesh`): terrain and concave walkable
  geometry (archways, bridges, building interiors). More expensive but handles
  concave shapes correctly.

Dedicated hand-authored collision meshes are not needed given low-poly source
assets. If a specific prop causes issues (e.g., convex hull fills a gap that
players should walk through), switch it to trimesh.

## Instancing

Bevy instances mesh data automatically. Spawning 200 entities that reference
`world/props/pine_tree.glb` loads the mesh once and shares it. The zone RON file
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
