//! Infrastructure provider abstractions.

pub mod kubernetes;
mod r#trait;

pub use r#trait::{Capacity, InfraProvider};
