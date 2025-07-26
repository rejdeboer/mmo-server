use futures_util::{StreamExt, stream::SplitSink};
use http::{
    Request,
    header::{self, HeaderValue},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use url::Url;

use crate::{action::SocialAction, event::SocialEvent, reader::run_reader_task};

type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub struct SocialClient {
    pub command_tx: mpsc::Sender<SocialAction>,
    pub event_rx: mpsc::Receiver<SocialEvent>,
}

impl SocialClient {
    pub async fn connect(
        server_url: &str,
        auth_token: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, mut command_rx) = mpsc::channel::<SocialAction>(32);
        let (event_tx, event_rx) = mpsc::channel::<SocialEvent>(32);

        let bearer_token = format!("Bearer {auth_token}");
        let auth_header = HeaderValue::from_str(&bearer_token)?;

        let request = Request::builder()
            .uri(server_url)
            .header("Host", Url::parse(server_url)?.host_str().unwrap())
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .header("Sec-WebSocket-Version", "13")
            .header(header::AUTHORIZATION, auth_header)
            .body(())?;

        let (shutdown_tx, shutdown_rx) = watch::channel(());

        let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;
        tracing::info!("WebSocket handshake with JWT auth successful");

        let (mut ws_writer, mut ws_reader) = ws_stream.split();

        tokio::spawn(async move {
            // ... writer loop ...
        });

        tokio::spawn(run_reader_task(ws_reader, event_tx, shutdown_tx));

        Ok(Self {
            command_tx,
            event_rx,
        })
    }
}
