use bevy::prelude::*;
use web_client::ChannelType;

use crate::web::{SocialChatMessage, SystemNotificationMessage, WhisperReceivedMessage, WhisperSentMessage};
use super::channels::{ChatLog, ChatMessage, ChatMessageChannel};

pub fn handle_social_chat(
    mut reader: MessageReader<SocialChatMessage>,
    mut chat_log: ResMut<ChatLog>,
) {
    for msg in reader.read() {
        let channel = match msg.channel {
            ChannelType::Guild => ChatMessageChannel::Guild,
            ChannelType::Party => ChatMessageChannel::Party,
            ChannelType::Trade => ChatMessageChannel::Trade,
        };
        chat_log.push(ChatMessage {
            channel,
            sender: msg.sender_name.clone(),
            text: msg.text.clone(),
        });
    }
}

pub fn handle_whisper_received(
    mut reader: MessageReader<WhisperReceivedMessage>,
    mut chat_log: ResMut<ChatLog>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for msg in reader.read() {
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::Whisper,
            sender: msg.sender_name.clone(),
            text: msg.text.clone(),
        });

        commands.spawn(AudioPlayer::new(
            asset_server.load("sounds/whisper-received.ogg"),
        ));
    }
}

pub fn handle_whisper_sent(
    mut reader: MessageReader<WhisperSentMessage>,
    mut chat_log: ResMut<ChatLog>,
) {
    for msg in reader.read() {
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::WhisperSent,
            sender: msg.recipient_name.clone(),
            text: msg.text.clone(),
        });
    }
}

pub fn handle_system_notification(
    mut reader: MessageReader<SystemNotificationMessage>,
    mut chat_log: ResMut<ChatLog>,
) {
    for msg in reader.read() {
        chat_log.push(ChatMessage {
            channel: ChatMessageChannel::System,
            sender: String::new(),
            text: msg.text.clone(),
        });
    }
}
