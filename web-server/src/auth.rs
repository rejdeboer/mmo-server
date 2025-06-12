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
    auth_header: Option<TypedHeader<headers::Authorization>>,
    State(signing_key): State<Secret<String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token_split = if let Some(TypedHeader(auth_header)) = auth_header {
        auth_header.to_string().split(" ")
    } else {
        tracing::error!("no auth token provided");
        return ApiError::AuthError("no auth token".to_string());
    };

    if token_split.len() != 2 || token_split[0] != "Bearer" {
        tracing::error!("invalid auth token format");
        return ApiError::AuthError("invalid auth token format".to_string());
    }

    let token = decode_jwt(&token_split[1], signing_key).map_err(|e| {
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
