# Combat System Design

## Overview

The combat system handles melee auto-attacks (white hits) as a server-authoritative system separate from the spell/ability pipeline. Players toggle auto-attack on a target, and the server manages a swing timer that fires hits automatically.

### Principles

1. **Server-authoritative** — the server owns the swing timer and validates all attacks. Clients cannot fake hits.
2. **Separate from spells** — auto-attack runs independently of the spell cast pipeline. Both can operate simultaneously.
3. **Toggle model** — the client sends a single `StartAttack` action; the server handles repeated swings until stopped.
4. **Pause/resume** — when out of melee range, the swing timer pauses. It resumes from where it left off when back in range.

## Architecture

```
Protocol (crates/protocol/src/)
├── client.rs       - PlayerAction::StartAttack, PlayerAction::StopAttack
└── server.rs       - AUTO_ATTACK_VISUAL_ID constant (sentinel for SpellImpact)

Server (crates/game-server/src/)
├── components.rs   - AutoAttack { target, swing_timer }
├── messages.rs     - StartAttackMessage, StopAttackMessage
├── systems/
│   └── combat.rs   - process_start_attack, process_stop_attack,
│                     tick_auto_attack, cancel_auto_attack_on_death
└── application.rs  - System registration

Client (crates/game-client/src/)
└── combat.rs       - send_attack_action (T key to toggle)
```

## Data Flow

```
Client                          Server
  │                               │
  ├─ PlayerAction::StartAttack ──►│
  │                               ├─ Validate target alive
  │                               ├─ Insert AutoAttack component
  │                               │
  │                               │  [FixedUpdate loop]
  │                               ├─ Check range
  │                               │   ├─ Out of range: pause timer
  │                               │   └─ In range: tick timer
  │                               │       └─ Timer finished:
  │                               │           ├─ Apply damage to Vitals
  │                               │           ├─ Tap target (first hit)
  │                               │           ├─ Broadcast SpellImpact
  │                               │           └─ Reset timer
  │◄─ SpellImpact (visual_id) ───┤
  │                               │
  ├─ PlayerAction::StopAttack ───►│
  │                               └─ Remove AutoAttack component
```

## Constants

| Constant | Value | Location |
|----------|-------|----------|
| `MELEE_RANGE` | 3.0 | `systems/combat.rs` |
| `AUTO_ATTACK_SPEED` | 2.0s | `systems/combat.rs` |
| `AUTO_ATTACK_DAMAGE` | 5 | `systems/combat.rs` |
| `AUTO_ATTACK_VISUAL_ID` | `u32::MAX` | `protocol/src/server.rs` |

## Server Systems

### `process_start_attack` (FixedPreUpdate)

Reads `StartAttackMessage` from the message bus. Validates:
- Attacker is alive (has `Vitals`, no `Dead`)
- Target is alive
- Not attacking self
- Not already attacking the same target

Inserts `AutoAttack` component with a pre-finished timer so the first swing fires immediately when in range.

### `process_stop_attack` (FixedPreUpdate)

Reads `StopAttackMessage`, removes the `AutoAttack` component.

### `tick_auto_attack` (FixedUpdate)

For each entity with `AutoAttack`:
1. If target query fails (dead/despawned): remove `AutoAttack`, skip
2. Compute distance to target
3. If out of `MELEE_RANGE`: skip (timer pauses)
4. If in range: tick timer
5. On timer finish: apply damage, tap target, broadcast `SpellImpact`

### `cancel_auto_attack_on_death` (FixedUpdate)

Removes `AutoAttack` from any entity that has the `Dead` component.

## Client Controls

- **T key**: Start attacking the selected target. If no target is selected, sends `StopAttack`.
- **Escape key**: Sends `StopAttack` to cancel auto-attack.

## Interaction with Existing Systems

- **Spell casting**: Auto-attack runs concurrently. Casting a spell does NOT cancel auto-attack.
- **AI mobs**: Continue using their spell-based melee attacks (e.g., "Skeleton Strike"). They do not use the `AutoAttack` system.
- **Death**: When the target dies, auto-attack is automatically cancelled. When the attacker dies, auto-attack is also removed.
- **Tapping**: First auto-attack hit on an un-tapped mob marks it as tapped by the player.
- **Threat**: Auto-attack damage flows through `Vitals` mutation, which is picked up by the existing threat system via `update_threat_on_damage`.

## Future Extensions

- **Weapon speed**: Replace `AUTO_ATTACK_SPEED` constant with a per-entity component sourced from equipped weapon stats.
- **Weapon damage**: Replace `AUTO_ATTACK_DAMAGE` with weapon-based damage calculation (base + stat modifiers).
- **Swing timer reset**: Special abilities (like Heroic Strike) that replace the next auto-attack swing.
- **GCD integration**: Add a shared global cooldown for special abilities (separate from auto-attack).
- **Mob auto-attacks**: Extend the system so mobs use `AutoAttack` for basic hits between special ability cooldowns.
- **Client feedback**: Swing timer bar UI, hit animations, floating combat text differentiating melee vs spell.
