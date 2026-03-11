use sqlx::{Pool, Postgres};
use tracing::instrument;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub rotation_yaw: f32,
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub guild_id: Option<i32>,
}

#[instrument(skip_all)]
pub async fn load_character_data(
    pool: Pool<Postgres>,
    character_id: i32,
) -> Result<CharacterRow, sqlx::Error> {
    sqlx::query_as!(
        CharacterRow,
        r#"
        SELECT id, guild_id, name, level, hp, max_hp,
            position_x, position_y, position_z,
            rotation_yaw
        FROM characters
        WHERE id = $1 
        "#,
        character_id,
    )
    .fetch_one(&pool)
    .await
}
