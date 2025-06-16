use std::{
    net::{IpAddr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::State, response::Result, Extension, Json};
use renetcode::ConnectToken;
use serde::{Deserialize, Serialize};

use crate::{auth::User, error::ApiError, ApplicationState};

#[derive(Deserialize)]
pub struct GameEntryRequest {
    pub character_id: i32,
}

#[derive(Serialize)]
pub struct GameEntryResponse {
    pub token: String,
}

pub async fn game_entry(
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
    Json(payload): Json<GameEntryRequest>,
) -> Result<Json<GameEntryResponse>, ApiError> {
    sqlx::query!(
        r#"
        SELECT EXISTS(SELECT 1 FROM characters WHERE id = $1 AND account_id = $2)
        "#,
        payload.character_id,
        user.account_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, ?user, "user does not have character");
        ApiError::BadRequest("user does not have character".to_string())
    })?;

    let mut token_buffer: Vec<u8> = vec![0; 1024];
    let connect_token = generate_connect_token(
        user.account_id,
        payload.character_id,
        state.netcode_private_key.as_ref(),
    )
    .map_err(|error| {
        tracing::error!(?error, ?user, "failed to generate netcode token");
        ApiError::UnexpectedError
    })?;
    connect_token.write(&mut token_buffer).map_err(|error| {
        tracing::error!(?error, ?user, "failed to write netcodet token to buffer");
        ApiError::UnexpectedError
    })?;
    let token = base64::encode(token_buffer);

    Ok(Json(GameEntryResponse { token }))
}

// TODO: These parameters are arbitrary for now
fn generate_connect_token(
    account_id: i32,
    character_id: i32,
    private_key: &[u8; 32],
) -> Result<ConnectToken, renetcode::TokenGenerationError> {
    let ip_addr = IpAddr::V4("127.0.0.1".parse().expect("host should be IPV4 addr"));
    let server_addr: SocketAddr = SocketAddr::new(ip_addr, 8000);
    let mut public_addresses: Vec<SocketAddr> = Vec::new();
    public_addresses.push(server_addr);

    ConnectToken::generate(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        0,
        300,
        account_id as u64,
        15,
        public_addresses,
        None,
        private_key,
    )
}
