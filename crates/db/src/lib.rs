//! Database access, migrations, and repositories for Klyster.

/// Error types for database operations.
pub mod error;
/// Database connection pool abstraction.
pub mod pool;

pub use error::{DbError, DbResult};
pub use pool::DatabasePool;
