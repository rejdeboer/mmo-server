use crate::{ApplicationState, auth::AccountContext, domain::CharacterName, error::ApiError};
use axum::{Extension, Json, extract::State, response::Result};
use sqlx::PgPool;
use tracing::instrument;
use web_types::{Character, CharacterCreate};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub level: i32,
    pub experience: i64,
}

impl From<CharacterRow> for Character {
    fn from(value: CharacterRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            level: value.level,
            experience: value.experience,
        }
    }
}

const STARTING_SPELLS: &[i32] = &[3, 4];

// TODO: Implement more validation: character limits, etc...
#[instrument(skip_all, fields(name = payload.name))]
pub async fn character_create(
    State(state): State<ApplicationState>,
    Extension(ctx): Extension<AccountContext>,
    Json(payload): Json<CharacterCreate>,
) -> Result<Json<Character>, ApiError> {
    let name = CharacterName::parse(payload.name).map_err(ApiError::BadRequest)?;

    let row = create_character(&state.pool, name.as_ref(), ctx.account_id, None)
        .await
        .map_err(|error| {
            tracing::error!(?error, ?ctx, "error creating character");
            ApiError::UnexpectedError
        })?;

    Ok(Json(row.into()))
}

pub async fn character_list(
    State(state): State<ApplicationState>,
    Extension(ctx): Extension<AccountContext>,
) -> Result<Json<Vec<Character>>, ApiError> {
    let rows = sqlx::query_as!(
        CharacterRow,
        r#"
        SELECT id, name, level, experience
        FROM characters
        WHERE account_id = $1 
        "#,
        ctx.account_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, ?ctx, "error fetching characters");
        ApiError::UnexpectedError
    })?;

    let characters = rows.into_iter().map(Character::from).collect();
    Ok(Json(characters))
}

pub async fn create_character(
    pool: &PgPool,
    name: &str,
    account_id: i32,
    guild_id: Option<i32>,
) -> sqlx::Result<CharacterRow> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as!(
        CharacterRow,
        r#"
        INSERT INTO characters (name, account_id, guild_id)
        VALUES ($1, $2, $3)
        RETURNING id, name, level, experience
        "#,
        name,
        account_id,
        guild_id,
    )
    .fetch_one(&mut *tx)
    .await?;

    for &spell_id in STARTING_SPELLS {
        sqlx::query!(
            "INSERT INTO character_abilities (character_id, spell_id) VALUES ($1, $2)",
            row.id,
            spell_id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(row)
}
