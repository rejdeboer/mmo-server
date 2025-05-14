//! Modified https://github.com/lucaspoffo/renet
mod crypto;
pub mod error;
pub mod packet;
mod serialize;
pub mod token;

const VERSION_INFO: &[u8; 13] = b"NETCODE 1.02\0";

/// The maximum number of bytes that a packet can contain
const MAX_PACKET_BYTES: usize = 1400;
/// The maximum number of bytes that a payload can have when generating a payload packet
const MAX_PAYLOAD_BYTES: usize = 1300;

const MAC_BYTES: usize = 16;
const CONNECT_TOKEN_XNONCE_BYTES: usize = 24;
const CHALLENGE_TOKEN_BYTES: usize = 300;
const CONNECT_TOKEN_PRIVATE_BYTES: usize = 1024;

const NETCODE_ADDRESS_NONE: u8 = 0;
const NETCODE_ADDRESS_IPV4: u8 = 1;
const NETCODE_ADDRESS_IPV6: u8 = 2;

const ADDITIONAL_DATA_SIZE: usize = 13 + 8 + 8;
