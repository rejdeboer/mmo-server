use std::ops::ControlFlow;

use axum::extract::ws::{Message as WSMessage, WebSocket};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use tokio::sync::mpsc::{error::SendError, Receiver, Sender};
use tracing::instrument;
use uuid::Uuid;

use crate::auth::User;

use super::Message;

#[derive(Debug)]
pub struct Client {
    id: Uuid,
    client_tx: Sender<Vec<u8>>,
    syncer_tx: Sender<Message>,
    user: User,
}

impl Client {
    pub fn new(user: User, client_tx: Sender<Vec<u8>>, syncer_tx: Sender<Message>) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_tx,
            syncer_tx,
            user,
        }
    }

    #[instrument(name="websocket client", skip_all, fields(user = ?self.user))]
    pub async fn run(self, socket: WebSocket, client_rx: Receiver<Vec<u8>>) {
        let (ws_tx, mut ws_rx) = socket.split();

        tokio::spawn(async move {
            write_pump(ws_tx, client_rx).await;
        });

        self.syncer_tx
            .send(Message::Connect(self.id, self.client_tx.clone()))
            .await
            .expect("client connects to syncer");
        tracing::info!("new client connected");

        while let Some(Ok(msg)) = ws_rx.next().await {
            if self.read_message(msg).await.is_break() {
                break;
            }
        }

        self.syncer_tx
            .send(Message::Disconnect(self.id))
            .await
            .expect("client disconnects from syncer");
        tracing::info!("client disconnected");
    }

    async fn read_message(&self, msg: WSMessage) -> ControlFlow<(), ()> {
        match msg {
            WSMessage::Binary(bytes) => {
                tracing::debug!(?bytes, "received bytes");
                if let Err(SendError(err)) = self.read_binary_message(bytes).await {
                    tracing::error!(?err, "error reading binary message");
                    return ControlFlow::Break(());
                };
            }
            WSMessage::Close(c) => {
                if let Some(cf) = c {
                    tracing::info!(code = %cf.code, reason = %cf.reason, "received close message");
                } else {
                    tracing::warn!("somehow received close message without CloseFrame");
                }
                return ControlFlow::Break(());
            }
            // Ping pong is handled by Axum, don't need to do anything here
            WSMessage::Pong(_) => (),
            WSMessage::Ping(_) => (),
            msg => tracing::warn!(?msg, "unhandled message"),
        }
        ControlFlow::Continue(())
    }

    async fn read_binary_message(&self, bytes: Vec<u8>) -> Result<(), SendError<Message>> {
        if bytes.is_empty() {
            tracing::error!("received empty binary message");
            return Ok(());
        }
        let message_type = *bytes.last().unwrap();

        match message_type {
            super::MESSAGE_UPDATE | super::MESSAGE_SYNC_STEP_2 => {
                tracing::info!("sending sync step 2 update");
                self.syncer_tx.send(Message::Update(self.id, bytes)).await?;
            }
            super::MESSAGE_SYNC_STEP_1 => {
                self.syncer_tx
                    .send(Message::GetDiff(self.id, bytes))
                    .await?;
            }
            super::MESSAGE_AWARENESS_UPDATE => {
                self.syncer_tx
                    .send(Message::UpdateAwareness(self.id, bytes))
                    .await?;
            }
            message_type => {
                tracing::error!(message_type, "unsupported message type");
            }
        };

        Ok(())
    }
}

async fn write_pump(mut ws_tx: SplitSink<WebSocket, WSMessage>, mut client_rx: Receiver<Vec<u8>>) {
    while let Some(msg) = client_rx.recv().await {
        ws_tx
            .send(WSMessage::Binary(msg))
            .await
            .expect("websocket message written");
    }
}
