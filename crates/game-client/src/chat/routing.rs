use bevy::prelude::*;
use bevy_renet::{RenetClient, renet::DefaultChannel};
use protocol::models::ChatChannel;
use web_client::{ChannelType, SocialAction};

use crate::web::SocialSender;

/// The target channel for an outgoing chat message.
#[derive(Debug, Clone)]
pub enum OutgoingChannel {
    Say,
    Yell,
    Zone,
    Guild,
    Party,
    Trade,
    Whisper { target_name: String },
}

/// Fired by the chat UI when the player submits a message.
#[derive(Event)]
pub struct OutgoingChatMessage {
    pub channel: OutgoingChannel,
    pub text: String,
}

pub fn route_outgoing_chat(
    event: On<OutgoingChatMessage>,
    mut renet_client: ResMut<RenetClient>,
    social_sender: Res<SocialSender>,
) {
    match &event.channel {
        OutgoingChannel::Say | OutgoingChannel::Yell | OutgoingChannel::Zone => {
            let chat_channel = match &event.channel {
                OutgoingChannel::Say => ChatChannel::Say,
                OutgoingChannel::Yell => ChatChannel::Yell,
                OutgoingChannel::Zone => ChatChannel::Zone,
                _ => unreachable!(),
            };
            let action = protocol::client::PlayerAction::Chat {
                channel: chat_channel,
                text: event.text.clone(),
            };
            let encoded = bitcode::encode(&action);
            renet_client.send_message(DefaultChannel::ReliableOrdered, encoded);
        }
        OutgoingChannel::Guild | OutgoingChannel::Party | OutgoingChannel::Trade => {
            let channel_type = match &event.channel {
                OutgoingChannel::Guild => ChannelType::Guild,
                OutgoingChannel::Party => ChannelType::Party,
                OutgoingChannel::Trade => ChannelType::Trade,
                _ => unreachable!(),
            };
            if let Some(ref sender) = social_sender.0 {
                let _ = sender.try_send(SocialAction::Chat {
                    channel: channel_type,
                    text: event.text.clone(),
                });
            }
        }
        OutgoingChannel::Whisper { target_name } => {
            if let Some(ref sender) = social_sender.0 {
                let _ = sender.try_send(SocialAction::WhisperByName {
                    recipient_name: target_name.clone(),
                    text: event.text.clone(),
                });
            }
        }
    }
}
