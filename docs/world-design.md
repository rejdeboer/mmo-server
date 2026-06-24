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
   modified. The export script only references them by path in the zone RON.
4. **Server skips visuals, not files** -- the server loads the same GLB files as
   the client but skips materials and textures via
   `load_materials = RenderAssetUsages::empty()`. No separate "server-only"
   exports are needed.
5. **Data-driven spawning** -- monster spawners, player spawn points, and prop
   placements are defined in the zone RON file, not in Rust code.

## Asset Management

Licensed art assets live in a **private Git repository** tracked with **Git LFS**
for binary files. This repo is added to the main project as a **Git submodule**
at the `assets/` directory.

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
        meadow/                        # Paid asset pack, original structure preserved
          Models/
            SM_Env_Tree_Birch_01.glb
            SM_Prop_Camp_Tent_01.glb
            SM_Env_Rock_01.glb
            ...
          Textures/
            PolygonNatureBiomes_Meadow_Texture_01.png
            ...
      zones/
        starting_meadow.ron            # Zone definition (exported from Blender)
    sounds/
      whisper-received.ogg
      party-invite.ogg
    icons/
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
    props/
      SM_Env_Tree_Birch_01.blend         # Per-prop .blend for linked instancing
      SM_Prop_Camp_Tent_01.blend
      ...
    starting_meadow.blend                # Zone file (terrain + placed instances)

  tools/
    blender_export_zone.py               # Blender Python export script
```

### Asset Pack Organization

Paid asset packs are stored with their **original directory structure** preserved
under `assets/world/props/<pack-name>/`. GLB files contain internal relative
paths to their textures. Preserving the original layout ensures these references
resolve correctly without needing to patch GLB files.

If textures don't load in Bevy (magenta/pink materials), inspect the GLB's
internal texture references and adjust the directory layout to match:

```
npx @gltf-transform/cli inspect props/meadow/Models/SM_Env_Tree_Birch_01.glb
```

### Symlinks

Both `game-client` and `game-server` access shared assets through identical
symlinks pointing into the submodule (`world -> ../../../assets/world`). Both
get the full directory including textures.

The server does not need textures, but having them on disk costs nothing since
`load_materials = RenderAssetUsages::empty()` prevents them from being loaded at
runtime. Texture exclusion for production is handled by `.dockerignore` (see
Deployment section).

Git tracks symlinks natively on Unix. Run `scripts/setup_assets.sh` after
`git submodule update --init` to create them. On Windows, developer mode or
`mklink` is required.

## Blender Workflow

### Prop Setup (One-Time Per Prop)

Each paid GLB asset gets a small `.blend` file in `zones/props/` for use as a
linked collection:

1. Create a new `.blend` file named after the GLB (e.g.,
   `SM_Env_Tree_Birch_01.blend`).
2. Import the GLB (File > Import > glTF 2.0).
3. Place all imported objects into a collection named after the prop.
4. Save. This file is the source for linked instancing in zone files.

This is a one-time setup per prop. Once done, the prop can be instanced across
any number of zones.

### Zone File (.blend)

One `.blend` file per zone, stored in `zones/` (source files, not shipped). The
scene contains:

| Object Type | Blender Representation | Identification |
|-------------|----------------------|----------------|
| Terrain | Sculpted mesh | In `Terrain` collection |
| Prop | Linked collection instance | Collection name matches prop `.blend` name |
| Spawner | Empty | Custom property `type = "spawner"` + config properties |
| Player spawn | Empty | Custom property `type = "player_spawn"` |

#### Building a Zone

1. **Sculpt terrain** directly as a mesh in the zone file.
2. **Place props** by linking collections from their `.blend` files
   (File > Link > select the collection). Place instances with Shift+D. Blender
   stays performant because instanced data is shared, even with hundreds of
   trees.
3. **Add spawners** as Empties with custom properties:
   - `type = "spawner"`
   - `monster_id` (string, matches a key in `monsters.ron`)
   - `max_count` (int)
   - `radius` (float)
   - `level_min`, `level_max` (int)
   - `respawn_secs` (float)
4. **Mark the player spawn** as an Empty with custom property
   `type = "player_spawn"`.

### Export Script

`tools/blender_export_zone.py` runs from the command line:

```
blender zones/starting_meadow.blend --background \
  --python tools/blender_export_zone.py -- \
  --output assets/world/
