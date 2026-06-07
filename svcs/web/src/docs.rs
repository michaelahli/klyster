//! OpenAPI documentation generation.

#![allow(clippy::needless_for_each)]

use utoipa::OpenApi;

/// `OpenAPI` documentation for Klyster API.
#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Klyster API",
        version = "0.1.0",
        description = "Capacity Planning Application for Kubernetes and VM Workloads",
        contact(
            name = "Klyster Contributors",
            url = "https://github.com/klyster/klyster"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    paths(
        // Health endpoints would be documented here
        // For now, we'll keep it simple with a basic structure
    ),
    components(schemas()),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "sources", description = "Metric source management"),
        (name = "metrics", description = "Metric data queries"),
        (name = "resource-groups", description = "Resource group management"),
        (name = "forecasts", description = "Forecast management"),
        (name = "recommendations", description = "Recommendation management"),
        (name = "analytics", description = "Analytics function management"),
        (name = "config", description = "Configuration management")
    )
)]
pub struct ApiDoc;

/// Get the `OpenAPI` specification as JSON.
///
/// # Panics
/// Panics if the `OpenAPI` spec cannot be serialized to JSON.
#[must_use]
pub fn openapi_spec() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = openapi_spec();
        assert!(spec.contains("Klyster API"));
        assert!(spec.contains("0.1.0"));
    }

    #[test]
    fn test_openapi_spec_is_valid_json() {
        let spec = openapi_spec();
        let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
        assert_eq!(parsed["info"]["title"], "Klyster API");
        assert_eq!(parsed["info"]["version"], "0.1.0");
    }

    #[test]
    fn test_openapi_spec_has_tags() {
        let spec = openapi_spec();
        let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
        let tags = parsed["tags"].as_array().unwrap();

        let tag_names: Vec<&str> = tags.iter().filter_map(|t| t["name"].as_str()).collect();

        assert!(tag_names.contains(&"health"));
        assert!(tag_names.contains(&"sources"));
        assert!(tag_names.contains(&"metrics"));
        assert!(tag_names.contains(&"resource-groups"));
        assert!(tag_names.contains(&"forecasts"));
        assert!(tag_names.contains(&"recommendations"));
        assert!(tag_names.contains(&"analytics"));
        assert!(tag_names.contains(&"config"));
    }
}
