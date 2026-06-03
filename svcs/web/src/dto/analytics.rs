//! DTOs for analytics function endpoints.

use serde::{Deserialize, Serialize};

/// Request to create a custom analytics function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFunctionRequest {
    /// Function name (must be unique).
    pub name: String,
    /// Function description.
    pub description: String,
    /// Programming language (e.g., "python").
    pub language: String,
    /// Source code for the function.
    pub source_code: String,
    /// JSON schema for parameters (optional).
    pub parameters_schema: Option<serde_json::Value>,
}

/// Request to update a custom analytics function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFunctionRequest {
    /// Function name (must be unique).
    pub name: String,
    /// Function description.
    pub description: String,
    /// Programming language (e.g., "python").
    pub language: String,
    /// Source code for the function.
    pub source_code: String,
    /// JSON schema for parameters (optional).
    pub parameters_schema: Option<serde_json::Value>,
    /// Whether the function is active.
    pub is_active: bool,
}

/// Response for an analytics function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Function ID.
    pub id: i64,
    /// Function name.
    pub name: String,
    /// Function description.
    pub description: String,
    /// Function type: "predefined" or "custom".
    #[serde(rename = "type")]
    pub function_type: String,
    /// Programming language.
    pub language: String,
    /// Source code (only for custom functions).
    pub source_code: Option<String>,
    /// JSON schema for parameters.
    pub parameters_schema: Option<serde_json::Value>,
    /// Whether the function is active.
    pub is_active: bool,
    /// Creation timestamp (ISO8601).
    pub created_at: String,
    /// Last update timestamp (ISO8601).
    pub updated_at: String,
}

impl FunctionResponse {
    /// Convert from domain model.
    #[must_use] 
    pub fn from_model(function: domain::models::AnalyticsFunction) -> Self {
        let parameters_schema: Option<serde_json::Value> = function
            .parameters_schema
            .and_then(|p| serde_json::from_str(&p).ok());

        Self {
            id: function.id,
            name: function.name,
            description: function.description,
            function_type: function.function_type,
            language: function.language,
            source_code: function.source_code,
            parameters_schema,
            is_active: function.is_active,
            created_at: function.created_at.to_rfc3339(),
            updated_at: function.updated_at.to_rfc3339(),
        }
    }
}

/// Response for function list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionListResponse {
    /// List of functions.
    pub functions: Vec<FunctionResponse>,
    /// Total count.
    pub total: usize,
}

/// Request to test a function with sample data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFunctionRequest {
    /// Sample input data (JSON).
    pub input_data: serde_json::Value,
    /// Function parameters (JSON).
    pub parameters: Option<serde_json::Value>,
}

/// Response for function test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFunctionResponse {
    /// Test result status: "success" or "error".
    pub status: String,
    /// Output data (if successful).
    pub output: Option<serde_json::Value>,
    /// Error message (if failed).
    pub error: Option<String>,
}
