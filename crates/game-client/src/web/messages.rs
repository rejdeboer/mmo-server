use bevy::prelude::*;
use web_client::ChannelType;

/// A chat message received from the social WebSocket (guild, party, trade).
#[derive(Message)]
pub struct SocialChatMessage {
    pub channel: ChannelType,
    pub sender_name: String,
    pub text: String,
}

/// An incoming whisper from another player.
#[derive(Message)]
pub struct WhisperReceivedMessage {
    pub sender_name: String,
    pub text: String,
}

/// Confirmation that a whisper we sent was delivered.
#[derive(Message)]
pub struct WhisperSentMessage {
    pub recipient_name: String,
    pub text: String,
}

/// A system notification from the social server.
#[derive(Message)]
pub struct SystemNotificationMessage {
    pub text: String,
}

/// A party invite from another player.
#[derive(Message)]
pub struct PartyInviteMessage {
    pub from_id: i32,
    pub from_name: String,
}

/// Party membership updated (new members, leader change, etc).
#[derive(Message)]
pub struct PartyUpdateMessage {
    pub party_id: i32,
    pub leader_id: i32,
    pub members: Vec<protocol::social::PartyMember>,
}

/// The party has been disbanded.
#[derive(Message)]
pub struct PartyDisbandedMessage;
