use axum::{
    extract::State,
    response::{Response, Result},
    Extension, Json,
};

use crate::{auth::User, error::ApiError, ApplicationState};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub level: i32,
    pub experience: i64,
}

pub async fn character_post(
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
) -> Result<Response, ApiError> {
}

pub async fn character_list(
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<CharacterRow>>, ApiError> {
    let rows = sqlx::query_as!(
        CharacterRow,
        r#"
        SELECT id, name, level, experience
        FROM characters
        WHERE account_id = $1 
        "#,
        user.account_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, ?user, "error fetching characters");
        ApiError::UnexpectedError
    })?;

    Ok(Json(rows))
}
