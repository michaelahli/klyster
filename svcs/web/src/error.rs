//! HTTP error handling and response types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standard error response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error details.
    pub error: ErrorDetail,
}

/// Error detail structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

/// Application errors that can be returned from handlers.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Resource not found.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid input or validation error.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Database error.
    #[error("Database error: {0}")]
    Database(#[from] db::DbError),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Conflict (e.g., duplicate resource).
    #[error("Conflict: {0}")]
    Conflict(String),
}

impl ApiError {
    /// Get the HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Database(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
        }
    }

    /// Get the error code string.
    #[must_use]
    pub fn error_code(&self) -> &str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::ValidationError(_) => "validation_error",
            ApiError::Database(_) => "database_error",
            ApiError::Internal(_) => "internal_error",
            ApiError::Conflict(_) => "conflict",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code().to_string();
        let message = self.to_string();

        let body = ErrorResponse {
            error: ErrorDetail { code, message },
        };

        (status, Json(body)).into_response()
    }
}

/// Result type for API handlers.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_has_correct_status() {
        let err = ApiError::NotFound("user".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code(), "not_found");
    }

    #[test]
    fn validation_error_has_correct_status() {
        let err = ApiError::ValidationError("invalid email".to_string());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.error_code(), "validation_error");
    }

    #[test]
    fn conflict_has_correct_status() {
        let err = ApiError::Conflict("duplicate name".to_string());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        assert_eq!(err.error_code(), "conflict");
    }
}
