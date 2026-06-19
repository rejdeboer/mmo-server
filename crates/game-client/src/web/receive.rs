use bevy::{ecs::system::SystemParam, prelude::*};

use super::connection::SocialReceiver;
use super::messages::*;

/// Writers for dispatching social events as Bevy messages.
#[derive(SystemParam)]
pub struct SocialMessageWriters<'w> {
    pub chats: MessageWriter<'w, SocialChatMessage>,
    pub whispers_received: MessageWriter<'w, WhisperReceivedMessage>,
    pub whispers_sent: MessageWriter<'w, WhisperSentMessage>,
    pub system_notifications: MessageWriter<'w, SystemNotificationMessage>,
    pub party_invites: MessageWriter<'w, PartyInviteMessage>,
    pub party_updates: MessageWriter<'w, PartyUpdateMessage>,
    pub party_disbanded: MessageWriter<'w, PartyDisbandedMessage>,
}

/// Drains the social WebSocket receiver and dispatches events as Bevy messages.
pub fn receive_social_events(
    mut social_receiver: ResMut<SocialReceiver>,
    mut writers: SocialMessageWriters,
) {
    let Some(ref mut rx) = social_receiver.0 else {
        return;
    };

    while let Ok(event) = rx.try_recv() {
        match event {
            web_client::SocialEvent::Chat {
                channel,
                sender_name,
                text,
                ..
            } => {
                writers.chats.write(SocialChatMessage {
                    channel,
                    sender_name,
                    text,
                });
            }
            web_client::SocialEvent::Whisper {
                sender_name, text, ..
            } => {
                writers
                    .whispers_received
                    .write(WhisperReceivedMessage { sender_name, text });
            }
            web_client::SocialEvent::WhisperReceipt {
                recipient_name,
                text,
                ..
            } => {
                writers
                    .whispers_sent
                    .write(WhisperSentMessage { recipient_name, text });
            }
            web_client::SocialEvent::SystemMessage { text } => {
                writers
                    .system_notifications
                    .write(SystemNotificationMessage { text });
            }
            web_client::SocialEvent::Error { message } => {
                writers
                    .system_notifications
                    .write(SystemNotificationMessage { text: message });
            }
            web_client::SocialEvent::PartyInvite { from_id, from_name } => {
                writers
                    .party_invites
                    .write(PartyInviteMessage { from_id, from_name });
            }
            web_client::SocialEvent::PartyUpdate {
                party_id,
                leader_id,
                members,
            } => {
                writers.party_updates.write(PartyUpdateMessage {
                    party_id,
                    leader_id,
                    members,
                });
            }
            web_client::SocialEvent::PartyDisbanded => {
                writers.party_disbanded.write(PartyDisbandedMessage);
            }
        }
    }
}
