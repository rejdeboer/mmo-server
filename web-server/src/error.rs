use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    AuthError(String),
    UnexpectedError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(e) => (
                StatusCode::BAD_REQUEST,
                Json::from(format!("Bad request: {}", e)),
            ),
            Self::AuthError(e) => (
                StatusCode::UNAUTHORIZED,
                Json::from(format!("Authorization error: {}", e)),
            ),
            Self::UnexpectedError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json::from("An unexpected error has occured".to_string()),
            ),
        }
        .into_response()
    }
}
