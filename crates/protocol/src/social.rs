use bitcode::{Decode, Encode};

/// Channel type for chat messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum ChannelType {
    Guild,
    Party,
    Trade,
}

/// Actions sent from client to server over the social WebSocket
#[derive(Debug, Clone, Encode, Decode)]
pub enum SocialAction {
    Chat {
        channel: ChannelType,
        text: String,
    },
    WhisperByName {
        recipient_name: String,
        text: String,
    },
    WhisperById {
        recipient_id: i32,
        text: String,
    },
    PartyInviteById {
        target_id: i32,
    },
    PartyInviteByName {
        target_name: String,
    },
    PartyAccept,
    PartyDecline,
    PartyLeave,
    PartyKick {
        target_id: i32,
    },
}

/// Events sent from server to client over the social WebSocket
#[derive(Debug, Clone, Encode, Decode)]
pub enum SocialEvent {
    Chat {
        channel: ChannelType,
        sender_id: i32,
        sender_name: String,
        text: String,
    },
    Whisper {
        sender_id: i32,
        sender_name: String,
        text: String,
    },
    WhisperReceipt {
        recipient_id: i32,
        recipient_name: String,
        text: String,
    },
    SystemMessage {
        text: String,
    },
    Error {
        message: String,
    },
    PartyInvite {
        from_id: i32,
        from_name: String,
    },
    PartyUpdate {
        party_id: i32,
        leader_id: i32,
        members: Vec<PartyMember>,
    },
    PartyDisbanded,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct PartyMember {
    pub character_id: i32,
    pub character_name: String,
}
