# TODO

## Quick Wins

- [ ] Add protocol serialization roundtrip tests using quickcheck (protocol crate)
- [ ] Fix `resolve_recipient_id` — handle `RowNotFound` separately from DB errors
- [ ] Remove `get_client_unchecked` / `get_guild_members_unchecked` panics in hub.rs
- [ ] Add underflow guard for `target_vitals.hp -= spell.damage`
- [ ] Update stale dependencies (base64 0.12 -> 0.22, standardize fake/serde-aux versions)

## Platform Engineering

- [ ] Graceful shutdown with WebSocket connection draining on SIGTERM
- [ ] Distributed tracing — propagate trace IDs through NATS messages (OpenTelemetry)
- [ ] Blue/green game server deploys — migrate players between instances without disconnect

## Distributed Systems

- [ ] Persistent chat history — async writes to Postgres, handle ordering with NATS delivery (should probably be done with analytics database)
- [ ] Presence system — track online players across web-server instances (NATS KV or Redis)
- [ ] Party system — ephemeral groups with distributed state and disconnect cleanup
- [ ] Message ordering guarantees — sequence numbers and client-side reordering

## Game Development

- [ ] Spell cooldowns and global cooldown (server-authoritative)
- [ ] Weapon speed-based auto-attack timing (replace constant swing speed)
- [ ] Weapon damage stats (replace constant auto-attack damage)
- [ ] Loot and inventory system — item definitions, slots, equip, DB persistence
- [ ] Zone transitions — handoff between game-server instances
- [ ] Death and respawn — corpse runs, respawn timers, XP loss

## Networking

- [ ] Client-side prediction and server reconciliation (input prediction with rollback)
- [ ] Interest management priority — closer entities update more frequently
- [ ] Bandwidth budgeting — cap outbound bytes/tick/client, prioritize important updates

## Game Client

- [ ] Show accept/decline UI for party invites (`game-client/src/party/mod.rs`)
- [ ] Proper logout — transition to character select and disconnect cleanly instead of `std::process::exit(0)` (`game-client/src/world/player_frame.rs`)
- [ ] Open options/escape menu when pressing Escape with no active auto-attack (`game-client/src/combat/auto_attack.rs`)
- [ ] Show loot notification UI on kill rewards (`game-client/src/networking/receive.rs`)

## Refactoring
- Game client is currently pretty hacky. many functions have too much responsibility. Should clean it up using bevy messages / events.

