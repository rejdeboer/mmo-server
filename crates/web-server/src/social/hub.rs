use flatbuffers::FlatBufferBuilder;
use futures_util::StreamExt;
use schemas::social::{self as schema, ChannelType};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::social::{
    command::{HubMessage, Recipient},
    error::HubError,
    nats::{NatsBridge, NatsEnvelope, guild_subject, whisper_subject},
};

use super::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Arc<[u8]>>,
}

/// Internal messages from NATS subscription tasks to the Hub.
enum NatsEvent {
    Guild {
        guild_id: i32,
        envelope: NatsEnvelope,
    },
    Whisper {
        character_id: i32,
        envelope: NatsEnvelope,
    },
}

pub struct Hub {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubMessage>,
    nats_rx: Receiver<NatsEvent>,
    nats_tx: Sender<NatsEvent>,
    guilds: HashMap<i32, Vec<i32>>,
    db_pool: PgPool,
    nats: Option<NatsBridge>,
    /// Track active guild subscriptions so we can unsubscribe
    guild_sub_handles: HashMap<i32, tokio::task::JoinHandle<()>>,
    /// Track active whisper subscriptions
    whisper_sub_handles: HashMap<i32, tokio::task::JoinHandle<()>>,
}

impl Hub {
    pub fn new(db_pool: PgPool, rx: Receiver<HubMessage>, nats: Option<NatsBridge>) -> Self {
        let (nats_tx, nats_rx) = channel::<NatsEvent>(256);
        Self {
            clients: HashMap::new(),
            rx,
            nats_rx,
            nats_tx,
            guilds: HashMap::new(),
            db_pool,
            nats,
            guild_sub_handles: HashMap::new(),
            whisper_sub_handles: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                Some(message) = self.rx.recv() => {
                    self.process_message(message).await;
                }

                Some(event) = self.nats_rx.recv() => {
                    match event {
                        NatsEvent::Guild { guild_id, envelope } => {
                            self.handle_remote_guild(guild_id, envelope).await;
                        }
                        NatsEvent::Whisper { character_id, envelope } => {
                            self.handle_remote_whisper(character_id, envelope).await;
                        }
                    }
                }

                else => break,
            }
        }
    }

    /// Handle a guild message arriving from NATS.
    async fn handle_remote_guild(&self, guild_id: i32, envelope: NatsEnvelope) {
        let Some(members) = self.guilds.get(&guild_id) else {
            return;
        };

        let msg: Arc<[u8]> = Arc::from(envelope.payload);
        for &member_id in members {
            if let Some(client) = self.clients.get(&member_id) {
                if let Err(err) = client.tx.send(msg.clone()).await {
                    tracing::error!(?err, member_id, "failed to deliver guild message");
                }
            }
        }
    }

    /// Handle a whisper arriving from NATS.
    async fn handle_remote_whisper(&self, character_id: i32, envelope: NatsEnvelope) {
        if let Some(client) = self.clients.get(&character_id) {
            let msg: Arc<[u8]> = Arc::from(envelope.payload);
            if let Err(err) = client.tx.send(msg).await {
                tracing::error!(?err, character_id, "failed to deliver whisper");
            }
        }
    }

    async fn process_message(&mut self, msg: HubMessage) {
        match msg.command {
            HubCommand::Connect {
                character_name,
                guild_id,
                tx,
            } => {
                // Spawn a whisper subscription task for this character
                self.spawn_whisper_sub(msg.sender_id);

                self.clients.insert(
                    msg.sender_id,
                    ConnectedClient {
                        character_name,
                        guild_id,
                        tx,
                    },
                );

                if let Some(guild_id) = guild_id {
                    let members = self.guilds.entry(guild_id).or_default();
                    let is_first = members.is_empty();
                    members.push(msg.sender_id);

                    if is_first {
                        self.spawn_guild_sub(guild_id);
                    }
                }
            }
            HubCommand::Disconnect => {
                // Cancel whisper subscription
                if let Some(handle) = self.whisper_sub_handles.remove(&msg.sender_id) {
                    handle.abort();
                }

                if let Some(client) = self.clients.remove(&msg.sender_id)
                    && let Some(gid) = client.guild_id
                    && let Some(members) = self.guilds.get_mut(&gid)
                {
                    members.retain(|&id| id != msg.sender_id);
                    if members.is_empty() {
                        self.guilds.remove(&gid);
                        if let Some(handle) = self.guild_sub_handles.remove(&gid) {
                            handle.abort();
                        }
                    }
                }
            }
            HubCommand::ChatMessage { channel, text } => {
                self.handle_chat(msg.sender_id, channel, &text).await;
            }
            HubCommand::Whisper { recipient, text } => {
                self.handle_whisper(msg.sender_id, recipient, &text).await;
            }
        };
    }

    fn spawn_whisper_sub(&mut self, character_id: i32) {
        let Some(nats) = self.nats.clone() else {
            return;
        };
        let nats_tx = self.nats_tx.clone();
        let subject = whisper_subject(character_id);

        let handle = tokio::spawn(async move {
            let mut sub = match nats.subscribe(&subject).await {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(?err, %subject, "failed to subscribe");
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                if let Some(envelope) = NatsBridge::deserialize_envelope(&msg.payload) {
                    if nats_tx
                        .send(NatsEvent::Whisper {
                            character_id,
                            envelope,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });

        self.whisper_sub_handles.insert(character_id, handle);
    }

    fn spawn_guild_sub(&mut self, guild_id: i32) {
        let Some(nats) = self.nats.clone() else {
            return;
        };
        let nats_tx = self.nats_tx.clone();
        let subject = guild_subject(guild_id);

        let handle = tokio::spawn(async move {
            let mut sub = match nats.subscribe(&subject).await {
                Ok(s) => s,
                Err(err) => {
                    tracing::error!(?err, %subject, "failed to subscribe");
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                if let Some(envelope) = NatsBridge::deserialize_envelope(&msg.payload) {
                    if nats_tx
                        .send(NatsEvent::Guild { guild_id, envelope })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });

        self.guild_sub_handles.insert(guild_id, handle);
    }

    async fn handle_chat(&self, sender_id: i32, channel: ChannelType, text: &str) {
        let sender_client = self.get_client_unchecked(&sender_id);
        match channel {
            ChannelType::Guild => {
                self.handle_guild_message(sender_id, sender_client, text)
                    .await
            }
            unsupported => {
                tracing::error!(?unsupported, "channel not supported");
                self.write_error(HubError::Unexpected, sender_client.tx.clone())
                    .await;
            }
        };
    }

    async fn handle_guild_message(
        &self,
        sender_id: i32,
        sender_client: &ConnectedClient,
        text: &str,
    ) {
        let Some(gid) = sender_client.guild_id else {
            return self
                .write_error(HubError::SenderNotInGuild, sender_client.tx.clone())
                .await;
        };

        let mut builder = FlatBufferBuilder::new();
        let fb_sender = builder.create_string(&sender_client.character_name);
        let fb_text = builder.create_string(text);
        let fb_msg = schema::ServerChatMessage::create(
            &mut builder,
            &schema::ServerChatMessageArgs {
                sender_id,
                channel: ChannelType::Guild,
                sender_name: Some(fb_sender),
                text: Some(fb_text),
            },
        )
        .as_union_value();
        let fb_event = schema::Event::create(
            &mut builder,
            &schema::EventArgs {
                data_type: schema::EventData::ServerChatMessage,
                data: Some(fb_msg),
            },
        );
        builder.finish_minimal(fb_event);
        let msg: Arc<[u8]> = Arc::from(builder.finished_data());

        // Publish to NATS — delivery happens when we receive our own message back
        if let Some(nats) = &self.nats {
            let envelope = NatsEnvelope {
                payload: msg.to_vec(),
            };
            nats.publish(&guild_subject(gid), &envelope).await;
        }
    }

    async fn handle_whisper(&self, sender_id: i32, recipient: Recipient, text: &str) {
        let sender_client = self.get_client_unchecked(&sender_id);
        let recipient_id = match self.resolve_recipient_id(recipient).await {
            Ok(id) => id,
            Err(err) => return self.write_error(err, sender_client.tx.clone()).await,
        };

        // Build the whisper message
        let mut builder = FlatBufferBuilder::new();
        let fb_sender = builder.create_string(&sender_client.character_name);
        let fb_text = builder.create_string(text);
        let fb_whisper = schema::ServerWhisper::create(
            &mut builder,
            &schema::ServerWhisperArgs {
                sender_id,
                sender_name: Some(fb_sender),
                text: Some(fb_text),
            },
        )
        .as_union_value();
        let fb_event = schema::Event::create(
            &mut builder,
            &schema::EventArgs {
                data_type: schema::EventData::ServerWhisper,
                data: Some(fb_whisper),
            },
        );
        builder.finish_minimal(fb_event);
        let whisper: Arc<[u8]> = Arc::from(builder.finished_data());

        // Try local delivery first
        if let Some(recipient_client) = self.clients.get(&recipient_id) {
            if let Err(err) = recipient_client.tx.send(whisper.clone()).await {
                tracing::error!(?err, "failed to send whisper to recipient");
                return self
                    .write_error(HubError::Unexpected, sender_client.tx.clone())
                    .await;
            }
        } else {
            // Recipient not on this instance - publish via NATS if available
            if let Some(nats) = &self.nats {
                let envelope = NatsEnvelope {
                    payload: whisper.to_vec(),
                };
                nats.publish(&whisper_subject(recipient_id), &envelope)
                    .await;
            } else {
                tracing::warn!(recipient_id, "recipient not connected and NATS unavailable");
            }
        }

        // Send receipt to sender (always local since sender is on this instance)
        builder.reset();
        let recipient_name = if let Some(rc) = self.clients.get(&recipient_id) {
            rc.character_name.clone()
        } else {
            match sqlx::query!("SELECT name FROM characters WHERE id = $1", recipient_id)
                .fetch_one(&self.db_pool)
                .await
            {
                Ok(row) => row.name,
                Err(_) => String::from("Unknown"),
            }
        };

        let fb_recipient = builder.create_string(&recipient_name);
        let fb_text = builder.create_string(text);
        let fb_receipt = schema::ServerWhisperReceipt::create(
            &mut builder,
            &schema::ServerWhisperReceiptArgs {
                recipient_id,
                recipient_name: Some(fb_recipient),
                text: Some(fb_text),
            },
        )
        .as_union_value();
        let fb_event = schema::Event::create(
            &mut builder,
            &schema::EventArgs {
                data_type: schema::EventData::ServerWhisperReceipt,
                data: Some(fb_receipt),
            },
        );
        builder.finish_minimal(fb_event);
        let receipt: Arc<[u8]> = Arc::from(builder.finished_data());

        if let Err(err) = sender_client.tx.send(receipt).await {
            tracing::error!(?err, "failed to send receipt to sender");
        }
    }

    async fn write_error(&self, error: HubError, tx: Sender<Arc<[u8]>>) {
        let text = match error {
            HubError::SenderNotInGuild => "You are not in a guild",
            HubError::RecipientNotFound => "Player not found",
            HubError::Unexpected => "An unexpected error occured, please try re-logging",
        };

        let mut builder = FlatBufferBuilder::new();
        let fb_text = builder.create_string(text);
        let fb_msg = schema::ServerSystemMessage::create(
            &mut builder,
            &schema::ServerSystemMessageArgs {
                text: Some(fb_text),
            },
        )
        .as_union_value();
        let fb_event = schema::Event::create(
            &mut builder,
            &schema::EventArgs {
                data_type: schema::EventData::ServerSystemMessage,
                data: Some(fb_msg),
            },
        );
        builder.finish_minimal(fb_event);
        let msg: Arc<[u8]> = Arc::from(builder.finished_data());

        if let Err(err) = tx.send(msg).await {
            tracing::error!(?err, "failed to send error");
        }
    }

    async fn resolve_recipient_id(&self, recipient: Recipient) -> Result<i32, HubError> {
        match recipient {
            Recipient::Id(id) => Ok(id),
            Recipient::Name(name) => {
                let id = sqlx::query!("SELECT id from characters WHERE name = $1 LIMIT 1", &name)
                    .fetch_one(&self.db_pool)
                    .await
                    .map_err(|err| match err {
                        sqlx::Error::Database(_) => HubError::RecipientNotFound,
                        _ => HubError::Unexpected,
                    })?
                    .id;

                Ok(id)
            }
        }
    }

    #[inline]
    fn get_client_unchecked(&self, client_id: &i32) -> &ConnectedClient {
        self.clients.get(client_id).expect("failed to fetch client")
    }
}
