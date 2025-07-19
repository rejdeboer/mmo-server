use flatbuffers::InvalidFlatbuffer;
use schemas::mmo::ChannelType;

#[derive(Debug)]
pub enum ChatClientError {
    DecodeError(InvalidFlatbuffer),
    InvalidChannel(ChannelType),
    Unexpected,
}
