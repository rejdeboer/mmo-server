use flatbuffers::{InvalidFlatbuffer, root};
use schema::ChannelType;
use schemas::social as schema;
use tokio_tungstenite::tungstenite::Bytes;

#[derive(Debug)]
pub enum SocialEvent {
    Chat {
        channel: ChannelType,
        text: String,
        sender_name: String,
        sender_id: i32,
    },
    Whisper {
        text: String,
        sender_name: String,
        sender_id: i32,
    },
    WhisperReceipt {
        text: String,
        recipient_name: String,
        recipient_id: i32,
    },
}

impl SocialEvent {
    pub fn deserialize(bytes: Bytes) -> Result<SocialEvent, InvalidFlatbuffer> {
        let event = root::<schema::Event>(&bytes)?;
        match event.data_type() {
            schema::EventData::ServerChatMessage => {
                let fb_event = event
                    .data_as_server_chat_message()
                    .expect("event should be some");
                Ok(Self::Chat {
                    channel: fb_event.channel(),
                    sender_name: fb_event.sender_name().to_string(),
                    sender_id: fb_event.sender_id(),
                    text: fb_event.text().to_string(),
                })
            }
            schema::EventData::ServerWhisper => {
                let fb_event = event
                    .data_as_server_whisper()
                    .expect("event should be some");
                Ok(Self::Whisper {
                    sender_name: fb_event.sender_name().to_string(),
                    sender_id: fb_event.sender_id(),
                    text: fb_event.text().to_string(),
                })
            }
            schema::EventData::ServerWhisperReceipt => {
                let fb_event = event
                    .data_as_server_whisper_receipt()
                    .expect("event should be some");
                Ok(Self::WhisperReceipt {
                    recipient_name: fb_event.recipient_name().to_string(),
                    recipient_id: fb_event.recipient_id(),
                    text: fb_event.text().to_string(),
                })
            }
            event_type => {
                todo!("handle event type: {:?}", event_type);
            }
        }
    }
}
