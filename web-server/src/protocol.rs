use crate::configuration::NetcodePrivateKey;
use flatbuffers::FlatBufferBuilder;
use renetcode::{ConnectToken, NETCODE_USER_DATA_BYTES, TokenGenerationError};
use schemas::protocol::{TokenUserData, TokenUserDataArgs};
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

    let mut builder = FlatBufferBuilder::new();
    let traceparent = traceparent.map(|v| builder.create_string(&v));
    let response_offset = TokenUserData::create(
        &mut builder,
        &TokenUserDataArgs {
            character_id,
            traceparent,
        },
    );
    builder.finish_minimal(response_offset);

    let mut user_data: [u8; NETCODE_USER_DATA_BYTES] = [0; NETCODE_USER_DATA_BYTES];
    let copy_data = builder.finished_data();
    user_data[0..copy_data.len()].copy_from_slice(copy_data);

    let token = ConnectToken::generate(
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        0,
        300,
        account_id as u64,
        15,
        public_addresses,
        Some(&user_data),
        private_key.as_ref(),
    )?;

    Ok(token)
}

pub fn encode_connect_token(token: ConnectToken) -> Result<String, std::io::Error> {
    let mut token_buffer: Vec<u8> = vec![];
    token.write(&mut token_buffer)?;
    Ok(base64::encode_config(token_buffer, base64::STANDARD))
}
