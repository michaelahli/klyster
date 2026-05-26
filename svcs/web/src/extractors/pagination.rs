//! Pagination extractor for query parameters.

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use serde::{Deserialize, Serialize};

/// Pagination parameters with defaults and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    50
}

impl Pagination {
    /// Maximum items per page.
    pub const MAX_PER_PAGE: u32 = 100;

    /// Calculate the offset for database queries.
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    /// Get the limit (capped at MAX_PER_PAGE).
    pub fn limit(&self) -> u32 {
        self.per_page.min(Self::MAX_PER_PAGE)
    }

    /// Validate and normalize pagination parameters.
    pub fn normalize(mut self) -> Self {
        if self.page == 0 {
            self.page = 1;
        }
        if self.per_page == 0 {
            self.per_page = default_per_page();
        }
        if self.per_page > Self::MAX_PER_PAGE {
            self.per_page = Self::MAX_PER_PAGE;
        }
        self
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = <Query<Pagination> as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<Pagination>::from_request_parts(parts, state).await?;
        Ok(pagination.normalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pagination() {
        let pagination = Pagination {
            page: default_page(),
            per_page: default_per_page(),
        };
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 50);
        assert_eq!(pagination.offset(), 0);
        assert_eq!(pagination.limit(), 50);
    }

    #[test]
    fn test_pagination_offset() {
        let pagination = Pagination {
            page: 1,
            per_page: 10,
        };
        assert_eq!(pagination.offset(), 0);

        let pagination = Pagination {
            page: 2,
            per_page: 10,
        };
        assert_eq!(pagination.offset(), 10);

        let pagination = Pagination {
            page: 5,
            per_page: 20,
        };
        assert_eq!(pagination.offset(), 80);
    }

    #[test]
    fn test_pagination_limit_cap() {
        let pagination = Pagination {
            page: 1,
            per_page: 200,
        };
        assert_eq!(pagination.limit(), 100); // Capped at MAX_PER_PAGE
    }

    #[test]
    fn test_pagination_normalize() {
        let pagination = Pagination {
            page: 0,
            per_page: 0,
        }
        .normalize();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 50);

        let pagination = Pagination {
            page: 1,
            per_page: 200,
        }
        .normalize();
        assert_eq!(pagination.per_page, 100); // Capped
    }
}
