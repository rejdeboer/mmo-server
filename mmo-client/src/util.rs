use renet_netcode::{ConnectToken, NetcodeError};

pub fn decode_token(encoded: String) -> Result<ConnectToken, NetcodeError> {
    let mut decoded: [u8; 1024] = [0; 1024];
    let bytes_written = base64::decode_config_slice(encoded, base64::STANDARD_NO_PAD, &mut decoded)
        .map_err(|_| NetcodeError::PayloadAboveLimit)?;

    let mut token = &decoded[..bytes_written];

    ConnectToken::read(&mut token)
}
