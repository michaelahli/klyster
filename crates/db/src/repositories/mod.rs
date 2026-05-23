//! Repository implementations for data access.

pub mod forecast_repo;
pub mod metric_repo;
pub mod resource_repo;

pub use forecast_repo::ForecastRepository;
pub use metric_repo::MetricRepository;
pub use resource_repo::ResourceRepository;
