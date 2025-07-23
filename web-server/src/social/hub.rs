use flatbuffers::FlatBufferBuilder;
use schemas::social as schema;
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{Instrument, instrument};

use super::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Vec<u8>>,
}

pub struct Hub {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubCommand>,
    guilds: HashMap<i32, Vec<i32>>,
}

impl Hub {
    pub fn build() -> (Self, Sender<HubCommand>) {
        let (tx, rx) = channel::<HubCommand>(128);

        (
            Self {
                clients: HashMap::new(),
                rx,
                guilds: HashMap::new(),
            },
            tx,
        )
    }

    #[instrument(name="Hub", parent=None, skip(self))]
    pub fn run(mut self) {
        tokio::spawn(
            async move {
                tracing::info!("starting hub");
                let mut builder = FlatBufferBuilder::new();
                while let Some(message) = self.rx.recv().await {
                    self.process_message(message, &mut builder).await;
                }
            }
            .instrument(tracing::Span::current()),
        );
    }

    async fn process_message(&mut self, msg: HubCommand, builder: &mut FlatBufferBuilder<'_>) {
        match msg {
            HubCommand::Connect {
                character_id,
                character_name,
                guild_id,
                tx,
            } => {
                self.clients.insert(
                    character_id,
                    ConnectedClient {
                        character_name,
                        guild_id,
                        tx,
                    },
                );

                if let Some(guild_id) = guild_id {
                    self.guilds.entry(guild_id).or_default().push(character_id);
                }
            }
            HubCommand::Disconnect { character_id } => {
                if let Some(client) = self.clients.remove(&character_id) {
                    if let Some(gid) = client.guild_id {
                        if let Some(members) = self.guilds.get_mut(&gid) {
                            members.retain(|&id| id != character_id);
                            if members.is_empty() {
                                self.guilds.remove(&gid);
                            }
                        }
                    }
                }
            }
            HubCommand::ChatMessage { sender_id, text } => {
                let Some(client) = self.clients.get(&sender_id) else {
                    return tracing::error!(?sender_id, "failed to get sender client");
                };

                let Some(gid) = client.guild_id else {
                    return tracing::error!(?sender_id, "sender is not in guild");
                    // TODO: Send error back to client
                };
            }
            HubCommand::WhisperById {
                sender_id,
                recipient_id,
                text,
            } => {
                let sender_client = self
                    .clients
                    .get(&sender_id)
                    .expect("failed to get sender client");
                let recipient_client = self
                    .clients
                    .get(&recipient_id)
                    .expect("failed to get recipient client");

                let fb_sender = builder.create_string(&sender_client.character_name);
                let fb_recipient = builder.create_string(&recipient_client.character_name);
                let fb_text = builder.create_string(&text);
                let fb_msg = schema::ServerWhisper::create(
                    builder,
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
                    builder,
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
        };
        builder.reset();
    }
}
