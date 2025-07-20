use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub enum HubCommand {
    Connect {
        character_id: i32,
        character_name: String,
        tx: Sender<Vec<u8>>,
    },
    Whisper {
        recipient_id: i32,
        text: Arc<str>,
    },
    Guild {
        text: Arc<str>,
    },
}
