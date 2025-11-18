use axum::{Extension, Json, extract::State, response::Result};
use flatbuffers::FlatBufferBuilder;
use renetcode::{ConnectToken, NETCODE_USER_DATA_BYTES};
use schemas::protocol::{TokenUserData, TokenUserDataArgs};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::instrument;

use crate::{
    ApplicationState,
    auth::{AccountContext, CharacterContext, encode_jwt},
    configuration::NetcodePrivateKey,
    error::ApiError,
    telemetry::get_trace_parent,
};

#[derive(Serialize, Deserialize)]
pub struct GameEntryRequest {
    pub character_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct GameEntryResponse {
    pub connect_token: String,
    pub jwt: String,
}

#[instrument(skip(state, payload))]
pub async fn game_entry(
    State(state): State<ApplicationState>,
    Extension(ctx): Extension<AccountContext>,
    Json(payload): Json<GameEntryRequest>,
) -> Result<Json<GameEntryResponse>, ApiError> {
    let has_character = sqlx::query!(
        r#"
        SELECT EXISTS(SELECT 1 FROM characters WHERE id = $1 AND account_id = $2)
        "#,
        payload.character_id,
        ctx.account_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, "failed to check character ownership");
        ApiError::UnexpectedError
    })?;

    if !has_character.exists.unwrap_or(false) {
        tracing::info!("user does not have character");
        return Err(ApiError::BadRequest(
            "user does not have character".to_string(),
        ));
    }

    // TODO: Use more than 1 realm
    let server_addr = state.realm_resolver.resolve("main").await?;

    let mut token_buffer: Vec<u8> = vec![];
    let connect_token = generate_connect_token(
        ctx.account_id,
        payload.character_id,
        state.netcode_private_key,
        server_addr,
        get_trace_parent(),
    )?;
    connect_token.write(&mut token_buffer).map_err(|error| {
        tracing::error!(?error, "failed to write netcode token to buffer");
        ApiError::UnexpectedError
    })?;

    let character_ctx = CharacterContext {
        account_id: ctx.account_id,
        username: ctx.username,
        character_id: payload.character_id,
    };
    let jwt = encode_jwt(character_ctx, state.jwt_signing_key.expose_secret()).map_err(|err| {
        tracing::error!(?err, "failed to encode character JWT");
        ApiError::UnexpectedError
    })?;

    Ok(Json(GameEntryResponse {
        connect_token: base64::encode_config(token_buffer, base64::STANDARD),
        jwt,
    }))
}

// TODO: These parameters are arbitrary for now
fn generate_connect_token(
    account_id: i32,
    character_id: i32,
    private_key: NetcodePrivateKey,
    server_addr: SocketAddr,
    traceparent: Option<String>,
) -> Result<ConnectToken, ApiError> {
    let public_addresses: Vec<SocketAddr> = vec![server_addr];

    let mut builder = FlatBufferBuilder::new();

    let traceparent = traceparent.map(|v| builder.create_string(&v));
    if traceparent.is_none() {
        tracing::warn!("no traceparent present");
    }

    let response_offset = TokenUserData::create(
        &mut builder,
        &TokenUserDataArgs {
            character_id,
            traceparent,
        },
    );
    builder.finish_minimal(response_offset);

    let mut user_data: [u8; NETCODE_USER_DATA_BYTES] = [0; NETCODE_USER_DATA_BYTES];
    let copy_data = builder.finished_data();
    user_data[0..copy_data.len()].copy_from_slice(copy_data);

    let token = ConnectToken::generate(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        0,
        300,
        account_id as u64,
        15,
        public_addresses,
        Some(&user_data),
        private_key.as_ref(),
    )
    .map_err(|error| {
        tracing::error!(?error, "failed to generate netcode token");
        ApiError::UnexpectedError
    })?;

    Ok(token)
}
