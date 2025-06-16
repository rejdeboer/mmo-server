use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::error::ApiError;

const TOKEN_DURATION_SEC: u64 = 7200;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub exp: u64,
    // TODO: Might be more secure to store some external account ID instead
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub account_id: i32,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub account_id: i32,
}

pub async fn auth_middleware(
    auth_header_option: Option<TypedHeader<headers::Authorization<Bearer>>>,
    State(jwt_signing_key): State<Secret<String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = auth_header_option.ok_or(ApiError::AuthError("no auth token".to_string()))?;
    let token = decode_jwt(auth_header.token(), jwt_signing_key).map_err(|e| {
        tracing::error!(?e, "JWT decoding error");
        ApiError::AuthError("invalid token".to_string())
    })?;

    let user = User {
        account_id: token.claims.account_id,
    };
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

pub fn encode_jwt(
    account_id: i32,
    username: String,
    signing_key: &str,
) -> jsonwebtoken::errors::Result<String> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH - Duration::from_secs(TOKEN_DURATION_SEC))
        .unwrap()
        .as_secs();

    let claims = Claims {
        account_id,
        username,
        exp,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(signing_key.as_bytes()),
    )
}

pub fn decode_jwt(
    token: &str,
    signing_key: Secret<String>,
) -> jsonwebtoken::errors::Result<TokenData<Claims>> {
    jsonwebtoken::decode(
        token,
        &DecodingKey::from_secret(signing_key.expose_secret().as_ref()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
}
