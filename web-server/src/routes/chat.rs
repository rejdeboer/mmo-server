use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, Result},
};
use futures::StreamExt;
use tokio::sync::mpsc::{Sender, channel};

use crate::{
    ApplicationState,
    auth::CharacterContext,
    chat::{HubCommand, SocketReader, SocketWriter},
    error::ApiError,
};

pub async fn chat(
    ws: WebSocketUpgrade,
    State(state): State<ApplicationState>,
    Extension(ctx): Extension<CharacterContext>,
) -> Result<Response, ApiError> {
    let row = sqlx::query!(
        r#"
        SELECT name
        FROM characters
        WHERE id = $1 
        "#,
        ctx.character_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, ?ctx, "error fetching character");
        ApiError::UnexpectedError
    })?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, ctx, row.name, state.chat_handle)))
}

async fn handle_socket(
    socket: WebSocket,
    ctx: CharacterContext,
    character_name: String,
    hub_tx: Sender<HubCommand>,
) {
    let (client_tx, hub_rx) = channel::<Vec<u8>>(128);
    hub_tx
        .send(HubCommand::Connect {
            character_id: ctx.character_id,
            character_name,
            tx: client_tx,
        })
        .await
        .expect("client connects to hub");

    let (socket_tx, socket_rx) = socket.split();

    let writer = SocketWriter::new(socket_tx, hub_rx);
    let reader = SocketReader::new(socket_rx, hub_tx);

    tokio::spawn(async move {
        writer.run().await;
    });

    reader.run();
}
