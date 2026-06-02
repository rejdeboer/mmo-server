use futures_util::StreamExt;
use protocol::social::SocialEvent;
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

pub async fn run_reader_task(
    mut ws_reader: futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_tx: mpsc::Sender<SocialEvent>,
    shutdown_tx: watch::Sender<()>,
) {
    while let Some(msg_result) = ws_reader.next().await {
        let msg = match msg_result {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("failed to receive message from server: {}", e);
                break;
            }
        };

        if let Message::Binary(buf) = msg {
            let event = match bitcode::decode::<SocialEvent>(&buf) {
                Ok(e) => e,
                Err(err) => {
                    tracing::error!(?err, "failed to decode social event");
                    continue;
                }
            };

            if event_tx.send(event).await.is_err() {
                tracing::error!("failed to send event to application");
                break;
            }
        }
    }
    let _ = shutdown_tx.send(());
    tracing::info!("reader task has terminated");
}
