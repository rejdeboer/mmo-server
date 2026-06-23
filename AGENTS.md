# Bevy MMORPG Project

This is a modern MMORPG monorepo using Rust and Bevy.

## Core Technologies

- Bevy: 0.18.1 (CRITICAL: Do not use Bevy 0.17 or older APIs. Ensure modern State and Plugin syntax is used)
- Axum web server
- Bitcode serialization
- Avian3d physics
- Deployed on Kubernetes

## Project Structure

The `crates/` directory contains the following crates:

- `game-server/` - A Bevy MMO game server
- `game-client/` - The Bevy client used by players to play the game
- `game-core/` - Rust code shared by the `game-client` and `game-server`
- `protocol/` - Bitcode structs used for communication between servers and clients
- `web-server/` - Axum server for auth, account / character management and in-game social features like guild / party chat and invites implemented using WebSockets
- `web-client/` - Client that talks to the web-server
- `web-types/` - Shared types for the web API layer (used by `web-server` and `web-client`)
- `provisioner/` - Database provisioning and seeding tool
- `simulator/` - Load/stress testing simulator for the game server

## Networking & Protocol
- **No Serde for Game Loop:** Use `bitcode` for all `protocol/` structs sent between `game-client` and `game-server` to save bandwidth.
- **Shared State:** If a Component needs to exist on both the client and server, define it in `game-core`.
- When updating the `protocol`, ensure you update the serialization/deserialization on *both* the `game-server` and `game-client`.

## Database
- **PostgreSQL 17** with **SQLx** for compile-time checked queries.
- Migrations live in `db/migrations/`. Run them via `scripts/init_db.sh` or `sqlx migrate run`.
- The `.sqlx/` directory contains offline query metadata for CI builds (`SQLX_OFFLINE=true`). After changing any query, run `cargo sqlx prepare --workspace` to update it.
- The `provisioner` crate handles database seeding for development and testing.

## NATS
- **async-nats** is used for inter-service messaging between `game-server` and `web-server`.
- NATS runs locally via `docker-compose.yml`.

## Rust Best Practices
- Keep imports clean. Group Bevy imports together (`use bevy::prelude::*;`).
- Use `clippy` to ensure idiomatic Rust. Handle all `Result` types properly; do not use `.unwrap()` in production server code unless absolute certainty exists.

## Bevy Best Practices
- Recent versions of Bevy use `Message` instead of `Event`. Keep this into account when writing event-driven code.
- Try to use Bevy observers where applicable, look at the game-server crate for examples.

## Assets

- Licensed art assets live in a **private Git repo with Git LFS**, added as a Git submodule at `assets/`.
- Both `game-client` and `game-server` access shared assets via **symlinks** into the submodule (e.g., `crates/game-client/assets/world -> ../../../assets/world`).
- The world is authored in **Blender** and exported via a Python script to GLB + RON files.
- The server loads the same GLB files as the client but skips materials (`load_materials = RenderAssetUsages::empty()`). There are no separate server-only asset exports.
- See `docs/world-design.md` for the full world creation workflow.

## TODO

For an overview of current todos check out `TODO.md`

## Deployment

This project is deployed using another repo containing Kubernetes manifests, Flux CD, Proxmox, LGTM. This can be found at `../mmo-deployment`

## Documentation

Only add comments whenever it adds value and proper explanation about what's being done. Avoid obvious or comments for self-describing code.
