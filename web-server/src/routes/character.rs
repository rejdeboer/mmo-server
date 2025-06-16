use axum::{extract::State, response::Result, Extension, Json};
use serde::{Deserialize, Serialize};

use crate::{auth::User, domain::CharacterName, error::ApiError, ApplicationState};

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub level: i32,
    pub experience: i64,
}

#[derive(Serialize, Deserialize)]
pub struct CharacterCreate {
    pub name: String,
}

// TODO: Implement more validation: character limits, etc...
pub async fn character_create(
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
    Json(payload): Json<CharacterCreate>,
) -> Result<Json<CharacterRow>, ApiError> {
    let name = CharacterName::parse(payload.name).map_err(ApiError::BadRequest)?;
    let row = sqlx::query_as!(
        CharacterRow,
        r#"
        INSERT INTO characters (name, account_id)
        VALUES ($1, $2)
        RETURNING id, name, level, experience
        "#,
        name.as_ref(),
        user.account_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, ?user, "error creating character");
        ApiError::UnexpectedError
    })?;

    Ok(Json(row))
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
