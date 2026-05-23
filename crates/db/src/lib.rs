//! Database access, migrations, and repositories for Klyster.

/// Error types for database operations.
pub mod error;
/// Database migration management.
pub mod migrate;
/// Database connection pool abstraction.
pub mod pool;

pub use error::{DbError, DbResult};
pub use migrate::run_migrations;
pub use pool::DatabasePool;
