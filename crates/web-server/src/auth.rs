use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::error::ApiError;

const TOKEN_DURATION_SEC: u64 = 7200;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims<T> {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub exp: u64,
    #[serde(flatten)]
    pub ctx: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountContext {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub account_id: i32,
    pub username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterContext {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub account_id: i32,
    pub username: String,
    pub character_id: i32,
}

pub async fn account_auth_middleware(
    auth_header_option: Option<TypedHeader<headers::Authorization<Bearer>>>,
    State(jwt_signing_key): State<SecretString>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = auth_header_option.ok_or(ApiError::AuthError("no auth token".to_string()))?;
    let token =
        decode_jwt::<AccountContext>(auth_header.token(), jwt_signing_key).map_err(|e| {
            tracing::error!(?e, "JWT decoding error");
            ApiError::AuthError("invalid token".to_string())
        })?;
    req.extensions_mut().insert(token.claims.ctx);
    Ok(next.run(req).await)
}

pub async fn character_auth_middleware(
    auth_header_option: Option<TypedHeader<headers::Authorization<Bearer>>>,
    State(jwt_signing_key): State<SecretString>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = auth_header_option.ok_or(ApiError::AuthError("no auth token".to_string()))?;
    let token =
        decode_jwt::<CharacterContext>(auth_header.token(), jwt_signing_key).map_err(|e| {
            tracing::error!(?e, "JWT decoding error");
            ApiError::AuthError(
                "invalid token, make sure you are logged in and have selected a character"
                    .to_string(),
            )
        })?;
    req.extensions_mut().insert(token.claims.ctx);
    Ok(next.run(req).await)
}

pub fn encode_jwt<T: Serialize + DeserializeOwned>(
    ctx: T,
    signing_key: &str,
) -> jsonwebtoken::errors::Result<String> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH - Duration::from_secs(TOKEN_DURATION_SEC))
        .unwrap()
        .as_secs();

    let claims = Claims { ctx, exp };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(signing_key.as_bytes()),
    )
}

pub fn decode_jwt<T: Serialize + DeserializeOwned>(
    token: &str,
    signing_key: SecretString,
) -> jsonwebtoken::errors::Result<TokenData<Claims<T>>> {
    jsonwebtoken::decode(
        token,
        &DecodingKey::from_secret(signing_key.expose_secret().as_ref()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
}
