use flatbuffers::InvalidFlatbuffer;
use schemas::social::ActionData;
use tokio::sync::mpsc::error::SendError;

use crate::social::HubMessage;

#[derive(Debug)]
pub enum ReaderError {
    InvalidSchema(InvalidFlatbuffer),
    InvalidActionType(ActionData),
    HubSendFailure(SendError<HubMessage>),
}

#[derive(Debug)]
pub enum HubError {
    RecipientNotFound,
    SenderNotInGuild,
    Unexpected,
}
