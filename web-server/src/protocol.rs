use crate::configuration::NetcodePrivateKey;
use protocol::server::TokenUserData;
use renetcode::{ConnectToken, NETCODE_USER_DATA_BYTES, TokenGenerationError};
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

// TODO: These parameters are arbitrary for now
pub fn generate_connect_token(
    account_id: i32,
    character_id: i32,
    private_key: &NetcodePrivateKey,
    server_addr: SocketAddr,
    traceparent: Option<String>,
) -> Result<ConnectToken, TokenGenerationError> {
    let public_addresses: Vec<SocketAddr> = vec![server_addr];

    let user_data = bitcode::encode(&TokenUserData {
        character_id,
        traceparent,
    });

    if user_data.len() > NETCODE_USER_DATA_BYTES - 1 {
        return Err(TokenGenerationError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("user data too large: {} bytes (max 255)", user_data.len()),
        )));
    }

    let mut user_data_buffer: [u8; NETCODE_USER_DATA_BYTES] = [0; NETCODE_USER_DATA_BYTES];
    user_data_buffer[0] = user_data.len() as u8;
    user_data_buffer[1..1 + user_data.len()].copy_from_slice(user_data.as_slice());

    let token = ConnectToken::generate(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        0,
        300,
        account_id as u64,
        15,
        public_addresses,
        Some(&user_data_buffer),
        private_key.as_ref(),
    )?;

    Ok(token)
}

pub fn encode_connect_token(token: ConnectToken) -> Result<String, std::io::Error> {
    let mut token_buffer: Vec<u8> = vec![];
    token.write(&mut token_buffer)?;
    Ok(base64::encode_config(token_buffer, base64::STANDARD))
}
