use axum::{
    Extension,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, Result},
};

use crate::{ApplicationState, auth::CharacterContext, error::ApiError};

struct ChatContext {
    pub account_id: i32,
    pub username: String,
    pub character_id: i32,
    pub character_name: String,
}

impl ChatContext {
    pub fn new(character_ctx: CharacterContext, character_name: String) -> Self {
        Self {
            account_id: character_ctx.account_id,
            username: character_ctx.username,
            character_id: character_ctx.character_id,
            character_name,
        }
    }
}

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

async fn handle_socket(socket: WebSocket, ctx: ChatContext) {}
