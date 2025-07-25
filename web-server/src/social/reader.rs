use std::ops::ControlFlow;

use crate::social::{
    HubMessage,
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
    pub hub_tx: Sender<HubMessage>,
}

impl SocketReader {
    pub fn new(
        character_id: i32,
        socket_rx: SplitStream<WebSocket>,
        hub_tx: Sender<HubMessage>,
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
            Message::Binary(bytes) => {
                if let Err(err) = self.read_binary_message(bytes.into()).await {
                    // TODO: We should probably make sure the websocket writer will close as well
                    tracing::error!(?err, "failed to send message to hub");
                    return ControlFlow::Break(());
                }
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
                    channel: data.channel(),
                    text: data.text().to_string(),
                })
            }
            schema::ActionData::ClientWhisperById => {
                let data = fb_action.data_as_client_whisper_by_id().unwrap();
                Ok(HubCommand::Whisper {
                    text: data.text().to_string(),
                    recipient: Recipient::Id(data.recipient_id()),
                })
            }
            action_type => Err(ReaderError::InvalidActionType(action_type)),
        }?;

        self.hub_tx
            .send(HubMessage::new(self.character_id, cmd))
            .await
            .map_err(ReaderError::HubSendFailure)?;

        Ok(())
    }
}
