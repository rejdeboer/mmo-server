use futures_util::StreamExt;
use http::{
    Request, StatusCode,
    header::{self, HeaderValue},
};
use tokio::sync::{mpsc, watch};
use url::Url;

use crate::{
    action::SocialAction, event::SocialEvent, reader::run_reader_task, writer::run_writer_task,
};

pub type ConnectionResult =
    Result<(mpsc::Sender<SocialAction>, mpsc::Receiver<SocialEvent>), ConnectionError>;

#[derive(Debug)]
pub enum ConnectionError {
    InvalidUrl,
    InvalidTokenFormat,
    // TODO: Send back error in json?
    Http(StatusCode),
    WebSocket(tokio_tungstenite::tungstenite::Error),
}

pub async fn connect(server_url: &str, auth_token: &str) -> ConnectionResult {
    let request = create_connection_request(server_url, auth_token)?;

    let (ws_stream, _) =
        tokio_tungstenite::connect_async(request)
            .await
            .map_err(|err| match err {
                tokio_tungstenite::tungstenite::Error::Http(res) => {
                    ConnectionError::Http(res.status())
                }
                // err => unreachable!("encountered unreachable error: {:?}", err),
                err => ConnectionError::WebSocket(err),
            })?;
    tracing::info!("WebSocket handshake with JWT auth successful");

    let (ws_writer, ws_reader) = ws_stream.split();
    let (action_tx, action_rx) = mpsc::channel::<SocialAction>(32);
    let (event_tx, event_rx) = mpsc::channel::<SocialEvent>(32);
    let (shutdown_tx, shutdown_rx) = watch::channel(());

    tokio::spawn(run_writer_task(ws_writer, action_rx, shutdown_rx));
    tokio::spawn(run_reader_task(ws_reader, event_tx, shutdown_tx));

    Ok((action_tx, event_rx))
}

fn create_connection_request(
    server_url: &str,
    auth_token: &str,
) -> Result<Request<()>, ConnectionError> {
    let bearer_token = format!("Bearer {auth_token}");
    let auth_header =
        HeaderValue::from_str(&bearer_token).map_err(|_| ConnectionError::InvalidTokenFormat)?;

    let url = Url::parse(server_url).map_err(|_| ConnectionError::InvalidUrl)?;
    let request = Request::builder()
        .uri(server_url)
        .header("Host", url.host_str().unwrap())
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .header("Sec-WebSocket-Version", "13")
        .header(header::AUTHORIZATION, auth_header)
        .body(())
        .map_err(|_| ConnectionError::InvalidTokenFormat)?;

    Ok(request)
}
