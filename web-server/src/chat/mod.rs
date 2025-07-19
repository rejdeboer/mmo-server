mod client;
mod error;
mod hub;
mod message;

pub use client::Client;

use crate::auth::CharacterContext;

pub struct ChatContext {
    pub account_id: i32,
    pub username: String,
    pub character_id: i32,
    pub character_name: String,
}

impl ChatContext {
    pub fn new(character_ctx: CharacterContext, character_name: String) -> Self {
        Self {
            account_id: character_ctx.account_id,
            username: character_ctx.username,
            character_id: character_ctx.character_id,
            character_name,
        }
    }
}
