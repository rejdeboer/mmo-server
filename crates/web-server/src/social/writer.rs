use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use futures::{SinkExt, stream::SplitSink};
use tokio::sync::mpsc::Receiver;

pub struct SocketWriter {
    pub socket_tx: SplitSink<WebSocket, Message>,
    pub hub_rx: Receiver<Arc<[u8]>>,
}

impl SocketWriter {
    pub fn new(socket_tx: SplitSink<WebSocket, Message>, hub_rx: Receiver<Arc<[u8]>>) -> Self {
        Self { socket_tx, hub_rx }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.hub_rx.recv().await {
            self.socket_tx
                .send(Message::Binary(Bytes::copy_from_slice(msg.as_ref())))
                .await
                .expect("websocket message written");
        }
    }
}
