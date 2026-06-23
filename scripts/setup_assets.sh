#!/bin/bash
# Sets up asset symlinks after git submodule initialization.
# Both client and server get the full world/ directory.
# The server skips textures at runtime (load_materials = empty()).
# For production, exclude textures from the server Docker image via Dockerfile.

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS_DIR="$REPO_ROOT/assets"
CLIENT_ASSETS="$REPO_ROOT/crates/game-client/assets"
SERVER_ASSETS="$REPO_ROOT/crates/game-server/assets"

# Verify submodule is initialized
if [ ! -d "$ASSETS_DIR/world" ]; then
    echo "Assets submodule not initialized. Run: git submodule update --init"
    exit 1
fi

echo "Setting up asset symlinks..."

# --- Client ---
if [ -e "$CLIENT_ASSETS/world" ]; then
    rm -rf "$CLIENT_ASSETS/world"
fi
ln -s ../../../assets/world "$CLIENT_ASSETS/world"
echo "  Client: world -> assets/world"

# --- Server ---
if [ -e "$SERVER_ASSETS/world" ]; then
    rm -rf "$SERVER_ASSETS/world"
fi
ln -s ../../../assets/world "$SERVER_ASSETS/world"
echo "  Server: world -> assets/world"

echo "Done."
