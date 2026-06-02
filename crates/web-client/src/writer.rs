use futures_util::{SinkExt, stream::SplitSink};
use protocol::social::SocialAction;
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Bytes, Message},
};

type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub async fn run_writer_task(
    mut ws_writer: WsWriter,
    mut command_rx: mpsc::Receiver<SocialAction>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    loop {
        tokio::select! {
            Some(action) = command_rx.recv() => {
                let bytes = bitcode::encode(&action);

                if let Err(err) = ws_writer.send(Message::Binary(Bytes::from(bytes))).await {
                    tracing::error!(?err, "failed to write to WebSocket (connection closed)");
                    break;
                }
            },
            // Wait for a shutdown signal from the reader task
            _ = shutdown_rx.changed() => {
                tracing::info!("shutdown signal received");
                break;
            }
        }
    }
    tracing::info!("writer task has terminated");
}
