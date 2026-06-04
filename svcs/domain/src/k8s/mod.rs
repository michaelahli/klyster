//! Kubernetes integration for Klyster.

pub mod client;
pub mod discovery;

pub use client::{init_client, K8sClientError, K8sResult};
