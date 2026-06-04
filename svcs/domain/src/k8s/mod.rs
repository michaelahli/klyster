//! Kubernetes integration for Klyster.

pub mod client;

pub use client::{init_client, K8sClientError, K8sResult};
