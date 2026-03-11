use flatbuffers::FlatBufferBuilder;
use schemas::social::{self as schema, ChannelType};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::social::{
    command::{HubMessage, Recipient},
    error::HubError,
};

use super::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Arc<[u8]>>,
}

pub struct Hub {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubMessage>,
    guilds: HashMap<i32, Vec<i32>>,
    db_pool: PgPool,
}

impl Hub {
    pub fn new(db_pool: PgPool, receiver: Receiver<HubMessage>) -> Self {
        Self {
            clients: HashMap::new(),
            rx: receiver,
            guilds: HashMap::new(),
            db_pool,
        }
    }

    pub async fn run(mut self) {
        while let Some(message) = self.rx.recv().await {
            self.process_message(message).await;
        }
    }

    async fn process_message(&mut self, msg: HubMessage) {
        match msg.command {
            HubCommand::Connect {
                character_name,
                guild_id,
                tx,
            } => {
                self.clients.insert(
                    msg.sender_id,
                    ConnectedClient {
                        character_name,
                        guild_id,
                        tx,
                    },
                );

                if let Some(guild_id) = guild_id {
                    self.guilds.entry(guild_id).or_default().push(msg.sender_id);
                }
            }
            HubCommand::Disconnect => {
                if let Some(client) = self.clients.remove(&msg.sender_id)
                    && let Some(gid) = client.guild_id
                    && let Some(members) = self.guilds.get_mut(&gid)
                {
                    members.retain(|&id| id != msg.sender_id);
                    if members.is_empty() {
                        self.guilds.remove(&gid);
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

        let members = self.get_guild_members_unchecked(&gid);
        for member in members {
            let Some(member_client) = self.clients.get(member) else {
                tracing::warn!(?member, ?gid, "member not found in guild");
                continue;
            };

            if let Err(err) = member_client.tx.send(msg.clone()).await {
                tracing::error!(?err, ?member, "failed to send message to guild member");
            }
        }
    }

    async fn handle_whisper(&self, sender_id: i32, recipient: Recipient, text: &str) {
        let sender_client = self.get_client_unchecked(&sender_id);
        let recipient_id = match self.resolve_recipient_id(recipient).await {
            Ok(id) => id,
            Err(err) => return self.write_error(err, sender_client.tx.clone()).await,
        };

        let Some(recipient_client) = self.clients.get(&recipient_id) else {
            return self
                .write_error(HubError::RecipientNotFound, sender_client.tx.clone())
                .await;
        };

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
        builder.reset();

        if let Err(err) = recipient_client.tx.send(whisper.clone()).await {
            tracing::error!(?err, "failed to send whisper to recipient");
            return self
                .write_error(HubError::Unexpected, sender_client.tx.clone())
                .await;
        }

        let fb_recipient = builder.create_string(&recipient_client.character_name);
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

    // WARNING: Utitlity function to grab the sender client from the hashmap
    #[inline]
    fn get_client_unchecked(&self, client_id: &i32) -> &ConnectedClient {
        self.clients.get(client_id).expect("failed to fetch client")
    }

    #[inline]
    fn get_guild_members_unchecked(&self, gid: &i32) -> &Vec<i32> {
        self.guilds.get(gid).expect("failed to fetch guild members")
    }
}
