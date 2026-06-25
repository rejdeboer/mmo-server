"""
Blender Zone Export Script

Exports a zone .blend file into:
  - A terrain GLB (from the 'Terrain' collection)
  - A zone RON file (prop placements, spawn points, player spawn)

Usage:
    blender zones/meadow.blend --background \
        --python tools/blender_export_zone.py -- \
        --output assets/world/

The script expects:
  - A 'Terrain' collection containing the terrain mesh(es)
  - Linked collection instances for props (auto-resolved from props directory)
  - Empties with custom property type="spawner" for mob spawners
  - An empty with custom property type="player_spawn" for the spawn point

Prop resolution: The script scans assets/world/props/ for all .glb files and
builds a lookup from filename stem to asset-relative path. Collection instance
names are matched against these stems. If two GLBs share a stem name across
different packs, the script will error and require manual disambiguation.
"""

import argparse
import re
import sys
from pathlib import Path

import bpy


# ---------------------------------------------------------------------------
# NAMING CONVENTIONS
#
# Blender uses PascalCase for .blend files (e.g. StartingMeadow.blend).
# Rust/RON files use kebab-case (e.g. starting-meadow.zone.ron).
# The terrain GLB also uses kebab-case since it's our own export.
# Prop GLBs keep their original names (third-party assets, unchanged).
# ---------------------------------------------------------------------------


def to_kebab_case(name: str) -> str:
    """
    Convert PascalCase, camelCase, or snake_case to kebab-case.

    Examples:
        StartingMeadow  -> starting-meadow
        Meadow          -> meadow
        starting_meadow -> starting-meadow
        HTTPServer      -> http-server
    """
    # Insert hyphen before uppercase letters that follow a lowercase letter or digit
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", name)
    # Insert hyphen between consecutive uppercase letters followed by lowercase
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1-\2", s)
    # Replace underscores and spaces with hyphens
    s = s.replace("_", "-").replace(" ", "-")
    # Collapse multiple hyphens and lowercase
    s = re.sub(r"-+", "-", s).strip("-").lower()
    return s

# ---------------------------------------------------------------------------
# PROP PATH RESOLUTION
# ---------------------------------------------------------------------------


def build_prop_index(props_dir: Path) -> dict[str, str]:
    """
    Scan the props directory for all .glb files and build a lookup from
    filename stem to asset-relative path (relative to the assets/ root).

    Example: SM_Env_Tree_Birch_01 -> world/props/meadow/Models/SM_Env_Tree_Birch_01.glb
    """
    index: dict[str, str] = {}
    duplicates: dict[str, list[str]] = {}

    if not props_dir.exists():
        print(f"WARNING: Props directory not found: {props_dir}")
        return index

    # props_dir is e.g. /path/to/assets/world/props
    # We want paths relative to the assets/ dir, e.g. "world/props/meadow/Models/foo.glb"
    assets_root = props_dir.parent.parent  # assets/world/props -> assets/

    for glb_path in props_dir.rglob("*.glb"):
        stem = glb_path.stem
        relative = glb_path.relative_to(assets_root)
        asset_path = str(relative).replace("\\", "/")  # normalize for Windows

        if stem in index:
            if stem not in duplicates:
                duplicates[stem] = [index[stem]]
            duplicates[stem].append(asset_path)
        else:
            index[stem] = asset_path

    if duplicates:
        print("ERROR: Duplicate GLB stems found across packs:")
        for stem, paths in duplicates.items():
            print(f"  {stem}:")
            for p in paths:
                print(f"    - {p}")
        print("Resolve by renaming or removing duplicates.")
        sys.exit(1)

    print(f"Indexed {len(index)} prop GLBs from {props_dir}")
    return index


# ---------------------------------------------------------------------------
# COORDINATE CONVERSION
#
# Blender uses Z-up right-handed coordinates:  X=right, Y=forward, Z=up
# glTF/Bevy uses Y-up right-handed coordinates: X=right, Y=up,      Z=back
#
# Conversion:
#   glTF X =  Blender X
#   glTF Y =  Blender Z
#   glTF Z = -Blender Y
#
# The glTF exporter applies this to mesh data inside GLBs automatically,
# but we must apply it manually to the world transforms written to RON.
# ---------------------------------------------------------------------------


def format_f32(value: float) -> str:
    """Format a float for RON output."""
    if value == int(value):
        return f"{value:.1f}"
    return f"{value:.6f}".rstrip("0").rstrip(".")


def vec3_blender_to_ron(vec) -> str:
    """Convert a Blender Z-up Vector to Y-up RON tuple."""
    return f"({format_f32(vec.x)}, {format_f32(vec.z)}, {format_f32(-vec.y)})"


