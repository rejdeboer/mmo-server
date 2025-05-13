//! Modified https://github.com/lucaspoffo/renet
pub mod error;
pub mod packet;

const MAX_PACKET_BYTES: usize = 1400;

const CONNECT_TOKEN_XNONCE_BYTES: usize = 24;
const CHALLENGE_TOKEN_BYTES: usize = 300;
const CONNECT_TOKEN_PRIVATE_BYTES: usize = 1024;
