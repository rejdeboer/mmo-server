use flatbuffers::InvalidFlatbuffer;
use schemas::social::{ActionData, ChannelType};
use tokio::sync::mpsc::error::SendError;

use crate::social::HubCommand;

#[derive(Debug)]
pub enum ReaderError {
    InvalidSchema(InvalidFlatbuffer),
    InvalidActionType(ActionData),
    HubSendFailure(SendError<HubCommand>),
}

#[derive(Debug)]
pub enum ChatSendError {
    RecipientNotOnline,
    SenderNotInGuild,
}
