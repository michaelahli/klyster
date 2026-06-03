//! Analytics function domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Analytics function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FunctionType {
    /// Predefined function
    Predefined,
    /// Custom user-defined function
    Custom,
}

impl FunctionType {
    /// Convert to database string representation.
    #[must_use] 
    pub fn as_str(&self) -> &'static str {
        match self {
            FunctionType::Predefined => "predefined",
            FunctionType::Custom => "custom",
        }
    }
}

impl FromStr for FunctionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "predefined" => Ok(FunctionType::Predefined),
            "custom" => Ok(FunctionType::Custom),
            _ => Err(format!("Invalid function type: {s}")),
        }
    }
}

/// Analytics function definition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsFunction {
    /// Unique identifier
    pub id: i64,
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Function type (predefined/custom)
    #[sqlx(rename = "type")]
    pub function_type: String,
    /// Programming language
    pub language: String,
    /// Source code (NULL for predefined functions)
    pub source_code: Option<String>,
    /// JSON schema for parameters
    pub parameters_schema: Option<String>,
    /// Whether the function is active
    pub is_active: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl AnalyticsFunction {
    /// Get the function type as enum.
    #[must_use] 
    pub fn get_type(&self) -> Option<FunctionType> {
        self.function_type.parse().ok()
    }

    /// Check if this is a predefined function.
    #[must_use] 
    pub fn is_predefined(&self) -> bool {
        self.get_type() == Some(FunctionType::Predefined)
    }

    /// Check if this is a custom function.
    #[must_use] 
    pub fn is_custom(&self) -> bool {
        self.get_type() == Some(FunctionType::Custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_type_serialization() {
        let predefined = FunctionType::Predefined;
        let json = serde_json::to_string(&predefined).unwrap();
        assert_eq!(json, "\"predefined\"");

        let custom = FunctionType::Custom;
        let json = serde_json::to_string(&custom).unwrap();
        assert_eq!(json, "\"custom\"");
    }

    #[test]
    fn test_function_type_deserialization() {
        let predefined: FunctionType = serde_json::from_str("\"predefined\"").unwrap();
        assert_eq!(predefined, FunctionType::Predefined);

        let custom: FunctionType = serde_json::from_str("\"custom\"").unwrap();
        assert_eq!(custom, FunctionType::Custom);
    }

    #[test]
    fn test_function_type_as_str() {
        assert_eq!(FunctionType::Predefined.as_str(), "predefined");
        assert_eq!(FunctionType::Custom.as_str(), "custom");
    }

    #[test]
    fn test_function_type_from_str() {
        assert_eq!(
            "predefined".parse::<FunctionType>().unwrap(),
            FunctionType::Predefined
        );
        assert_eq!(
            "custom".parse::<FunctionType>().unwrap(),
            FunctionType::Custom
        );
        assert!("invalid".parse::<FunctionType>().is_err());
    }
}
