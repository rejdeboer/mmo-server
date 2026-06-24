import bpy
import sys
import os

argv = sys.argv
argv = argv[argv.index("--") + 1:]

fbx_filepath = argv[0]
glb_filepath = argv[1]
texture_dir = argv[2] if len(argv) > 2 and argv[2] else None

# Reset Blender scene
bpy.ops.wm.read_factory_settings(use_empty=True)

# Import .fbx
bpy.ops.import_scene.fbx(filepath=fbx_filepath)

# Resolve missing textures by searching the texture directory (or FBX directory)
search_dir = texture_dir or os.path.dirname(fbx_filepath)
for img in bpy.data.images:
    if not img.filepath:
        continue
    if os.path.isfile(bpy.path.abspath(img.filepath)):
        continue
    basename = os.path.basename(img.filepath).lower()
    for root, dirs, files in os.walk(search_dir):
        for f in files:
            if f.lower() == basename:
                img.filepath = os.path.join(root, f)
                img.reload()
                break
        else:
            continue
        break

# Pack all images so they get embedded in the GLB
bpy.ops.file.pack_all()

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
