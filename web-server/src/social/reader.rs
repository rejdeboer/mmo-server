use std::{ops::ControlFlow, sync::Arc};

use crate::social::{
    command::{HubCommand, Recipient},
    error::ReaderError,
};

use axum::extract::ws::{Message, WebSocket};
use flatbuffers::root;
use futures::{StreamExt, stream::SplitStream};
use schemas::social as schema;
use tokio::sync::mpsc::Sender;

pub struct SocketReader {
    pub character_id: i32,
    pub socket_rx: SplitStream<WebSocket>,
    pub hub_tx: Sender<HubCommand>,
}

impl SocketReader {
    pub fn new(
        character_id: i32,
        socket_rx: SplitStream<WebSocket>,
        hub_tx: Sender<HubCommand>,
    ) -> Self {
        Self {
            character_id,
            socket_rx,
            hub_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(Ok(msg)) = self.socket_rx.next().await {
            if self.read_message(msg).await.is_break() {
                break;
            }
        }
        // TODO: Send disconnect message to hub
    }

    async fn read_message(&mut self, msg: Message) -> ControlFlow<(), ()> {
        match msg {
            Message::Binary(_) => {
                todo!("read flatbuffer");
            }
            Message::Close(c) => {
                if let Some(cf) = c {
                    tracing::info!(code = %cf.code, reason = %cf.reason, "received close message");
                } else {
                    tracing::warn!("somehow received close message without CloseFrame");
                }
                return ControlFlow::Break(());
            }
            Message::Text(text) => {
                tracing::warn!(?text, "received text message");
            }
            // Ping pong is handled by Axum, don't need to do anything here
            Message::Ping(_) => {}
            Message::Pong(_) => {}
        };
        ControlFlow::Continue(())
    }

    async fn read_binary_message(&mut self, bytes: Vec<u8>) -> Result<(), ReaderError> {
        let fb_action = root::<schema::Action>(&bytes).map_err(ReaderError::InvalidSchema)?;

        let cmd = match fb_action.data_type() {
            schema::ActionData::ClientChatMessage => {
                let data = fb_action.data_as_client_chat_message().unwrap();
                Ok(HubCommand::ChatMessage {
                    sender_id: self.character_id,
                    channel: data.channel(),
                    text: Arc::from(data.text()),
                })
            }
            schema::ActionData::ClientWhisperById => {
                let data = fb_action.data_as_client_whisper_by_id().unwrap();
                Ok(HubCommand::Whisper {
                    sender_id: self.character_id,
                    text: Arc::from(data.text()),
                    recipient: Recipient::Id(data.recipient_id()),
                })
            }
            action_type => Err(ReaderError::InvalidActionType(action_type)),
        }?;

        self.hub_tx
            .send(cmd)
            .await
            .map_err(ReaderError::HubSendFailure)?;

        Ok(())
    }
}
