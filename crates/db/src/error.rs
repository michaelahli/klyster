//! Database error types.

use thiserror::Error;

/// Database operation errors.
#[derive(Debug, Error)]
pub enum DbError {
    /// `SQLx` database error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for database operations.
pub type DbResult<T> = Result<T, DbError>;
