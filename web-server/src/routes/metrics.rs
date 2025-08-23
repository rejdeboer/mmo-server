use crate::telemetry::REGISTRY;
use axum::http::StatusCode;
use prometheus::{Encoder, TextEncoder};

pub async fn metrics_get() -> (StatusCode, String) {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        );
    }

    let response = String::from_utf8(buffer.clone()).unwrap_or_else(|err| {
        tracing::error!(?err, "metrics buffer is not valid UTF-8");
        String::new()
    });

    (StatusCode::OK, response)
}
