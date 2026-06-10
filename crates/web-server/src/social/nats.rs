use async_nats::{Client, Subscriber};
use metrics::counter;
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

/// Extract the subject prefix (e.g. "social.guild.123" → "social.guild").
fn subject_prefix(subject: &str) -> &str {
    // Find the second-to-last dot or return the whole subject
    match subject.rmatch_indices('.').nth(0) {
        Some((idx, _)) => &subject[..idx],
        None => subject,
    }
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

    #[tracing::instrument(name = "nats.publish", skip(self, envelope), fields(%subject))]
    pub async fn publish(&self, subject: &str, envelope: &NatsEnvelope) {
        let prefix = subject_prefix(subject).to_owned();
        let bytes = match serde_json::to_vec(envelope) {
            Ok(b) => b,
            Err(err) => {
                tracing::error!(?err, "failed to serialize NATS envelope");
                return;
            }
        };

        if let Err(err) = self.client.publish(subject.to_owned(), bytes.into()).await {
            counter!("nats_publish_failures_total", "subject_prefix" => prefix.clone()).increment(1);
            tracing::error!(?err, %subject, "failed to publish to NATS");
        }
        counter!("nats_publishes_total", "subject_prefix" => prefix).increment(1);
    }

    #[tracing::instrument(name = "nats.publish", skip(self, msg), fields(%subject))]
    pub async fn publish_json<T: Serialize>(&self, subject: &str, msg: &T) {
        let prefix = subject_prefix(subject).to_owned();
        let bytes = match serde_json::to_vec(msg) {
            Ok(b) => b,
            Err(err) => {
                tracing::error!(?err, "failed to serialize NATS message");
                return;
            }
        };

        if let Err(err) = self.client.publish(subject.to_owned(), bytes.into()).await {
            counter!("nats_publish_failures_total", "subject_prefix" => prefix.clone()).increment(1);
            tracing::error!(?err, %subject, "failed to publish to NATS");
        }
        counter!("nats_publishes_total", "subject_prefix" => prefix).increment(1);
    }

    pub fn deserialize_envelope(data: &[u8]) -> Option<NatsEnvelope> {
        serde_json::from_slice(data).ok()
    }
}
