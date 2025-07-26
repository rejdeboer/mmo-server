use flatbuffers::{FlatBufferBuilder, WIPOffset};
use schema::ChannelType;
use schemas::social as schema;

#[derive(Debug)]
pub enum SocialAction {
    WhisperByName {
        recipient_name: String,
        text: String,
    },
    Chat {
        channel: ChannelType,
        text: String,
    },
}

impl SocialAction {
    pub fn encode<'a>(&self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Action<'a>> {
        let data_type;
        let data = match self {
            Self::Chat { channel, text } => {
                data_type = schema::ActionData::ClientChatMessage;
                let fb_msg = builder.create_string(text);
                schema::ClientChatMessage::create(
                    builder,
                    &schema::ClientChatMessageArgs {
                        channel: *channel,
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
            Self::WhisperByName {
                recipient_name,
                text,
            } => {
                data_type = schema::ActionData::ClientWhisperByName;
                let fb_recipient = builder.create_string(recipient_name);
                let fb_msg = builder.create_string(text);
                schema::ClientWhisperByName::create(
                    builder,
                    &schema::ClientWhisperByNameArgs {
                        recipient_name: Some(fb_recipient),
                        text: Some(fb_msg),
                    },
                )
                .as_union_value()
            }
        };

        schema::Action::create(
            builder,
            &schema::ActionArgs {
                data_type,
                data: Some(data),
            },
        )
    }
}
