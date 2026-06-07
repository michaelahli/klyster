//! Infrastructure provider trait for capacity management.

use crate::models::Resource;
use std::fmt;

/// Capacity information for a resource group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capacity {
    /// Current ready/running replica count.
    pub current: u32,
    /// Desired replica count from spec.
    pub desired: u32,
    /// Minimum allowed capacity.
    pub min: u32,
    /// Maximum allowed capacity.
    pub max: u32,
}

impl Capacity {
    /// Returns true if the current capacity differs from desired.
    #[must_use]
    pub fn has_drift(&self) -> bool {
        self.current != self.desired
    }
}

/// Infrastructure provider abstraction.
///
/// Implementations provide access to infrastructure resources (K8s, VMs, cloud)
/// and enable capacity queries and validation.
#[async_trait::async_trait]
pub trait InfraProvider: Send + Sync {
    /// Error type for provider operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// List all discoverable resources managed by this provider.
    async fn get_resources(&self) -> Result<Vec<Resource>, Self::Error>;

    /// Get current capacity for a resource group.
    async fn get_current_capacity(&self, group_id: &str) -> Result<Capacity, Self::Error>;

    /// Validate that a scale target is within acceptable bounds.
    async fn validate_scale_target(&self, group_id: &str, target: u32) -> Result<(), Self::Error>;

    /// Provider name for logging and identification.
    fn name(&self) -> &str;
}

impl fmt::Display for Capacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Capacity(current={}, desired={}, min={}, max={})",
            self.current, self.desired, self.min, self.max
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacity_display() {
        let cap = Capacity {
            current: 3,
            desired: 3,
            min: 1,
            max: 10,
        };
        assert_eq!(
            cap.to_string(),
            "Capacity(current=3, desired=3, min=1, max=10)"
        );
    }

    #[test]
    fn test_capacity_no_drift() {
        let cap = Capacity {
            current: 5,
            desired: 5,
            min: 1,
            max: 10,
        };
        assert!(!cap.has_drift());
    }

    #[test]
    fn test_capacity_has_drift() {
        let cap = Capacity {
            current: 3,
            desired: 5,
            min: 1,
            max: 10,
        };
        assert!(cap.has_drift());
    }
}
