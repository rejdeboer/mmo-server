use tokio::sync::mpsc::error::SendError;

use crate::social::HubMessage;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ReaderError {
    InvalidPayload(String),
    HubSendFailure(SendError<HubMessage>),
}

#[derive(Debug)]
pub enum HubError {
    RecipientNotFound,
    SenderNotInGuild,
    RateLimited,
    TargetAlreadyInParty,
    NoPendingInvite,
    NotInParty,
    NotPartyLeader,
    Unexpected,
}
