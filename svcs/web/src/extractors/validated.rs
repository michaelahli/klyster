//! Validated JSON extractor with field-level error messages.

use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// Validated JSON extractor that returns 422 with field-level errors on validation failure.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidationError::JsonRejection)?;

        value.validate().map_err(ValidationError::Validation)?;

        Ok(ValidatedJson(value))
    }
}

/// Validation error response.
#[derive(Debug)]
pub enum ValidationError {
    /// JSON parsing error.
    JsonRejection(JsonRejection),
    /// Validation error.
    Validation(validator::ValidationErrors),
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        match self {
            ValidationError::JsonRejection(rejection) => {
                let message = format!("Invalid JSON: {rejection}");
                let body = serde_json::json!({
                    "error": {
                        "code": "invalid_json",
                        "message": message,
                    }
                });
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            ValidationError::Validation(errors) => {
                let field_errors: Vec<_> = errors
                    .field_errors()
                    .iter()
                    .map(|(field, errors)| {
                        let messages: Vec<String> = errors
                            .iter()
                            .filter_map(|e| e.message.as_ref().map(std::string::ToString::to_string))
                            .collect();
                        serde_json::json!({
                            "field": field,
                            "messages": messages,
                        })
                    })
                    .collect();

                let body = serde_json::json!({
                    "error": {
                        "code": "validation_error",
                        "message": "Request validation failed",
                        "fields": field_errors,
                    }
                });
                (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
        Router,
    };
    use serde::Deserialize;
    use tower::ServiceExt;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestRequest {
        #[validate(length(min = 1, message = "Name cannot be empty"))]
        name: String,
        #[validate(range(min = 1, max = 100, message = "Age must be between 1 and 100"))]
        age: u32,
        #[validate(email(message = "Invalid email format"))]
        email: String,
    }

    async fn test_handler(ValidatedJson(req): ValidatedJson<TestRequest>) -> String {
        format!("Hello, {}!", req.name)
    }

    #[tokio::test]
    async fn test_valid_request() {
        let app = Router::new().route("/test", post(test_handler));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"John","age":30,"email":"john@example.com"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let app = Router::new().route("/test", post(test_handler));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"invalid json"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_validation_error_empty_name() {
        let app = Router::new().route("/test", post(test_handler));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"","age":30,"email":"john@example.com"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "validation_error");
    }

    #[tokio::test]
    async fn test_validation_error_invalid_age() {
        let app = Router::new().route("/test", post(test_handler));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"John","age":150,"email":"john@example.com"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_validation_error_invalid_email() {
        let app = Router::new().route("/test", post(test_handler));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"John","age":30,"email":"invalid"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
