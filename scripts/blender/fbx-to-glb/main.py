import argparse
import subprocess
import os
import shutil
import sys


def find_blender():
    blender = shutil.which("blender")
    if blender:
        return blender

    # Common macOS location
    mac_path = "/Applications/Blender.app/Contents/MacOS/Blender"
    if os.path.isfile(mac_path):
        return mac_path

    return None


def main():
    parser = argparse.ArgumentParser(description="Batch convert FBX files to GLB using Blender")
    parser.add_argument("input_dir", help="Directory containing .fbx files")
    parser.add_argument("output_dir", help="Directory to write .glb files to")
    parser.add_argument("texture_dir", nargs="?", default=None, help="Directory to search for missing textures")
    parser.add_argument("--blender", help="Path to the Blender executable", default=None)
    args = parser.parse_args()

    input_dir = os.path.abspath(args.input_dir)
    output_dir = os.path.abspath(args.output_dir)
    texture_dir = os.path.abspath(args.texture_dir) if args.texture_dir else None

    if not os.path.isdir(input_dir):
        print(f"Error: input directory does not exist: {input_dir}", file=sys.stderr)
        sys.exit(1)

    if texture_dir and not os.path.isdir(texture_dir):
        print(f"Error: texture directory does not exist: {texture_dir}", file=sys.stderr)
        sys.exit(1)

    blender = args.blender or find_blender()
    if not blender:
        print("Error: could not find Blender. Install it or pass --blender <path>", file=sys.stderr)
        sys.exit(1)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    convert_script = os.path.join(script_dir, "convert_script.py")

    os.makedirs(output_dir, exist_ok=True)

    converted = 0
    failed = 0

    for root, dirs, files in os.walk(input_dir):
        for file in files:
            if not file.lower().endswith(".fbx"):
                continue

            fbx_file = os.path.join(root, file)
            rel_path = os.path.relpath(root, input_dir)
            glb_name = f"{os.path.splitext(file)[0]}.glb"
            glb_dir = os.path.join(output_dir, rel_path)
            glb_file = os.path.join(glb_dir, glb_name)

            os.makedirs(glb_dir, exist_ok=True)

            print(f"Converting: {os.path.relpath(fbx_file, input_dir)}")
            try:
                subprocess.run(
                    [
                        blender,
                        "--background",
                        "--python", convert_script,
                        "--",
                        fbx_file,
                        glb_file,
                        texture_dir or "",
                    ],
                    check=True,
                    capture_output=True,
                )
                converted += 1
            except subprocess.CalledProcessError as e:
                print(f"  FAILED: {e}", file=sys.stderr)
                if e.stderr:
                    print(f"  stderr: {e.stderr.decode()}", file=sys.stderr)
                failed += 1

    print(f"\nDone. Converted: {converted}, Failed: {failed}")


if __name__ == "__main__":
    main()
