use crate::ApplicationState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};

pub async fn health(State(state): State<ApplicationState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            tracing::error!(?err, "health check failed: database unreachable");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
