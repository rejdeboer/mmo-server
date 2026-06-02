use async_nats::{Client, Subscriber};
use serde::{Deserialize, Serialize};

/// Messages published over NATS between web-server instances.
/// These carry pre-serialized payloads so the receiving hub
/// can forward them directly to client writers without re-serialization.
#[derive(Serialize, Deserialize)]
pub struct NatsEnvelope {
    /// The pre-built Bitcode event bytes
    pub payload: Vec<u8>,
}

/// Subject helpers
pub fn guild_subject(guild_id: i32) -> String {
    format!("social.guild.{guild_id}")
}

pub fn whisper_subject(character_id: i32) -> String {
    format!("social.whisper.{character_id}")
}

pub fn party_chat_subject(party_id: i32) -> String {
    format!("social.party.{party_id}")
}

/// Subject for party membership updates (consumed by game server).
pub fn party_update_subject(character_id: i32) -> String {
    format!("party.update.{character_id}")
}

/// Party membership update published to NATS for game server consumption.
#[derive(Serialize, Deserialize)]
pub struct PartyUpdate {
    pub party_id: Option<i32>,
    pub members: Vec<i32>,
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

    pub async fn publish_json<T: Serialize>(&self, subject: &str, msg: &T) {
        let bytes = match serde_json::to_vec(msg) {
            Ok(b) => b,
            Err(err) => {
                tracing::error!(?err, "failed to serialize NATS message");
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
