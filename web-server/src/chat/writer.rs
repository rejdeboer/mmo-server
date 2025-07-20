use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, stream::SplitSink};
use tokio::sync::mpsc::Receiver;

pub struct SocketWriter {
    pub socket_tx: SplitSink<WebSocket, Message>,
    pub hub_rx: Receiver<Vec<u8>>,
}

impl SocketWriter {
    pub fn new(socket_tx: SplitSink<WebSocket, Message>, hub_rx: Receiver<Vec<u8>>) -> Self {
        Self { socket_tx, hub_rx }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.hub_rx.recv().await {
            self.socket_tx
                .send(Message::Binary(msg.into()))
                .await
                .expect("websocket message written");
        }
    }
}
