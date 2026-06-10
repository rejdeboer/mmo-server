use crate::{
    ApplicationState,
    auth::CharacterContext,
    error::ApiError,
    social::{HubCommand, HubMessage, SocketReader, SocketWriter},
};
use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, Result},
};
use futures::StreamExt;
use metrics::{gauge, histogram};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{Sender, channel};
use tracing::Instrument;

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

    Ok(ws.on_upgrade(move |socket| {
        let span = tracing::info_span!(
            "websocket_connection",
            character_id = ctx.character_id,
            character_name = %row.name,
        );
        handle_socket(socket, ctx, row.name, row.guild_id, state.hub_tx).instrument(span)
    }))
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

    let _guard = ConnectionGuard::new();
    tokio::spawn(async move {
        writer.run().await;
    });

    reader.run().await;
}

struct ConnectionGuard {
    connected_at: Instant,
}

impl ConnectionGuard {
    fn new() -> Self {
        gauge!("social_connections_active").increment(1.0);
        Self {
            connected_at: Instant::now(),
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        gauge!("social_connections_active").decrement(1.0);
        let duration = self.connected_at.elapsed().as_secs_f64();
        histogram!("social_connection_duration_seconds").record(duration);
    }
}
