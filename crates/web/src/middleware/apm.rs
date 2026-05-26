//! APM (Application Performance Monitoring) logging middleware.

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

/// Middleware for APM logging compatible with Kibana/Elastic APM format.
///
/// Logs request/response traces with duration, trace IDs, and error tracking.
pub async fn apm_logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    
    // Generate trace ID for distributed tracing
    let trace_id = Uuid::new_v4().to_string();
    
    // Extract request details
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let version = format!("{:?}", request.version());
    
    // Add trace ID to request headers (for downstream services)
    // Note: In a real implementation, we'd use request extensions
    
    // Process request
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status = response.status().as_u16();
    
    // Log in APM-compatible format
    if status >= 500 {
        warn!(
            trace_id = %trace_id,
            http_method = %method,
            http_url = %uri,
            http_version = %version,
            http_status_code = status,
            duration_ms = duration.as_millis() as u64,
            event_outcome = "failure",
            "HTTP request failed"
        );
    } else if status >= 400 {
        warn!(
            trace_id = %trace_id,
            http_method = %method,
            http_url = %uri,
            http_version = %version,
            http_status_code = status,
            duration_ms = duration.as_millis() as u64,
            event_outcome = "failure",
            "HTTP request client error"
        );
    } else {
        info!(
            trace_id = %trace_id,
            http_method = %method,
            http_url = %uri,
            http_version = %version,
            http_status_code = status,
            duration_ms = duration.as_millis() as u64,
            event_outcome = "success",
            "HTTP request completed"
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

    #[tokio::test]
    async fn test_apm_middleware_success() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(apm_logging_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_apm_middleware_error() {
        let app = Router::new()
            .route("/error", get(error_handler))
            .layer(middleware::from_fn(apm_logging_middleware));

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
