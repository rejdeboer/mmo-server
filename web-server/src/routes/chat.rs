use std::ops::ControlFlow;

use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::{Response, Result},
};

use crate::{ApplicationState, auth::CharacterContext, error::ApiError};

pub struct ChatContext {
    pub account_id: i32,
    pub username: String,
    pub character_id: i32,
    pub character_name: String,
}

pub struct Client {
    pub ctx: ChatContext,
    pub socket: WebSocket,
}

impl Client {
    pub fn new(ctx: ChatContext, socket: WebSocket) -> Self {
        Self { ctx, socket }
    }

    pub async fn run(mut self) {
        while let Some(Ok(msg)) = self.socket.recv().await {
            if self.read_message(msg).is_break() {
                break;
            }
        }
    }

    fn read_message(&mut self, msg: Message) -> ControlFlow<(), ()> {
        match msg {
            Message::Text(text) => {
                tracing::warn!(?text, "received text message");
            }
            Message::Binary(_) => {
                todo!("read flatbuffer");
            }
            Message::Ping(_) => {
                tracing::info!("Received ping");
            }
            Message::Pong(_) => {
                tracing::info!("Received pong");
            }
            Message::Close(_) => {
                tracing::info!("Client disconnected.");
                return ControlFlow::Break(());
            }
        };
        ControlFlow::Continue(())
    }
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

async fn handle_socket(socket: WebSocket, ctx: ChatContext) {
    let client = Client::new(ctx, socket);
    client.run().await;
}
