//! Prometheus integration for metric collection.

pub mod adapter;
pub mod client;
pub mod collector;
pub mod discovery;
pub mod health;
pub mod query_builder;

pub use adapter::{AdapterError, PrometheusAdapter};
pub use client::{PrometheusClient, PrometheusConfig, PrometheusError};
pub use collector::{CollectorConfig, CustomQuery, MetricCollector};
pub use discovery::{DiscoveryConfig, ServiceDiscovery, Target};
pub use health::{HealthMonitor, HealthStatus};
pub use query_builder::{Aggregation, CommonQueries, QueryBuilder};
