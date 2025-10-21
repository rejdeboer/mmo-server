use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum ApiError {
    UnexpectedError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::UnexpectedError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json::from("An unexpected error has occured".to_string()),
            ),
        }
        .into_response()
    }
}
