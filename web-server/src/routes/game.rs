use std::{
    net::{IpAddr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{Extension, Json, extract::State, response::Result};
use flatbuffers::FlatBufferBuilder;
use renetcode::{ConnectToken, NETCODE_USER_DATA_BYTES};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{ApplicationState, auth::User, configuration::GameServerSettings, error::ApiError};

#[derive(Serialize, Deserialize)]
pub struct GameEntryRequest {
    pub character_id: i32,
}

#[derive(Serialize)]
pub struct GameEntryResponse {
    pub token: String,
}

#[instrument(skip(state, payload))]
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
        tracing::error!(?error, "user does not have character");
        ApiError::BadRequest("user does not have character".to_string())
    })?;

    let mut token_buffer: Vec<u8> = vec![0; 1024];
    let connect_token = generate_connect_token(
        user.account_id,
        payload.character_id,
        state.game_server_settings,
    )?;
    connect_token.write(&mut token_buffer).map_err(|error| {
        tracing::error!(?error, "failed to write netcode token to buffer");
        ApiError::UnexpectedError
    })?;
    let token = base64::encode_config(token_buffer, base64::STANDARD);

    Ok(Json(GameEntryResponse { token }))
}

// TODO: These parameters are arbitrary for now
fn generate_connect_token(
    account_id: i32,
    character_id: i32,
    game_server_settings: GameServerSettings,
) -> Result<ConnectToken, ApiError> {
    let ip_addr = IpAddr::V4(
        game_server_settings
            .host
            .parse()
            .expect("host should be IPV4 addr"),
    );
    let server_addr: SocketAddr = SocketAddr::new(ip_addr, game_server_settings.port);
    let mut public_addresses: Vec<SocketAddr> = Vec::new();
    public_addresses.push(server_addr);

    let mut builder = FlatBufferBuilder::new();
    let response_offset = schemas::mmo::NetcodeTokenUserData::create(
        &mut builder,
        &schemas::mmo::NetcodeTokenUserDataArgs { character_id },
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
        game_server_settings.netcode_private_key.as_ref(),
    )
    .map_err(|error| {
        tracing::error!(?error, "failed to generate netcode token");
        ApiError::UnexpectedError
    })?;

    Ok(token)
}
