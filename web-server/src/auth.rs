use std::str::FromStr;

use axum::{
    extract::{Query, Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, TokenData, Validation};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use uuid::Uuid;

use crate::{error::ApiError, server::QueryParams};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub exp: u64,
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
}

pub async fn auth_middleware(
    // TODO: Using query parameters for the token is not very secure
    // But the WebSocket web API does not support the usage of custom headers
    // It's probably better to use some ephemeral OTP
    Query(params): Query<QueryParams>,
    State(signing_key): State<Secret<String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = decode_jwt(&params.token, signing_key).map_err(|e| {
        tracing::error!(?e, "JWT decoding error");
        ApiError::AuthError("invalid token".to_string())
    })?;

    let user = User {
        id: Uuid::from_str(&token.claims.user_id).unwrap(),
    };
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

fn decode_jwt(
    token: &str,
    signing_key: Secret<String>,
) -> jsonwebtoken::errors::Result<TokenData<Claims>> {
    decode(
        token,
        &DecodingKey::from_secret(signing_key.expose_secret().as_ref()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
}
