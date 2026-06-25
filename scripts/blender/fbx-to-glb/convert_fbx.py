import bpy
import sys
import os
import struct
import json

argv = sys.argv
argv = argv[argv.index("--") + 1:]

fbx_filepath = argv[0]
glb_filepath = argv[1]

# Reset Blender scene
bpy.ops.wm.read_factory_settings(use_empty=True)

# Import .fbx
bpy.ops.import_scene.fbx(filepath=fbx_filepath)

# Resolve missing textures by searching the FBX directory and its parent
# (common for asset packs with FBX/ + Textures/ as siblings).
# If a texture can't be found by its original basename, fall back to the
# pack's main texture atlas (many Synty props UV-map to a shared atlas).
search_dirs = [os.path.dirname(fbx_filepath)]
fbx_parent = os.path.dirname(search_dirs[0])
if fbx_parent != search_dirs[0]:
    search_dirs.append(fbx_parent)

# Detect the main texture atlas (largest PNG in the Textures root)
fallback_texture = None
textures_dir = os.path.join(fbx_parent, "Textures")
if os.path.isdir(textures_dir):
    best_size = 0
    for f in os.listdir(textures_dir):
        if not f.lower().endswith(".png"):
            continue
        fp = os.path.join(textures_dir, f)
        sz = os.path.getsize(fp)
        if sz > best_size:
            best_size = sz
            fallback_texture = fp

for img in bpy.data.images:
    if img.packed_file is not None:
        continue
    if not img.filepath:
        continue
    if os.path.isfile(bpy.path.abspath(img.filepath)):
        continue
    # Extract basename, handling embedded Windows paths in FBX references
    raw_path = img.filepath.replace("\\", "/")
    basename = raw_path.rsplit("/", 1)[-1].lower()
    found = False
    for sd in search_dirs:
        if found:
            break
        for root, dirs, files in os.walk(sd):
            for f in files:
                if f.lower() == basename:
                    img.filepath = os.path.join(root, f)
                    img.reload()
                    found = True
                    break
            if found:
                break
    if not found and fallback_texture:
        print(f"INFO: Texture '{basename}' not found, using atlas fallback")
        img.filepath = fallback_texture
        img.reload()

# Strip all texture data from materials before export.
# Textures are loaded at runtime from config (props.ron) via the AssetServer,
# which deduplicates by path. Embedding textures in GLBs would cause VRAM
# duplication since many models share the same atlas/grass textures.
# We keep the material node graph structure (preserving alpha_mode, doubleSided)
# but remove the actual image data.
for img in list(bpy.data.images):
    bpy.data.images.remove(img)

# Preserve volume on armature modifiers
for obj in bpy.context.scene.objects:
    if obj.type == 'MESH':
        for mod in obj.modifiers:
            if mod.type == 'ARMATURE':
                mod.use_deform_preserve_volume = True

# Export .glb
bpy.ops.export_scene.gltf(
    filepath=glb_filepath,
    export_materials="EXPORT",
)

# Post-process: drop uniform COLOR attributes from the exported GLB.
# Blender's FBX importer often creates a default all-white vertex color
# attribute that gets exported as COLOR_0, pushing the actual FBX vertex
# colors to COLOR_1. Bevy reads COLOR_0, so we need to fix this.
def _drop_uniform_colors(path):
    with open(path, "rb") as f:
        magic = f.read(4)
        if magic != b"glTF":
            return
        version = struct.unpack("<I", f.read(4))[0]
        length = struct.unpack("<I", f.read(4))[0]

        json_len = struct.unpack("<I", f.read(4))[0]
        json_type = f.read(4)
        json_bytes = f.read(json_len)

        remainder = f.read()

    gltf = json.loads(json_bytes.decode("utf-8"))

    changed = False
    for mesh in gltf.get("meshes", []):
        for prim in mesh.get("primitives", []):
            attrs = prim.get("attributes", {})
            # Find all COLOR_N keys sorted by index
            color_keys = sorted(
                (k for k in attrs if k.startswith("COLOR_")),
                key=lambda k: int(k.split("_")[1]),
            )
            if len(color_keys) < 2:
                continue

            # Check each COLOR attribute; read a few values to detect
            # uniform (constant) data.
            uniform_keys = []
            for ck in color_keys:
                acc = gltf["accessors"][attrs[ck]]
                bv = gltf["bufferViews"][acc["bufferView"]]
                ct = acc["componentType"]
                count = acc["count"]
                if count == 0:
                    continue

                if ct == 5121:
                    bpc, fmt = 1, "B"
                elif ct == 5123:
                    bpc, fmt = 2, "H"
                elif ct == 5126:
                    bpc, fmt = 4, "f"
                else:
                    continue

                stride = bv.get("byteStride", bpc * 4)
                # Binary chunk starts at offset 12 (header) + 8 (json chunk header) + json_len
                bin_offset = 12 + 8 + json_len + 8
                base = bin_offset + bv.get("byteOffset", 0) + acc.get("byteOffset", 0)

                with open(path, "rb") as bf:
                    bf.seek(base)
                    first_data = bf.read(stride)

                is_uniform = True
                n = min(count, 200)
                with open(path, "rb") as bf:
                    for i in range(n):
                        bf.seek(base + i * stride)
                        cur = bf.read(bpc * 4)
                        if cur != first_data[:bpc * 4]:
                            is_uniform = False
                            break

                if is_uniform:
                    uniform_keys.append(ck)

            # Only drop if we keep at least one
            keep = [k for k in color_keys if k not in uniform_keys]
            if not keep or len(keep) == len(color_keys):
                continue

            # Remove uniform keys and renumber the rest as COLOR_0, COLOR_1, ...
            for uk in uniform_keys:
                del attrs[uk]
            for i, k in enumerate(sorted(
                (k for k in attrs if k.startswith("COLOR_")),
                key=lambda k: int(k.split("_")[1]),
            )):
                new_key = f"COLOR_{i}"
                if k != new_key:
                    attrs[new_key] = attrs.pop(k)
            changed = True

    if not changed:
        return

    new_json = json.dumps(gltf, separators=(",", ":")).encode("utf-8")
    # Pad to 4-byte alignment
    while len(new_json) % 4 != 0:
        new_json += b" "

    with open(path, "wb") as f:
        f.write(b"glTF")
        total = 12 + 8 + len(new_json) + len(remainder)
        f.write(struct.pack("<I", version))
        f.write(struct.pack("<I", total))
        f.write(struct.pack("<I", len(new_json)))
        f.write(json_type)
        f.write(new_json)
        f.write(remainder)

_drop_uniform_colors(glb_filepath)
