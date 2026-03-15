# Bevy MMORPG Project

This is a modern MMORPG monorepo using Rust and Bevy.

## Project Structure

The `crates/` directory contains the following crates:

- `game-server/` - A Bevy MMO game server
- `game-client/` - The Bevy client used by players to play the game
- `game-core/` - Rust code shared by the `game-client` and `game-server`
- `protocol/` - Bitcode structs used for communication between servers and clients
- `web-server/` - Axum server for auth, account / character management and in-game social features like guild / party chat and invites implemented using WebSockets
- `web-client/` - Client that talks to the web-server
