use crate::{
    ApplicationState,
    auth::{AccountContext, CharacterContext, encode_jwt},
    error::ApiError,
    protocol::{encode_connect_token, generate_connect_token},
    telemetry::get_trace_parent,
};
use axum::{Extension, Json, extract::State, response::Result};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::instrument;

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
    let connect_token = generate_connect_token(
        ctx.account_id,
        payload.character_id,
        &state.netcode_private_key,
        server_addr,
        get_trace_parent(),
    )
    .map_err(|err| {
        tracing::error!(?err, "failed to generate connect token");
        ApiError::UnexpectedError
    })?;
    let connect_token = encode_connect_token(connect_token).map_err(|error| {
        tracing::error!(?error, "failed to encode connect token");
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

    Ok(Json(GameEntryResponse { connect_token, jwt }))
}
