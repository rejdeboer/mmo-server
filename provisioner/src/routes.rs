use crate::server::ApplicationState;
use crate::{error::ApiError, seed_db};
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use web_server::protocol::{encode_connect_token, generate_connect_token};

#[derive(Clone, Deserialize, Serialize)]
pub struct SeedParameters {
    pub count: usize,
    pub server_addr: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SeedResult {
    pub tokens: Vec<String>,
}

pub async fn provision_route(
    State(state): State<ApplicationState>,
    Json(payload): Json<SeedParameters>,
) -> Result<Json<SeedResult>, ApiError> {
    seed_db(state.pool.clone(), payload.count)
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to seed db");
            ApiError::UnexpectedError
        })?;

    let server_addr = SocketAddr::from_str(&payload.server_addr).map_err(|err| {
        tracing::error!(?err, "failed to parse server addr");
        ApiError::BadRequest
    })?;

    let tokens = Vec::with_capacity(payload.count);
    for i in 1..=payload.count {
        let connect_token = generate_connect_token(
            i as i32,
            i as i32,
            &state.netcode_private_key,
            server_addr,
            None,
        )
        .map_err(|err| {
            tracing::error!(?err, "failed to generate connect token");
            ApiError::UnexpectedError
        })?;

        let token = encode_connect_token(connect_token).map_err(|error| {
            tracing::error!(?error, "failed to encode connect token");
            ApiError::UnexpectedError
        })?;

        tokens.push(token);
    }

    Ok(Json(SeedResult { tokens }))
}
