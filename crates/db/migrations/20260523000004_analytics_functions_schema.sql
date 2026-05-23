-- Analytics function registry schema
-- Table for storing registered analytics functions (predefined + custom)

CREATE TABLE IF NOT EXISTS analytics_functions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('predefined', 'custom')),
    language TEXT NOT NULL DEFAULT 'python' CHECK(language IN ('python')),
    source_code TEXT, -- NULL for predefined functions
    parameters_schema TEXT, -- JSON schema for function parameters
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed predefined analytics functions
INSERT INTO analytics_functions (name, description, type, language, parameters_schema, is_active) VALUES
('linear_regression', 'Simple linear regression for trend forecasting', 'predefined', 'python', '{"type": "object", "properties": {"lookback_days": {"type": "integer", "default": 7}}}', 1),
('arima', 'ARIMA (AutoRegressive Integrated Moving Average) time series forecasting', 'predefined', 'python', '{"type": "object", "properties": {"p": {"type": "integer", "default": 1}, "d": {"type": "integer", "default": 1}, "q": {"type": "integer", "default": 1}}}', 1),
('seasonal_decomposition', 'Seasonal decomposition for identifying trends and seasonality', 'predefined', 'python', '{"type": "object", "properties": {"period": {"type": "integer", "default": 24}}}', 1),
('threshold_rules', 'Simple threshold-based rules for capacity recommendations', 'predefined', 'python', '{"type": "object", "properties": {"upper_threshold": {"type": "number", "default": 0.8}, "lower_threshold": {"type": "number", "default": 0.3}}}', 1);

-- Index for query performance
CREATE INDEX IF NOT EXISTS idx_analytics_functions_type ON analytics_functions(type);
CREATE INDEX IF NOT EXISTS idx_analytics_functions_is_active ON analytics_functions(is_active);
