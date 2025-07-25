use std::sync::Arc;

use schemas::social::ChannelType;
use tokio::sync::mpsc::Sender;

pub enum Recipient {
    Id(i32),
    Name(String),
}

pub struct HubMessage {
    pub sender_id: i32,
    pub command: HubCommand,
}

impl HubMessage {
    pub fn new(sender_id: i32, command: HubCommand) -> Self {
        Self { sender_id, command }
    }
}

pub enum HubCommand {
    Connect {
        character_name: String,
        guild_id: Option<i32>,
        tx: Sender<Arc<[u8]>>,
    },
    Whisper {
        recipient: Recipient,
        text: String,
    },
    ChatMessage {
        channel: ChannelType,
        text: String,
    },
    Disconnect,
}
