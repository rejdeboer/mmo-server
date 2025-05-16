use crate::MAX_PAYLOAD_BYTES;
use crate::channel_packet::SerializationError;
use crate::token::TokenGenerationError;
use chacha20poly1305::aead::Error as CryptoError;
use std::{error, fmt};

/// Possible reasons for a disconnection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Connection was terminated by the transport layer
    Transport,
    /// Connection was terminated by the server
    DisconnectedByClient,
    /// Connection was terminated by the server
    DisconnectedByServer,
    /// Failed to serialize packet
    PacketSerialization(SerializationError),
    /// Failed to deserialize packet
    PacketDeserialization(SerializationError),
    /// Received message from channel with invalid id
    ReceivedInvalidChannelId(u8),
    /// Error occurred in a send channel
    SendChannelError { channel_id: u8, error: ChannelError },
    /// Error occurred in a receive channel
    ReceiveChannelError { channel_id: u8, error: ChannelError },
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DisconnectReason::*;

        match *self {
            ConnectTokenExpired => write!(f, "connection token has expired"),
            ConnectionTimedOut => write!(f, "connection timed out"),
            ConnectionResponseTimedOut => write!(f, "connection timed out during response step"),
            ConnectionRequestTimedOut => write!(f, "connection timed out during request step"),
            ConnectionDenied => write!(f, "server denied connection"),
            DisconnectedByClient => write!(f, "connection terminated by client"),
            DisconnectedByServer => write!(f, "connection terminated by server"),
        }
    }
}

impl error::Error for DisconnectReason {}

#[derive(Debug)]
pub enum NetcodeError {
    /// No private keys was available while decrypting.
    UnavailablePrivateKey,
    /// The type of the packet is invalid.
    InvalidPacketType,
    /// The connect token has an invalid protocol id.
    InvalidProtocolID,
    /// The connect token has an invalid version.
    InvalidVersion,
    /// Packet size is too small to be a netcode packet.
    PacketTooSmall,
    /// Payload is above the maximum limit
    PayloadAboveLimit,
    /// The processed packet is duplicated
    DuplicatedSequence,
    /// No more host are available in the connect token..
    NoMoreServers,
    /// The connect token has expired.
    Expired,
    /// The client is disconnected.
    Disconnected(crate::netcode_client::DisconnectReason),
    /// An error ocurred while encrypting or decrypting.
    CryptoError,
    /// The server address is not in the connect token.
    NotInHostList,
    /// Client was not found.
    ClientNotFound,
    /// Client is not connected.
    ClientNotConnected,
    // IO error.
    IoError(std::io::Error),
    // An error occured while generating the connect token.
    TokenGenerationError(TokenGenerationError),
}

impl fmt::Display for NetcodeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use NetcodeError::*;

        match *self {
            UnavailablePrivateKey => write!(fmt, "no private key was found for this address"),
            InvalidPacketType => write!(fmt, "invalid packet type"),
            InvalidProtocolID => write!(fmt, "invalid protocol id"),
            InvalidVersion => write!(fmt, "invalid version info"),
            PacketTooSmall => write!(fmt, "packet is too small"),
            PayloadAboveLimit => write!(
                fmt,
                "payload is above the {} bytes limit",
                MAX_PAYLOAD_BYTES
            ),
            Expired => write!(fmt, "connection expired"),
            DuplicatedSequence => write!(fmt, "sequence already received"),
            Disconnected(reason) => write!(fmt, "disconnected: {}", reason),
            NoMoreServers => write!(fmt, "client has no more servers to connect"),
            CryptoError => write!(fmt, "error while encoding or decoding"),
            NotInHostList => write!(fmt, "token does not contain the server address"),
            ClientNotFound => write!(fmt, "client was not found"),
            ClientNotConnected => write!(fmt, "client is disconnected or connecting"),
            IoError(ref err) => write!(fmt, "{}", err),
            TokenGenerationError(ref err) => write!(fmt, "{}", err),
        }
    }
}

impl error::Error for NetcodeError {}

impl From<std::io::Error> for NetcodeError {
    fn from(inner: std::io::Error) -> Self {
        NetcodeError::IoError(inner)
    }
}

impl From<TokenGenerationError> for NetcodeError {
    fn from(inner: TokenGenerationError) -> Self {
        NetcodeError::TokenGenerationError(inner)
    }
}

impl From<CryptoError> for NetcodeError {
    fn from(_: CryptoError) -> Self {
        NetcodeError::CryptoError
    }
}

#[derive(Debug)]
pub enum NetcodeTransportError {
    Netcode(NetcodeError),
    Renet(DisconnectReason),
    IO(std::io::Error),
}

impl error::Error for NetcodeTransportError {}

impl fmt::Display for NetcodeTransportError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NetcodeTransportError::Netcode(ref err) => err.fmt(fmt),
            NetcodeTransportError::Renet(ref err) => err.fmt(fmt),
            NetcodeTransportError::IO(ref err) => err.fmt(fmt),
        }
    }
}

impl From<NetcodeError> for NetcodeTransportError {
    fn from(inner: NetcodeError) -> Self {
        NetcodeTransportError::Netcode(inner)
    }
}

impl From<TokenGenerationError> for NetcodeTransportError {
    fn from(inner: TokenGenerationError) -> Self {
        NetcodeTransportError::Netcode(NetcodeError::TokenGenerationError(inner))
    }
}

impl From<DisconnectReason> for NetcodeTransportError {
    fn from(inner: DisconnectReason) -> Self {
        NetcodeTransportError::Renet(inner)
    }
}

impl From<std::io::Error> for NetcodeTransportError {
    fn from(inner: std::io::Error) -> Self {
        NetcodeTransportError::IO(inner)
    }
}

/// Possibles errors that can occur in a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelError {
    /// Reliable channel reached maximum allowed memory
    ReliableChannelMaxMemoryReached,
    /// Received an invalid slice message in the channel.
    InvalidSliceMessage,
}

impl fmt::Display for ChannelError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use ChannelError::*;

        match *self {
            ReliableChannelMaxMemoryReached => {
                write!(fmt, "reliable channel memory usage was exausted")
            }
            InvalidSliceMessage => write!(fmt, "received an invalid slice packet"),
        }
    }
}

impl std::error::Error for ChannelError {}

#[derive(Debug)]
pub struct ClientNotFound;

impl std::error::Error for ClientNotFound {}

impl fmt::Display for ClientNotFound {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "client with given id was not found")
    }
}
