//! Domain models, configuration, and shared utilities for Klyster.

/// Configuration management.
pub mod config;
/// Kubernetes integration.
pub mod k8s;
/// Logging initialization.
pub mod logging;
/// Domain models.
pub mod models;
/// Recommendation engine: pure decision logic on top of forecasts.
pub mod recommendation_engine;
/// Graceful shutdown orchestration.
pub mod shutdown;

pub use config::Config;
pub use recommendation_engine::{
    evaluate as evaluate_recommendation, ForecastSummary, RecommendationDraft, RecommendationPolicy,
};
