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
    error::ApiError,
    social::{HubCommand, SocketReader, SocketWriter},
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

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, ctx, row.name, state.social_handle)))
}

async fn handle_socket(
    socket: WebSocket,
    ctx: CharacterContext,
    character_name: String,
    hub_tx: Sender<(i32, HubCommand)>,
) {
    let (client_tx, hub_rx) = channel::<Vec<u8>>(128);
    hub_tx
        .send(
            ctx.character_id,
            HubCommand::Connect {
                character_name,
                guild_id: None,
                tx: client_tx,
            },
        )
        .await
        .expect("client connects to hub");

    let (socket_tx, socket_rx) = socket.split();

    let writer = SocketWriter::new(socket_tx, hub_rx);
    let reader = SocketReader::new(ctx.character_id, socket_rx, hub_tx);

    tokio::spawn(async move {
        writer.run().await;
    });

    reader.run().await;
}
