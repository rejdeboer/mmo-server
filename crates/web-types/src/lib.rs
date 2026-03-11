use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct AccountCreate {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Account {
    pub username: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameEntryRequest {
    pub character_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct GameEntryResponse {
    pub connect_token: String,
    pub jwt: String,
}

#[derive(Serialize, Deserialize)]
pub struct CharacterCreate {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: i32,
    pub name: String,
    pub level: i32,
    pub experience: i64,
}

#[derive(Deserialize, Serialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub jwt: String,
}
