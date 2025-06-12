use crate::{auth::User, error::ApiError, ApplicationState};
use axum::extract::ConnectInfo;
use axum::{extract::State, response::Response, Extension};
use axum_extra::TypedHeader;
use std::net::SocketAddr;

// TODO: Add connection context for tracing
pub async fn character_post(
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<ApplicationState>,
    Extension(user): Extension<User>,
) -> Result<Response, ApiError> {
    let _user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown client")
    };
}
