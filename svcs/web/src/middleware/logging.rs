//! Request/response logging middleware.

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info, warn};

/// Middleware for detailed request/response logging.
///
/// Logs every request with method, path, status, duration, and request ID.
#[allow(clippy::cast_possible_truncation)]
pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request details
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    // Generate request ID (or extract from header if present)
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok()).map_or_else(|| uuid::Uuid::new_v4().to_string(), std::string::ToString::to_string);

    // Process request
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    // Log based on status code
    if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            version = ?version,
            status = %status.as_u16(),
            duration_ms = duration.as_millis() as u64,
            "Request completed with server error"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            version = ?version,
            status = %status.as_u16(),
            duration_ms = duration.as_millis() as u64,
            "Request completed with client error"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            version = ?version,
            status = %status.as_u16(),
            duration_ms = duration.as_millis() as u64,
            "Request completed successfully"
        );
    }

    // Warn on slow requests (> 1 second)
    if duration.as_secs() >= 1 {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            duration_ms = duration.as_millis() as u64,
            "Slow request detected"
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    async fn error_handler() -> impl IntoResponse {
        (StatusCode::INTERNAL_SERVER_ERROR, "error")
    }

    async fn slow_handler() -> impl IntoResponse {
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        (StatusCode::OK, "slow response")
    }

    #[tokio::test]
    async fn test_request_logging_success() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_logging_middleware));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_logging_error() {
        let app = Router::new()
            .route("/error", get(error_handler))
            .layer(middleware::from_fn(request_logging_middleware));

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_request_logging_with_request_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_logging_middleware));

        let request = Request::builder()
            .uri("/test")
            .header("x-request-id", "test-request-id-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_logging_slow_request() {
        let app = Router::new()
            .route("/slow", get(slow_handler))
            .layer(middleware::from_fn(request_logging_middleware));

        let request = Request::builder().uri("/slow").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
