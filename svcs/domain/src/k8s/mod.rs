//! Kubernetes integration for Klyster.

pub mod client;
/// Kubernetes resource discovery.
pub mod discovery;

pub use client::{init_client, K8sClientError, K8sResult};
