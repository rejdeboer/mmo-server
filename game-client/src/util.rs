use renet_netcode::{ConnectToken, NetcodeError};

pub fn decode_token(encoded: String) -> Result<ConnectToken, NetcodeError> {
    // NOTE: Take the base64 padding into account
    let mut decoded: [u8; 2048] = [0; 2048];
    base64::decode_config_slice(encoded, base64::STANDARD, &mut decoded)
        .map_err(|_| NetcodeError::PayloadAboveLimit)?;

    let mut token = &decoded[..];

    ConnectToken::read(&mut token)
}

#[test]
fn test_decode() {
    let encoded = "AQAAAAAAAABORVRDT0RFIDEuMDIAAAAAAAAAAABF4lFoAAAAAHHjUWgAAAAAh9jDHynYtfY/kY+EdWYMNdb6H5xSuR+dhincwKVJNBtKcTLvvfJufcTdHteBFw+nu8q5DMRznnnBIyCzM0KRUOvicsxd6teSPsxDZX7H08ISVKrEg3r33bJUYUtehpUw5EPgTzjUthOExdqXIdLetl3BAicb2JUqjEnZgLmCgybpWCNtdB0oJNM8kIoVgCIBldiKKQNNETpE4CppiYjy72mXTKxPcY0Dlf+aZE25GC7BdBLgSjTDMAzy/uvZiDi8LCoW7Xjg0yMcEMjj5MsEm3vPIlLv2FS4ina2VLlb+0qJS/TauyLoF2IE2UqhkGRT21tnOsxPlXp4q2SPMuZ3OiHWE66OnSUjFmKXWzFqTvj1Jc5lJthCVw54fr7f8hpHREdQoI3XQFLcZYojrA8dYpjQRCpvdrVPs3LzLgFqj2LF8H75RGpzlZUVRtVqfraGBUSA5S/w2iA2hRfrZpnmzkn6HEegkoffBNd3e46aRSQ/ou9YIADe1DuRYIuJLoU+0UfPb1jPS9+ZlaAurl8iJm/wzDThnziHFV2WXXExiCLHPNeifFiPZZm1U/YnS6k7jI8jua2Az4+gis2yx5HaUHz4bo0fF1XtFGcAct4WE+lwc3Mlj81CgdmzYf+QPHqOxnM6yGMO9BbhYt60GPNueKNXJ5mlLDEWRAVbBjDg/teryq9pG8khrrZ5+pa6+jAu6YRoVTFirOT3h9Nhb7FtFrgyJ18LeV7k1vPs2goIEk0kgcy7uDBdsbzkgfVlu/xTKEZjZNzVliTnlcVApcdZ8VJmj5SASeKpOtD1WbhPV3rYWuY1SEfP6cLM9kLxCwaX6xArVeN4hDbB3L6OjHvhM/4yiE9F71N/TATo5Q158Lm0xjcjxfibQ7bkgP/oFDcaHtwTvZtkczIC+Vviu9xjIPIGeAe4GyRU2tjbHqUfRfUjXT/SqKXdrHP/plRx65JVxb/YFVtwxS/Yw6nh8dk4f94fHfMSaHiB+P9JZjTCZz7XVWUXAKYmZqmmA2kNtgXE9kBx7r5H3fgRpL+s0o4Z7A8k5yKOReBCkWZlya6zFYicS3wEtJn0t1R9YiU0wnPUkPFxJtxQFERDVulV9LcVo/IqAagIRS/QWiOUu0D9NqpOfdLwlVtqZshIrPLoK/hm6q5zWHHh0BVcz+1Fx2nllAKk1HCIJX77BVoW74xmK/gnFj/8iKddGr3ITPzHCPPN7utAjPzVBGBNvDN9a3PBc8tBiRxJgIdhw2J7+fV6q2d8inp0NMc7Utc3lO3rXovRwb1tEV/0idkrS/bQv6BUyeOFFb2ACius19/z86smYzp+xLRJYpWyGnn1IdLepEfpzRkMiCb9HIqUiHJMQ7u3j6Rlft89sKrUctv3EcTNlhrlCjCKpgmuHQ8AAAABAAAAAX8AAAFAH4VciIuUZRpEOVcmBhEZdUxTRCFPuXEhuNTIB3m8vLOTi1Iy+O32E/KvlJYBxjUpXijtK8RUAadeqIMk0mRJpyY=".to_string();

    decode_token(encoded).unwrap();
}
