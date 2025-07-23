use schemas::social::ChannelType;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub enum Recipient {
    Id(i32),
    Name(Arc<str>),
}

pub enum HubCommand {
    Connect {
        character_id: i32,
        character_name: String,
        guild_id: Option<i32>,
        tx: Sender<Vec<u8>>,
    },
    Whisper {
        sender_id: i32,
        recipient: Recipient,
        text: Arc<str>,
    },
    ChatMessage {
        sender_id: i32,
        channel: ChannelType,
        text: Arc<str>,
    },
    Disconnect {
        character_id: i32,
    },
}
