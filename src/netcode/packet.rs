#[derive(Debug)]
#[repr(u8)]
pub enum PacketType {
    ConnectionRequest = 0,
    ConnectionDenied = 1,
    Challenge = 2,
    Response = 3,
    KeepAlive = 4,
    Payload = 5,
    Disconnect = 6,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Packet<'a> {
    ConnectionRequest {
        version_info: [u8; 5], // "0.01" ASCII with null terminator.
        protocol_id: u64,
        expire_timestamp: u64,
        xnonce: [u8; CONNECT_TOKEN_XNONCE_BYTES],
        data: [u8; CONNECT_TOKEN_PRIVATE_BYTES],
    },
    ConnectionDenied,
    Challenge {
        token_sequence: u64,
        token_data: [u8; CHALLENGE_TOKEN_BYTES], // encrypted ChallengeToken
    },
    Response {
        token_sequence: u64,
        token_data: [u8; CHALLENGE_TOKEN_BYTES], // encrypted ChallengeToken
    },
    KeepAlive {
        client_index: u32,
        max_clients: u32,
    },
    Payload(&'a [u8]),
    Disconnect,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChallengeToken {
    pub client_id: u64,
    pub user_data: [u8; 256],
}

impl PacketType {
    fn from_u8(value: u8) -> Result<Self, NetcodeError> {
        use PacketType::*;

        let packet_type = match value {
            0 => ConnectionRequest,
            1 => ConnectionDenied,
            2 => Challenge,
            3 => Response,
            4 => KeepAlive,
            5 => Payload,
            6 => Disconnect,
            _ => return Err(NetcodeError::InvalidPacketType),
        };
        Ok(packet_type)
    }

    fn apply_replay_protection(&self) -> bool {
        use PacketType::*;

        matches!(self, KeepAlive | Payload | Disconnect)
    }
}
