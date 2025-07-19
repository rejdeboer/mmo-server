use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, Result},
};

use crate::{
    ApplicationState,
    auth::CharacterContext,
    chat::{ChatContext, Client},
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

    let chat_ctx = ChatContext::new(ctx, row.name);
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, chat_ctx)))
}

async fn handle_socket(socket: WebSocket, ctx: ChatContext) {
    let client = Client::new(ctx, socket);
    client.run().await;
}
