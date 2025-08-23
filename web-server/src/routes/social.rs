use crate::{
    ApplicationState,
    auth::CharacterContext,
    error::ApiError,
    social::{HubCommand, HubMessage, SocketReader, SocketWriter},
    telemetry::ACTIVE_WS_CONNECTIONS,
};
use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, Result},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, channel};

pub async fn social(
    ws: WebSocketUpgrade,
    State(state): State<ApplicationState>,
    Extension(ctx): Extension<CharacterContext>,
) -> Result<Response, ApiError> {
    let row = sqlx::query!(
        r#"
        SELECT name, guild_id
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

    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, ctx, row.name, row.guild_id, state.hub_tx)))
}

async fn handle_socket(
    socket: WebSocket,
    ctx: CharacterContext,
    character_name: String,
    guild_id: Option<i32>,
    hub_tx: Sender<HubMessage>,
) {
    let (client_tx, hub_rx) = channel::<Arc<[u8]>>(128);

    let cmd = HubCommand::Connect {
        character_name,
        guild_id,
        tx: client_tx,
    };

    hub_tx
        .send(HubMessage::new(ctx.character_id, cmd))
        .await
        .expect("client connects to hub");

    let (socket_tx, socket_rx) = socket.split();

    let writer = SocketWriter::new(socket_tx, hub_rx);
    let reader = SocketReader::new(ctx.character_id, socket_rx, hub_tx);

    tokio::spawn(async move {
        writer.run().await;
    });

    ACTIVE_WS_CONNECTIONS.inc();
    reader.run().await;
    ACTIVE_WS_CONNECTIONS.dec();
}
