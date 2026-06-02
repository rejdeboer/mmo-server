use crate::social::{
    HubMessage,
    command::{HubCommand, Recipient},
    error::ReaderError,
};
use axum::extract::ws::{Message, WebSocket};
use futures::{StreamExt, stream::SplitStream};
use protocol::social::SocialAction;
use std::ops::ControlFlow;
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
        let action: SocialAction =
            bitcode::decode(&bytes).map_err(|e| ReaderError::InvalidPayload(e.to_string()))?;

        let cmd = match action {
            SocialAction::Chat { channel, text } => HubCommand::ChatMessage { channel, text },
            SocialAction::WhisperByName {
                recipient_name,
                text,
            } => HubCommand::Whisper {
                recipient: Recipient::Name(recipient_name),
                text,
            },
            SocialAction::WhisperById { recipient_id, text } => HubCommand::Whisper {
                recipient: Recipient::Id(recipient_id),
                text,
            },
            SocialAction::PartyInviteById { target_id } => HubCommand::PartyInvite {
                target: Recipient::Id(target_id),
            },
            SocialAction::PartyInviteByName { target_name } => HubCommand::PartyInvite {
                target: Recipient::Name(target_name),
            },
            SocialAction::PartyAccept => HubCommand::PartyAccept,
            SocialAction::PartyDecline => HubCommand::PartyDecline,
            SocialAction::PartyLeave => HubCommand::PartyLeave,
            SocialAction::PartyKick { target_id } => HubCommand::PartyKick { target_id },
        };

        self.hub_tx
            .send(HubMessage::new(self.character_id, cmd))
            .await
            .map_err(ReaderError::HubSendFailure)?;

        Ok(())
    }
}
