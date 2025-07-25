use flatbuffers::FlatBufferBuilder;
use schemas::social as schema;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::social::{
    command::{HubMessage, Recipient},
    error::HubError,
};

use super::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Vec<u8>>,
}

impl ConnectedClient {}

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
            HubCommand::ChatMessage { channel, text } => {}
            HubCommand::Whisper { recipient, text } => {
                self.handle_whisper(msg.sender_id, recipient, text.as_ref())
                    .await;
            }
        };
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
        let fb_recipient = builder.create_string(&recipient_client.character_name);
        let fb_text = builder.create_string(&text);
        let fb_msg = schema::ServerWhisper::create(
            &mut builder,
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
            &mut builder,
            &schema::EventArgs {
                data_type: schema::EventData::ServerWhisper,
                data: Some(fb_msg),
            },
        );
        builder.finish_minimal(fb_event);
        let bytes = builder.finished_data().to_vec();

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

    async fn write_error(&self, error: HubError, tx: Sender<Vec<u8>>) {
        // TODO: Write out system error message back to client
    }

    async fn resolve_recipient_id(&self, recipient: Recipient) -> Result<i32, HubError> {
        match recipient {
            Recipient::Id(id) => Ok(id),
            Recipient::Name(name) => {
                let id = sqlx::query!(
                    "SELECT id from characters WHERE name = $1 LIMIT 1",
                    name.as_ref(),
                )
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
    fn get_client_unchecked(&self, client_id: &i32) -> &ConnectedClient {
        self.clients.get(client_id).expect("failed to fetch client")
    }
}
