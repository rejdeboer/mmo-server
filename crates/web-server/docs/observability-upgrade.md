# Web Server Observability Upgrade

## Overview

Upgrade the web-server's observability stack to provide visibility into social
feature usage and health. This involves migrating from the `prometheus` crate to
the `metrics` facade (already used by game-server) and adding comprehensive
instrumentation to the social hub, NATS bridge, and HTTP layer.

## Current State

- Single metric: `web_social_active_ws_connections` (IntGauge via `prometheus` crate)
- OpenTelemetry traces exported via OTLP/gRPC to Tempo
- Structured JSON logs shipped to Loki
- `tower-http` TraceLayer for HTTP request spans
- `#[instrument]` on a handful of route handlers

## 1. Replace `prometheus` with `metrics` Facade

The `game-server` already uses `metrics` 0.24 + `metrics-exporter-prometheus` 0.18.
The web-server should align on the same approach.

**Why:**
- Ergonomic macros (`counter!`, `gauge!`, `histogram!`) vs manual `Lazy<IntGauge>` boilerplate
- Facade pattern (like `tracing`) -- recording is decoupled from export
- Labels are first-class -- no need to pre-register label combinations
- Consistent across the workspace

**Migration steps:**
- Remove `prometheus` from workspace and web-server dependencies
- Add `metrics` and `metrics-exporter-prometheus` to web-server
- Replace the custom `/metrics` handler with `PrometheusBuilder` from the exporter
- Delete `REGISTRY`, `ACTIVE_WS_CONNECTIONS` statics and `ConnectionGuard` -- replace with direct `gauge!` calls

## 2. Social Feature Metrics

### Connections

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_connections_active` | Gauge | -- | Current WebSocket connections |
| `social_connection_duration_seconds` | Histogram | -- | Session lifetimes (detect flapping) |

### Chat

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_messages_total` | Counter | `channel` (guild/party/whisper) | Volume by chat type |
| `social_messages_delivered_total` | Counter | `channel`, `delivery` (local/nats) | Local vs cross-instance delivery |

### Party

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_party_actions_total` | Counter | `action` (invite/accept/decline/leave/kick) | Party feature usage |
| `social_parties_active` | Gauge | -- | Current party count |

### Rate Limiting

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_rate_limit_denied_total` | Counter | -- | Signals abusive clients or too-tight limits |

### Errors

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_errors_total` | Counter | `error` (not_in_guild/recipient_not_found/rate_limited/...) | Error distribution |

### NATS Health

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `nats_publishes_total` | Counter | `subject_prefix` (social.guild/social.whisper/social.party/party.update) | Outbound volume |
| `nats_publish_failures_total` | Counter | `subject_prefix` | NATS reliability |
| `nats_messages_received_total` | Counter | `subject_prefix` | Inbound volume |

### Hub Internals

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `social_hub_queue_depth` | Gauge | `channel` (commands/nats) | Backpressure indicator |
| `social_guilds_active` | Gauge | -- | Guilds with connected members |

## 3. HTTP Layer Metrics

Add an Axum middleware that records metrics for all routes:

| Metric | Type | Labels | Purpose |
|--------|------|--------|---------|
| `http_requests_total` | Counter | `method`, `route`, `status` | Request volume and error rates |
| `http_request_duration_seconds` | Histogram | `method`, `route` | Latency distribution (p50/p95/p99) |

## 4. Tracing Improvements

### Hub-level span

Wrap `Hub::run` so all hub activity is grouped under a single long-lived span:
```rust
#[tracing::instrument(name = "social_hub", skip_all)]
pub async fn run(mut self) { ... }
```

### Per-command spans

Instrument `process_message` with the command variant and sender_id for
per-action traces in Tempo:
```rust
#[tracing::instrument(name = "hub.process", skip(self, msg), fields(sender_id = msg.sender_id, command = %msg.command))]
```

### NATS publish span

Wrap `NatsBridge::publish` for cross-instance trace correlation:
```rust
#[tracing::instrument(name = "nats.publish", skip(self, envelope), fields(%subject))]
```

### WebSocket lifecycle span

`handle_socket` should have a span covering the full connection lifetime with
`character_id` and `character_name` as fields. This enables filtering Loki logs
and Tempo traces by player.

### Structured log fields

Ensure all hub error paths include:
- `character_id`
- Command variant name
- `guild_id` / `party_id` where applicable

Log rate limit denials at `warn` level with the character_id for querying in
Loki without a custom dashboard.

## 5. Additional Improvements

### SQLx pool metrics

Expose `PgPool` stats via a periodic task or gauge callback:
- `db_pool_connections_active`
- `db_pool_connections_idle`
- `db_pool_connections_max`

### NATS reconnection logging

`async-nats` emits connection state events. Log reconnections at `warn` level
to surface intermittent network issues.

## 6. Alerting Recommendations

With these metrics exported to Prometheus, configure alerts for:

| Condition | Meaning |
|-----------|---------|
| `social_hub_queue_depth > 100` sustained | Hub overloaded |
| `rate(nats_publish_failures_total) > 0` | NATS connectivity issue |
| `social_connections_active` sudden drop | Mass disconnect / crash |
| `http_request_duration_seconds` p99 > threshold | Latency regression |

## 7. Implementation Order

1. Add `metrics` + `metrics-exporter-prometheus` to `web-server/Cargo.toml`, remove `prometheus`
2. Create `src/metrics.rs` -- metric name constants and exporter initialization
3. Instrument `routes/social.rs` (connection gauge, duration histogram)
4. Instrument `social/hub.rs` (message counters, party gauge, rate limit counter, error counter, queue depth)
5. Instrument `social/nats.rs` (publish/receive counters, failure counter)
6. Add HTTP request metrics middleware to the Axum router
7. Add tracing spans as described above
8. Remove old `prometheus`-based statics and `ConnectionGuard`