def quat_blender_to_ron(quat) -> str:
    """Convert a Blender Z-up quaternion to Y-up RON tuple [x, y, z, w]."""
    return (
        f"({format_f32(quat.x)}, {format_f32(quat.z)}, "
        f"{format_f32(-quat.y)}, {format_f32(quat.w)})"
    )


def scale_blender_to_ron(scale) -> str:
    """Convert a Blender Z-up scale to Y-up RON tuple."""
    return f"({format_f32(scale.x)}, {format_f32(scale.z)}, {format_f32(scale.y)})"


def prop_instance_to_ron(asset: str, obj, indent: str = "        ") -> str:
    """Serialize a prop instance to RON format."""
    loc = obj.matrix_world.to_translation()
    rot = obj.matrix_world.to_quaternion()
    scale = obj.matrix_world.to_scale()

    lines = [
        f"{indent}(",
        f"{indent}    asset: \"{asset}\",",
        f"{indent}    translation: {vec3_blender_to_ron(loc)},",
        f"{indent}    rotation: {quat_blender_to_ron(rot)},",
        f"{indent}    scale: {scale_blender_to_ron(scale)},",
        f"{indent}),",
    ]
    return "\n".join(lines)


def spawn_point_to_ron(obj, indent: str = "        ") -> str:
    """Serialize a spawn point empty to RON format."""
    loc = obj.matrix_world.to_translation()

    spawn_id = obj.get("id", obj.name)
    monster_id = obj.get("monster_id", "unknown")
    max_count = int(obj.get("max_count", 5))
    radius = float(obj.get("radius", 20.0))
    level_min = int(obj.get("level_min", 1))
    level_max = int(obj.get("level_max", 3))
    respawn_secs = float(obj.get("respawn_secs", 5.0))

    lines = [
        f"{indent}(",
        f"{indent}    id: \"{spawn_id}\",",
        f"{indent}    position: {vec3_blender_to_ron(loc)},",
        f"{indent}    radius: {format_f32(radius)},",
        f"{indent}    monster_id: \"{monster_id}\",",
        f"{indent}    max_count: {max_count},",
        f"{indent}    level_range: ({level_min}, {level_max}),",
        f"{indent}    respawn_secs: {format_f32(respawn_secs)},",
        f"{indent}),",
    ]
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# TERRAIN EXPORT
# ---------------------------------------------------------------------------


def export_terrain(output_dir: Path, zone_name: str) -> str:
    """Export the Terrain collection as a GLB file. Returns the asset-relative path."""
    terrain_collection = bpy.data.collections.get("Terrain")
    if terrain_collection is None:
        print("WARNING: No 'Terrain' collection found. Skipping terrain export.")
        return ""

    # Select only terrain objects
    bpy.ops.object.select_all(action="DESELECT")
    terrain_objects = []
    for obj in terrain_collection.all_objects:
        if obj.type == "MESH":
            obj.select_set(True)
            terrain_objects.append(obj)

    if not terrain_objects:
        print("WARNING: Terrain collection has no mesh objects.")
        return ""

    terrain_dir = output_dir / "terrain"
    terrain_dir.mkdir(parents=True, exist_ok=True)

    glb_path = terrain_dir / f"{zone_name}.glb"
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path),
        use_selection=True,
        export_format="GLB",
        export_apply=True,
        export_cameras=False,
        export_lights=False,
        export_animations=False,
    )

    print(f"Exported terrain: {glb_path}")
    return f"world/terrain/{zone_name}.glb"


# ---------------------------------------------------------------------------
# PROP COLLECTION
# ---------------------------------------------------------------------------


def collect_props(prop_index: dict[str, str]) -> list[tuple[str, object, str]]:
    """
    Find all collection instances in the scene and resolve their prop paths
    using the auto-generated prop index.
    Returns list of (asset_path, blender_object).
    """
    props = []

    for obj in bpy.context.scene.objects:
        if obj.instance_type != "COLLECTION" or obj.instance_collection is None:
            continue

        collection_name = obj.instance_collection.name

        if collection_name == "Terrain":
            continue

        asset_path = prop_index.get(collection_name)
        if asset_path is None:
            print(f"WARNING: No GLB found for collection '{collection_name}' "
                  f"(object '{obj.name}'). Skipping.")
            continue

        props.append((asset_path, obj))

    return props


# ---------------------------------------------------------------------------
# SPAWNER / PLAYER SPAWN COLLECTION
# ---------------------------------------------------------------------------


