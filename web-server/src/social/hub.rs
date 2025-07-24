use flatbuffers::FlatBufferBuilder;
use schemas::social as schema;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{instrument};

use crate::social::{command::{HubMessage, Recipient}, error::HubError};

use super::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Vec<u8>>,
}

impl ConnectedClient {
    pub async fn write_error(&self, error: HubError) {
        // TODO: Write out system error message back to client
    }
}

pub struct Hub<'fbb> {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubMessage>,
    guilds: HashMap<i32, Vec<i32>>,
    db_pool: PgPool,
    fb_builder: FlatBufferBuilder<'fbb>,
}

impl Hub<'_> {
    pub fn new(db_pool: PgPool, receiver: Receiver<HubMessage>) -> Self {
        Self {
            clients: HashMap::new(),
            rx: receiver,
            guilds: HashMap::new(),
            db_pool,
            fb_builder: FlatBufferBuilder::new(),
        }
    }

    #[instrument(name="Hub", parent=None, skip(self))]
    pub async fn run(mut self) {
        while let Some(message) = self.rx.recv().await {
            self.process_message(message).await;
            self.fb_builder.reset();
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
                if let Some(client) = self.clients.remove(&msg.sender_id) {
                    if let Some(gid) = client.guild_id {
                        if let Some(members) = self.guilds.get_mut(&gid) {
                            members.retain(|&id| id != msg.sender_id);
                            if members.is_empty() {
                                self.guilds.remove(&gid);
                            }
                        }
                    }
                }
            }
            HubCommand::ChatMessage { channel, text } => {
                let Some(client) = self.clients.get(&sender_id) else {
                    tracing::error!(?sender_id, "failed to get sender client");
                    return Err(HubError::Unexpected);
                };

                let Some(gid) = client.guild_id else {
                    tracing::error!(?sender_id, "sender is not in guild");
                    return Err(HubError::SenderNotInGuild);
                };
            }
            HubCommand::Whisper {
                sender_id,
                recipient,
                text,
            } => {
                let recipient_id = match recipient {
                    Recipient::Id(id) => id,
                    Recipient::Name(name) =>
                }
                let sender_client = self
                    .clients
                    .get(&sender_id)
                    .expect("failed to get sender client");
                let recipient_client = self
                    .clients
                    .get(&recipient_id)
                    .expect("failed to get recipient client");

                let fb_sender = self.fb_builder.create_string(&sender_client.character_name);
                let fb_recipient = self.fb_builder.create_string(&recipient_client.character_name);
                let fb_text = self.fb_builder.create_string(&text);
                let fb_msg = schema::ServerWhisper::create(
                    &mut self.fb_builder,
                    &schema::ServerWhisperArgs {
                        sender_id,
                        sender_name: Some(fb_sender),
                        recipient_id,
                        recipient_name: Some(fb_recipient),
                        text: Some(fb_text),
                    },
                )
                .as_union_value();
                let fb_event = schema::Event::create(
                    &mut self.fb_builder,
                    &schema::EventArgs {
                        data_type: schema::EventData::ServerWhisper,
                        data: Some(fb_msg),
                    },
                );
                self.fb_builder.finish_minimal(fb_event);
                let bytes = self.fb_builder.finished_data().to_vec();

                sender_client
                    .tx
                    .send(bytes.clone())
                    .await
                    .expect("failed to send to sender");
                recipient_client
                    .tx
                    .send(bytes)
                    .await
                    .expect("failed to send to recipient");
            }
        };
    }

    async fn get_character_id_by_name(&self, character_name: &str) -> Result<i32, HubError> {
        let id = sqlx::query!(
            "SELECT id from characters WHERE name = $1 LIMIT 1", 
            character_name
        ).fetch_one(&self.db_pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(_) => HubError::RecipientNotFound,
            _ => HubError::Unexpected,
        })?
        .id;

        Ok(id)
    }
}
