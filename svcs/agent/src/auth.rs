//! Agent authentication middleware.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Validate API key from X-API-Key header.
pub async fn validate_api_key(request: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if !key.is_empty() => {
            // TODO: Validate key against database
            // For now, accept any non-empty key
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
