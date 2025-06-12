mod client;
mod syncer;

use axum::extract::ws::WebSocket;
use tokio::sync::mpsc::{channel, Sender};
use uuid::Uuid;

use crate::auth::User;

use self::client::Client;
pub use syncer::Syncer;

// NOTE: WebSocket message type flags
// The message type is appended to the end of each message, because pop() is more efficient than remove(0)
pub const MESSAGE_UPDATE: u8 = 0;
pub const MESSAGE_SYNC_STEP_1: u8 = 1;
pub const MESSAGE_SYNC_STEP_2: u8 = 2;
pub const MESSAGE_AWARENESS_UPDATE: u8 = 3;
pub const MESSAGE_GET_AWARENESS: u8 = 3;

pub async fn handle_socket(socket: WebSocket, user: User, doc_handle: Sender<Message>) {
    let (client_tx, client_rx) = channel(128);
    let client = Client::new(user, client_tx, doc_handle);

    client.run(socket, client_rx).await;
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect(Uuid, Sender<Vec<u8>>),
    Disconnect(Uuid),
    Update(Uuid, Vec<u8>),
    GetDiff(Uuid, Vec<u8>),
    UpdateAwareness(Uuid, Vec<u8>),
    GetAwareness(Uuid),
}
