//! Domain models for Klyster.

pub mod analytics;
pub mod forecast;
pub mod metric;
pub mod resource;

pub use analytics::{AnalyticsFunction, FunctionType};
pub use forecast::{
    Forecast, ForecastPoint, Recommendation, RecommendationAction, RecommendationStatus,
};
pub use metric::{Metric, MetricLabel, MetricSource, MetricSourceType};
pub use resource::{Resource, ResourceGroup, ResourceKind, ScalingTarget};
