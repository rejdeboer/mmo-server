# Simulator Design

## Purpose

The simulator generates realistic traffic patterns against the deployed MMO infrastructure.
Its primary use case is providing input for the monitoring stack (Prometheus, Grafana)
and validating platform behaviour under load (autoscaling, NATS throughput, connection limits).

## Architecture

```
simulator/
├── src/
│   ├── main.rs              # CLI entry point, scenario selection
│   ├── lib.rs
│   ├── net.rs               # Low-level GameClient (renet/netcode)
│   ├── scenarios/
│   │   ├── mod.rs
│   │   ├── movement.rs      # Current bot movement logic
│   │   ├── guild_chat.rs    # Guild chat flood
│   │   ├── whisper.rs       # Cross-instance whisper pairs
│   │   ├── churn.rs         # Rapid connect/disconnect cycles
│   │   └── mixed.rs         # Combined game + social traffic
```

### CLI

```
simulator --scenario movement --clients 50 --duration 120
simulator --scenario guild-chat --clients 20 --duration 60
simulator --scenario mixed --clients 30 --duration 300
```

Each scenario is independent and self-contained. Scenarios handle their own
provisioning (account/character creation) or rely on the provisioner service.

## Scenarios

### 1. Movement (existing)

Spawns N bots that connect to the game server and send randomized movement at 20Hz.

**What it exercises:** game server tick loop, spatial grid, entity sync, netcode throughput.

**Metrics to observe:** server tick duration, entity count, network bytes in/out.

### 2. Guild Chat

Spawns N bots in the same guild. Each bot sends a guild message every 1-5 seconds.
At the end, each bot reports how many messages it received vs expected.

**What it exercises:** NATS pub/sub throughput, WebSocket fan-out, hub message processing,
serialization overhead.

**Metrics to observe:** NATS message rate, WebSocket connection count, message delivery
latency (if timestamps are added to messages), hub channel backpressure.

### 3. Whisper Pairs

Spawns N bots arranged in pairs. Each pair exchanges whispers at a configurable rate.
Validates that every whisper arrives and measures round-trip time.

**What it exercises:** NATS point-to-point routing, cross-pod delivery (when bots land
on different web-server instances), whisper receipt generation.

**Metrics to observe:** whisper delivery latency, NATS subject cardinality, per-pod
WebSocket distribution.

### 4. Connection Churn

Spawns bots that repeatedly connect, stay for 5-30 seconds, disconnect, and reconnect.
Simulates realistic player session patterns.

**What it exercises:** connection lifecycle handling, subscription cleanup, memory leaks,
hub client map growth/shrinkage, Kubernetes connection draining during rollouts.

**Metrics to observe:** active WebSocket connections over time, memory usage trend,
NATS subscription count, goroutine/task count stability.

### 5. Mixed Traffic

Combines movement + guild chat + whisper + churn in realistic proportions:
- 60% of bots do movement only
- 20% do movement + guild chat
- 10% do movement + whisper pairs
- 10% do connection churn

**What it exercises:** the full system under realistic combined load. Useful for
capacity planning and identifying resource contention between subsystems.

**Metrics to observe:** all of the above, plus cross-system interference (e.g. does
chat load impact game tick duration?).

### 6. Burst / Spike (future)

Ramps from 0 to N clients over 10 seconds, holds for 30 seconds, then drops to 0.
Repeat. Tests autoscaler responsiveness and graceful degradation.

**What it exercises:** HPA scaling triggers, pod startup time, NATS reconnection
after pod churn, connection queuing during scale-up.

**Metrics to observe:** HPA replica count vs active connections, request latency
during scale-up, error rate during transitions.

## Implementation Notes

- Use `web-client` crate for all social bot interactions (account creation, WebSocket connect, actions/events).
- Each scenario should output a summary at the end: success rate, messages sent/received, average latency.
- Bots should use deterministic seeded RNG for reproducibility (already done for movement).
- Keep scenarios independent — don't force a bot to do both game and social unless that's the specific scenario.
- Consider adding a `--web-server-url` CLI flag alongside the existing game server flags.
- For guild chat, bots need to be in the same guild. Either pre-seed via the provisioner or have the scenario create a guild via API.

## Verification vs Load Generation

Some scenarios have a verification component (e.g. "did all messages arrive?").
This is optional and controlled by a `--verify` flag. Without it, the simulator
just generates traffic — useful for sustained load during platform development.
With `--verify`, it acts as a smoke test and exits non-zero on failures.
