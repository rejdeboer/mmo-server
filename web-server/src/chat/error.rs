use flatbuffers::InvalidFlatbuffer;
use schemas::social::ChannelType;
use tokio::sync::mpsc::error::SendError;

use crate::chat::HubCommand;

#[derive(Debug)]
pub enum ChatReceiveError {
    InvalidSchema(InvalidFlatbuffer),
    InvalidChannel(ChannelType),
    HubSendFailure(SendError<HubCommand>),
}

#[derive(Debug)]
pub enum ChatSendError {
    RecipientNotOnline,
    SenderNotInGuild,
}
