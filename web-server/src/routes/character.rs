#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i32,
    pub name: String,
    pub level: i32,
    pub experience: i64,
}
