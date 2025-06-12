use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    AuthError(String),
    DocumentNotFoundError(Uuid),
    UnexpectedError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(e) => (StatusCode::BAD_REQUEST, format!("Bad request: {}", e)),
            Self::AuthError(e) => (
                StatusCode::UNAUTHORIZED,
                format!("Authorization error: {}", e),
            ),
            Self::DocumentNotFoundError(doc_id) => (
                StatusCode::NOT_FOUND,
                format!("Document {} could not be found for user", doc_id),
            ),
            Self::UnexpectedError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error has occured".to_string(),
            ),
        }
        .into_response()
    }
}
