use std::{collections::HashMap, ops::ControlFlow};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{Instrument, instrument};

use crate::chat::command::HubCommand;

struct ConnectedClient {
    character_name: String,
    guild_id: Option<i32>,
    tx: Sender<Vec<u8>>,
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
                while let Some(message) = self.rx.recv().await {
                    if self.process_message(message).await.is_break() {
                        tracing::info!("stopping hub");
                        break;
                    };
                }
            }
            .instrument(tracing::Span::current()),
        );
    }

    async fn process_message(&mut self, msg: HubCommand) -> ControlFlow<(), ()> {
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
            HubCommand::Guild { text } => {}
            HubCommand::Whisper { recipient_id, text } => {}
        };
        ControlFlow::Continue(())
    }
}
