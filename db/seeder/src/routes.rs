use crate::server::ApplicationState;
use crate::{error::ApiError, seed_db};
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct SeedParameters {
    pub count: usize,
}

pub async fn seed_route(
    State(state): State<ApplicationState>,
    Json(payload): Json<SeedParameters>,
) -> Result<(), ApiError> {
    seed_db(state.pool.clone(), payload.count)
        .await
        .map_err(|err| {
            tracing::error!(?err, "failed to seed db");
            ApiError::UnexpectedError
        })
}
