use std::{ops::ControlFlow, sync::Arc};

use crate::chat::{
    command::{GuildMessage, HubCommand, WhisperMessage},
    error::ChatClientError,
};

use super::ChatContext;
use axum::extract::ws::{Message, WebSocket};
use flatbuffers::root;
use schemas::mmo::ChannelType;

pub struct Client {
    pub ctx: ChatContext,
    pub socket: WebSocket,
}

impl Client {
    pub fn new(ctx: ChatContext, socket: WebSocket) -> Self {
        Self { ctx, socket }
    }

    pub async fn run(mut self) {
        while let Some(Ok(msg)) = self.socket.recv().await {
            if self.read_message(msg).is_break() {
                break;
            }
        }
    }

    fn read_message(&mut self, msg: Message) -> ControlFlow<(), ()> {
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

    fn read_binary_message(&mut self, bytes: Vec<u8>) -> Result<(), ChatClientError> {
        let fb_msg = root::<schemas::mmo::ClientChatMessage>(&bytes)
            .map_err(ChatClientError::DecodeError)?;

        let msg = match fb_msg.channel() {
            ChannelType::Whisper => Ok(HubCommand::Whisper(WhisperMessage {
                author_id: self.ctx.character_id,
                author_name: self.ctx.character_name.clone(),
                text: Arc::from(fb_msg.text()),
                recipient_id: fb_msg.recipient_id(),
            })),
            ChannelType::Guild => Ok(HubCommand::Guild(GuildMessage {
                author_id: self.ctx.character_id,
                author_name: self.ctx.character_name.clone(),
                text: Arc::from(fb_msg.text()),
            })),
            channel => Err(ChatClientError::InvalidChannel(channel)),
        }?;

        // TODO: Send to hub

        Ok(())
    }
}
