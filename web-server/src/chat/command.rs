use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub enum HubCommand {
    Connect {
        character_id: i32,
        character_name: String,
        guild_id: Option<i32>,
        tx: Sender<Vec<u8>>,
    },
    WhisperById {
        sender_id: i32,
        recipient_id: i32,
        text: Arc<str>,
    },
    Guild {
        sender_id: i32,
        text: Arc<str>,
    },
    Disconnect {
        character_id: i32,
    },
}
