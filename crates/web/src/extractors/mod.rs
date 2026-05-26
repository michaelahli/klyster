//! Custom extractors for request validation and pagination.

pub mod pagination;
pub mod validated;

pub use pagination::Pagination;
pub use validated::ValidatedJson;
