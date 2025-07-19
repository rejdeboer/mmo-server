use std::sync::Arc;

pub enum HubCommand {
    Whisper(WhisperMessage),
    Guild(GuildMessage),
}

pub struct WhisperMessage {
    pub author_id: i32,
    pub author_name: String,
    pub recipient_id: i32,
    pub text: Arc<str>,
}

pub struct GuildMessage {
    pub author_id: i32,
    pub author_name: String,
    pub text: Arc<str>,
}
