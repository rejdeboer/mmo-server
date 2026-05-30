# NATS Social Messaging Design

## Overview

The web-server uses NATS as a message bus to enable cross-instance communication
for social features (guild chat, whispers). This allows horizontal scaling of
web-server pods without losing the ability to deliver messages between players
connected to different instances.

## Architecture

```
┌─────────────────┐         ┌─────────────────┐
│  Web Server A   │         │  Web Server B   │
│                 │         │                 │
│  ┌───────────┐  │  NATS   │  ┌───────────┐  │
│  │    Hub    │◄─┼────────►┼──│    Hub    │  │
│  └───────────┘  │         │  └───────────┘  │
│   ▲ ▲ ▲        │         │   ▲ ▲ ▲        │
│   │ │ │        │         │   │ │ │        │
│  Clients       │         │  Clients       │
└─────────────────┘         └─────────────────┘
```

Each Hub instance:
- Delivers messages **locally first** to connected clients on the same instance
- Publishes to NATS for delivery to clients on **other** instances
- Subscribes to NATS subjects relevant to its locally connected clients

## NATS Subject Design

| Subject Pattern | Purpose |
|---|---|
| `social.guild.<guild_id>` | Guild chat broadcast |
| `social.whisper.<character_id>` | Direct whisper to a character |

## Message Flow

### Guild Chat (Local + Remote)

1. Player sends guild chat message
2. Hub delivers directly to all local guild members (zero NATS latency)
3. Hub publishes an envelope to `social.guild.<guild_id>` via NATS
4. Other instances subscribed to that guild subject receive the message
5. Remote instances deliver to their local guild members, skipping `origin_sender_id`

### Whisper (Local-First)

1. Player sends whisper to a recipient
2. Hub checks if recipient is connected locally
   - **If local**: delivers directly, no NATS publish
   - **If remote**: publishes to `social.whisper.<recipient_id>` via NATS
3. The instance where the recipient is connected receives and delivers locally
4. Whisper receipt is always sent locally to the sender

## Subscription Lifecycle

- **On Connect**: Hub spawns a background task subscribing to `social.whisper.<character_id>`.
  If the character is the first guild member on this instance, also subscribes to `social.guild.<guild_id>`.
- **On Disconnect**: Subscription tasks are aborted. If the last guild member on
  this instance disconnects, the guild subscription is also removed.

## Envelope Format

Messages on the NATS bus use a JSON envelope wrapping the pre-serialized FlatBuffer payload:

```json
{
  "origin_sender_id": 42,
  "payload": [/* FlatBuffer bytes */]
}
```

The `origin_sender_id` field allows receiving instances to avoid double-delivering
to the original sender (relevant for guild chat where the sender is also a member).

## Configuration

```yaml
nats:
  url: "nats://localhost:4222"
```

Environment variable override: `APP__NATS__URL`

## Future Considerations

- **Invites**: Add `social.invite.<character_id>` subject when party/guild invites are implemented
- **JetStream**: If offline message delivery is needed, migrate from core NATS pub/sub to JetStream with message persistence
- **Presence**: Could publish connect/disconnect events to track cross-instance online status
