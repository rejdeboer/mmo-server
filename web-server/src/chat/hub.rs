use flatbuffers::FlatBufferBuilder;
use schemas::mmo::ChannelType;
use std::{collections::HashMap, ops::ControlFlow};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{Instrument, instrument};

use crate::chat::command::HubCommand;

struct ConnectedClient {
    pub character_name: String,
    pub guild_id: Option<i32>,
    pub tx: Sender<Vec<u8>>,
}

pub struct Hub {
    clients: HashMap<i32, ConnectedClient>,
    rx: Receiver<HubCommand>,
}

impl Hub {
    pub fn build() -> (Self, Sender<HubCommand>) {
        let (tx, rx) = channel::<HubCommand>(128);

        (
            Self {
                clients: HashMap::new(),
                rx,
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
                    if self.process_message(message, &mut builder).await.is_break() {
                        tracing::info!("stopping hub");
                        break;
                    };
                }
            }
            .instrument(tracing::Span::current()),
        );
    }

    async fn process_message(
        &mut self,
        msg: HubCommand,
        builder: &mut FlatBufferBuilder<'_>,
    ) -> ControlFlow<(), ()> {
        match msg {
            HubCommand::Connect {
                character_id,
                character_name,
                tx,
            } => {
                self.clients.insert(
                    character_id,
                    ConnectedClient {
                        character_name,
                        guild_id: None,
                        tx,
                    },
                );
            }
            HubCommand::Guild { sender_id, text } => {}
            HubCommand::Whisper {
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

                let fb_author = builder.create_string(&sender_client.character_name);
                let fb_text = builder.create_string(&text);
                let fb_msg = schemas::mmo::ServerChatMessage::create(
                    builder,
                    &schemas::mmo::ServerChatMessageArgs {
                        channel: ChannelType::Whisper,
                        author_name: Some(fb_author),
                        text: Some(fb_text),
                    },
                );
                builder.finish_minimal(fb_msg);
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
        ControlFlow::Continue(())
    }
}