```

The script:

1. Finds the terrain mesh (objects in the `Terrain` collection), exports it as
   `terrain/<zone_name>.glb`.
2. Iterates all collection instances. Uses the linked collection name to resolve
   the prop's GLB path. Collects their world-space transforms.
3. Iterates empties with `type = "spawner"`, collects spawn configuration from
   custom properties.
4. Finds the empty with `type = "player_spawn"`.
5. Writes `zones/<zone_name>.ron` with all collected data.
6. Validates that every referenced prop has a corresponding `.glb` in the props
   directory.

The script **does not export prop GLBs** -- paid assets ship as-is, untouched.
Only the terrain mesh is exported.

## Zone Definition Format

The zone RON file is the glue between Blender and the engine:

```ron
ZoneDef(
    id: "starting_meadow",
    terrain: "world/terrain/starting_meadow.glb",
    player_spawn: (0.0, 5.0, 0.0),
    props: [
        PropInstance(
            asset: "world/props/meadow/Models/SM_Env_Tree_Birch_01.glb",
            transform: (
                translation: (10.0, 0.0, -5.0),
                rotation: (0.0, 0.7071, 0.0, 0.7071),
                scale: (1.0, 1.0, 1.0),
            ),
            collision: ConvexHull,
        ),
        PropInstance(
            asset: "world/props/meadow/Models/SM_Env_Rock_01.glb",
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
visual mesh without dedicated collision geometry:

- **Decorative only** (`collision: None`): grass, flowers, small ground clutter.
  Client renders them; server ignores them entirely.
- **Convex hull** (`collision: ConvexHull`): the default for most props. Avian3d
  computes a convex hull from the mesh at load time. Works well for rocks, trees,
  crates, tents. Low-poly meshes make this cheap.
- **Trimesh** (`collision: TrimeshFromMesh`): for concave walkable geometry --
  archways, bridges, building interiors where a convex hull would fill in gaps
  players need to walk through.

Dedicated hand-authored collision meshes are not needed given low-poly source
assets.

## Instancing

Bevy instances mesh data automatically. Spawning 200 entities that reference the
same GLB loads the mesh data once and shares it. The zone RON file can reference
the same prop hundreds of times with different transforms at negligible memory
cost.

This matches the Blender workflow where linked collection instances share the
same underlying mesh data.

## Iteration Loop

1. Edit terrain or prop placement in Blender.
2. Run the export script (one command).
3. Restart the game server/client. With `AssetPlugin` file watching enabled,
   some changes can be picked up without a restart.

## Deployment

The game server is deployed as a Docker container. The client is distributed as
a native desktop binary (not containerized).

### Server Docker Image

Texture files (`.png`, `.jpg`, `.tga`, `.tif`) are excluded from the Docker build
context via `.dockerignore`. The server never loads textures at runtime
(`load_materials = RenderAssetUsages::empty()`), so excluding them reduces image
size with no functional impact.

### Client Distribution

The game client is a native Bevy/wgpu application. It is distributed as a
compiled binary bundled with the full asset directory (including textures). CI
cross-compiles for target platforms and uploads to the distribution channel
(Steam, itch.io, or direct download).

## Custom Shader Conversion

Paid asset packs often rely on custom shaders (Unity ShaderGraph, Unreal
materials) for their intended look. When a GLB is exported from these engines,
vertex colors, UVs, and mesh data are preserved, but the shader logic is lost.
The result in Bevy is typically a flat gray or magenta mesh. To restore the
intended appearance, the shader must be ported to a Bevy custom `Material` with
a WGSL shader.

### When You Need a Custom Shader

- Props render as flat/untextured despite having vertex colors and UVs.
- The original asset pack documents a custom shader (check its README or
  Unity/Unreal project files).
- Vertex color channels encode data for the shader (masks, wind weights) rather
  than display colors.

### Conversion Workflow

1. **Obtain the shader source.** For Unity, find the `.shader` or generated HLSL
   in the Library or Packages folder. For Unreal, screenshot the material graph
   or export the HLSL via the material editor.

2. **Identify the render passes.** Unity/Unreal shaders contain many passes
   (forward, shadow, depth, meta, etc.). Only the **forward/lit fragment
   function** and the **vertex function** matter for Bevy -- Bevy handles other
   passes automatically.

3. **Extract the core logic.** Strip away engine boilerplate and isolate:
   - **Uniforms/parameters**: color values, floats, toggles, textures.
   - **Vertex color semantics**: what each RGBA channel encodes.
   - **Fragment output**: how `BaseColor`, `Alpha`, `Metallic`, `Roughness`,
     `Normal`, and `Emission` are computed.
   - **Vertex displacement**: any world-space animation (wind, waves).

4. **Design the Bevy material struct.** Create a Rust struct implementing
   `Material` (from `bevy::pbr`) with the extracted uniforms as fields. Use
   `#[uniform]` for scalar/vector data and `#[texture]`/`#[sampler]` for
   textures.

5. **Write the WGSL shader.** Port the HLSL/GLSL logic to WGSL. Key differences:
   - WGSL uses `fn` syntax, `var<uniform>`, `@group/@binding` decorations.
   - Texture sampling: `textureSample(t, s, uv)` instead of `tex2D`.
   - No implicit type conversions -- cast explicitly (`f32()`, `vec3f()`).
   - Built-in functions differ: `lerp` -> `mix`, `saturate` -> `clamp(..., 0.0, 1.0)`,
     `frac` -> `fract`.

6. **Integrate with the asset pipeline.** The material must be applied at load
   time by replacing the default `StandardMaterial` on entities that use the
   custom shader. Use a marker (prop path pattern, mesh name, or a tag in the
   zone RON) to identify which entities need it.

7. **Iterate visually.** Use Bevy's hot-reloading (`watch_for_changes`) to tweak
   uniform values and shader code without restarting.

### Vertex Color Conventions

Many stylized asset packs encode shader data in vertex colors rather than display
colors. Document the channel mapping for each pack:

| Channel | Common Uses |
|---------|------------|
| R | Wind bend weight, AO, blend mask |
| G | Secondary animation mask, gradient, height |
| B | Category mask (e.g., leaf vs trunk), detail blend |
| A | Opacity mask, snow/moss coverage |

Always inspect vertex colors in Blender (Vertex Paint mode) to verify what data
is present before assuming the GLB is broken.

### Example: Synty Foliage Shader

The Synty "PolygonNatureBiomes" pack uses a custom foliage shader. GLBs contain
vertex colors but render flat in Bevy because colors come from shader parameters
and world-space noise, not from vertex color display.

**Vertex color semantics:**
- **R**: Wind bend weight (0 = fixed root, 1 = full sway)
- **G**: Leaf wind fade mask
- **B**: Category discriminator (>0.5 = leaf, <=0.5 = trunk)

**Fragment logic (simplified):**
```
if vertex_color.b > 0.5:
    // Leaf path
    color = mix(leaf_noise_color, leaf_base_color, noise(world_xz * small_freq))
    color = mix(color, leaf_large_noise_color, noise(world_xz * large_freq))
    alpha = sample(leaf_texture).a
else:
    // Trunk path
    color = mix(trunk_noise_color, trunk_base_color, noise(world_xz * small_freq))
    alpha = sample(trunk_texture).a

clip(alpha - alpha_threshold)
```

**Required uniforms:**
- `leaf_base_color`, `leaf_noise_color`, `leaf_large_noise_color`: `Color`
- `trunk_base_color`, `trunk_noise_color`: `Color`
- `color_noise_small_freq`, `color_noise_large_freq`: `f32`
- `alpha_clip_threshold`: `f32`
- `leaf_texture`, `trunk_texture`: optional `Handle<Image>`

**Vertex displacement (wind, optional polish):**
```
bend = vertex_color.r * wind_strength * sin(time + world_pos.x * freq)
position.xz += bend * wind_direction
```

**Implementation priority:**
1. Static coloring (fragment logic + noise) -- required for correct appearance.
2. Alpha clipping -- required for leaves to look like leaves.
3. Wind animation -- visual polish, can ship without.

### Shader File Organization

```
crates/game-client/
  assets/
    shaders/
      foliage.wgsl              # WGSL shader source
  src/
    materials/
      mod.rs                    # MaterialPlugin registration
      foliage.rs                # FoliageMaterial struct + Material impl
```

Register materials in the client's plugin setup. The server never loads shaders
(it skips materials entirely).

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

### Additional Asset Packs

When adding a new paid asset pack:

1. Add the pack under `assets/world/props/<pack-name>/` preserving its original
   directory structure.
2. Create per-prop `.blend` files in `zones/props/` for linked instancing.
3. No changes needed to `setup_assets.sh`, the Dockerfile, or the zone loading
   code -- new props are picked up automatically via the zone RON file.