def collect_spawners() -> list[object]:
    """Find all empties with custom property type='spawner'."""
    spawners = []
    for obj in bpy.context.scene.objects:
        if obj.type == "EMPTY" and obj.get("type") == "spawner":
            spawners.append(obj)
    return spawners


def find_player_spawn():
    """Find the empty with custom property type='player_spawn'."""
    for obj in bpy.context.scene.objects:
        if obj.type == "EMPTY" and obj.get("type") == "player_spawn":
            return obj
    return None


# ---------------------------------------------------------------------------
# MAIN
# ---------------------------------------------------------------------------


def main():
    # Parse arguments after '--'
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []

    parser = argparse.ArgumentParser(description="Export Blender zone to terrain GLB + zone RON")
    parser.add_argument(
        "--output",
        required=True,
        help="Output directory for assets (e.g. assets/world/)",
    )
    parser.add_argument(
        "--zone-name",
        default=None,
        help="Zone name override. Defaults to the .blend filename without extension.",
    )
    parser.add_argument(
        "--skip-terrain",
        action="store_true",
        help="Skip terrain GLB export (useful if terrain hasn't changed).",
    )
    parser.add_argument(
        "--no-validate",
        action="store_true",
        help="Skip validation that referenced prop GLBs exist on disk.",
    )
    args = parser.parse_args(argv)

    output_dir = Path(args.output).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    # Determine zone name from the blend file, converted to kebab-case
    blend_path = Path(bpy.data.filepath)
    zone_name = args.zone_name or to_kebab_case(blend_path.stem)

    print(f"=== Exporting zone: {zone_name} ===")
    print(f"Output directory: {output_dir}")

    # Build prop index by scanning the props directory
    props_dir = output_dir / "props"
    prop_index = build_prop_index(props_dir)

    # Export terrain
    terrain_asset_path = ""
    if not args.skip_terrain:
        terrain_asset_path = export_terrain(output_dir, zone_name)
    else:
        terrain_asset_path = f"world/terrain/{zone_name}.glb"
        print(f"Skipping terrain export (using path: {terrain_asset_path})")

    # Collect props
    props = collect_props(prop_index)
    print(f"Found {len(props)} prop instance(s)")

    # Validate
    if not args.no_validate and props:
        assets_root = output_dir.parent
        valid = True
        seen_paths: set[str] = set()
        for asset_path, obj in props:
            if asset_path in seen_paths:
                continue
            seen_paths.add(asset_path)
            full_path = assets_root / asset_path
            if not full_path.exists():
                print(f"ERROR: Referenced GLB not found: {full_path} "
                      f"(from '{obj.name}')")
                valid = False
        if not valid:
            print("ERROR: Validation failed. Fix missing assets and re-run.")
            sys.exit(1)

    # Collect spawners
    spawners = collect_spawners()
    print(f"Found {len(spawners)} spawner(s)")

    # Find player spawn
    player_spawn_obj = find_player_spawn()
    if player_spawn_obj:
        player_spawn = player_spawn_obj.matrix_world.to_translation()
        print(f"Player spawn: ({player_spawn.x}, {player_spawn.y}, {player_spawn.z})")
    else:
        print("WARNING: No player_spawn empty found. Defaulting to origin.")
        import mathutils
        player_spawn = mathutils.Vector((0.0, 5.0, 0.0))

    # Build RON output
    ron_lines = [
        "(",
        f"    id: \"{zone_name}\",",
        f"    terrain: \"{terrain_asset_path}\",",
        f"    player_spawn: {vec3_blender_to_ron(player_spawn)},",
        "    props: [",
    ]

    for asset_path, obj in props:
        ron_lines.append(prop_instance_to_ron(asset_path, obj))

    ron_lines.append("    ],")
    ron_lines.append("    spawn_points: [")

    for spawner_obj in spawners:
        ron_lines.append(spawn_point_to_ron(spawner_obj))

    ron_lines.append("    ],")
    ron_lines.append(")")
    ron_lines.append("")  # trailing newline

    # Write zone RON
    zones_dir = output_dir / "zones"
    zones_dir.mkdir(parents=True, exist_ok=True)
    ron_path = zones_dir / f"{zone_name}.zone.ron"
    ron_path.write_text("\n".join(ron_lines), encoding="utf-8")
    print(f"Written zone RON: {ron_path}")

    print(f"=== Done: {zone_name} ===")
    print(f"  Terrain: {terrain_asset_path}")
    print(f"  Props: {len(props)}")
    print(f"  Spawners: {len(spawners)}")


if __name__ == "__main__":
    main()
