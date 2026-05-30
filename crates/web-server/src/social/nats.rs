use async_nats::{Client, Subscriber};
use serde::{Deserialize, Serialize};

/// Messages published over NATS between web-server instances.
/// These carry pre-serialized FlatBuffer payloads so the receiving hub
/// can forward them directly to client writers without re-serialization.
#[derive(Serialize, Deserialize)]
pub struct NatsEnvelope {
    /// The character_id of the original sender (used to skip self-delivery)
    pub origin_sender_id: i32,
    /// The pre-built FlatBuffer event bytes
    pub payload: Vec<u8>,
}

/// Subject helpers
pub fn guild_subject(guild_id: i32) -> String {
    format!("social.guild.{guild_id}")
}

pub fn whisper_subject(character_id: i32) -> String {
    format!("social.whisper.{character_id}")
}

/// Thin wrapper around the async-nats client for social messaging.
#[derive(Clone)]
pub struct NatsBridge {
    client: Client,
}

impl NatsBridge {
    pub async fn connect(url: &str) -> Result<Self, async_nats::ConnectError> {
        let client = async_nats::connect(url).await?;
        tracing::info!(%url, "connected to NATS");
        Ok(Self { client })
    }

    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber, async_nats::SubscribeError> {
        self.client.subscribe(subject.to_owned()).await
    }

    pub async fn publish(&self, subject: &str, envelope: &NatsEnvelope) {
        let bytes = match serde_json::to_vec(envelope) {
            Ok(b) => b,
            Err(err) => {
                tracing::error!(?err, "failed to serialize NATS envelope");
                return;
            }
        };

        if let Err(err) = self.client.publish(subject.to_owned(), bytes.into()).await {
            tracing::error!(?err, %subject, "failed to publish to NATS");
        }
    }

    pub fn deserialize_envelope(data: &[u8]) -> Option<NatsEnvelope> {
        serde_json::from_slice(data).ok()
    }
}
