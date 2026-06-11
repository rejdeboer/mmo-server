# TODO

## Quick Wins

- [ ] Replace `entity.to_bits()` with a proper network ID system (game-server, game-core)
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
- [ ] NPC AI state machine — threat/aggro tables, leashing, basic pathing
- [ ] Loot and inventory system — item definitions, slots, equip, DB persistence
- [ ] Zone transitions — handoff between game-server instances
- [ ] Death and respawn — corpse runs, respawn timers, XP loss

## Networking

- [ ] Client-side prediction and server reconciliation (input prediction with rollback)
- [ ] Interest management priority — closer entities update more frequently
- [ ] Bandwidth budgeting — cap outbound bytes/tick/client, prioritize important updates
