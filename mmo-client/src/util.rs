use renet_netcode::{ConnectToken, NetcodeError};

pub fn decode_token(encoded: String) -> Result<ConnectToken, NetcodeError> {
    // NOTE: Take the base64 padding into account
    let mut decoded: [u8; 1100] = [0; 1100];
    let bytes_written = base64::decode_config_slice(encoded, base64::STANDARD, &mut decoded)
        .map_err(|_| NetcodeError::PayloadAboveLimit)?;

    let mut token = &decoded[..bytes_written];

    ConnectToken::read(&mut token)
}
