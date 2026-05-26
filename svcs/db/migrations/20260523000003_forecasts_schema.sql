-- Forecast and recommendation schema
-- Tables for storing forecast results and scaling recommendations

-- Forecasts (prediction results)
CREATE TABLE IF NOT EXISTS forecasts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_group_id INTEGER NOT NULL,
    metric_name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    parameters TEXT, -- JSON parameters used for the forecast
    horizon_start TIMESTAMP NOT NULL,
    horizon_end TIMESTAMP NOT NULL,
    confidence_score REAL CHECK(confidence_score >= 0 AND confidence_score <= 1),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (resource_group_id) REFERENCES resource_groups(id) ON DELETE CASCADE
);

-- Forecast points (individual predictions in time series)
CREATE TABLE IF NOT EXISTS forecast_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    forecast_id INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    predicted_value REAL NOT NULL,
    lower_bound REAL,
    upper_bound REAL,
    FOREIGN KEY (forecast_id) REFERENCES forecasts(id) ON DELETE CASCADE
);

-- Recommendations (scaling actions)
CREATE TABLE IF NOT EXISTS recommendations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    forecast_id INTEGER,
    resource_group_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('scale_up', 'scale_down', 'none')),
    current_count INTEGER NOT NULL CHECK(current_count >= 0),
    recommended_count INTEGER NOT NULL CHECK(recommended_count >= 0),
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'dismissed', 'executed')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    decided_at TIMESTAMP,
    decided_by TEXT,
    FOREIGN KEY (forecast_id) REFERENCES forecasts(id) ON DELETE SET NULL,
    FOREIGN KEY (resource_group_id) REFERENCES resource_groups(id) ON DELETE CASCADE
);

-- Indexes for query performance
CREATE INDEX IF NOT EXISTS idx_forecasts_resource_group_id ON forecasts(resource_group_id);
CREATE INDEX IF NOT EXISTS idx_forecasts_created_at ON forecasts(created_at);
CREATE INDEX IF NOT EXISTS idx_forecast_points_forecast_id ON forecast_points(forecast_id);
CREATE INDEX IF NOT EXISTS idx_forecast_points_timestamp ON forecast_points(timestamp);
CREATE INDEX IF NOT EXISTS idx_recommendations_resource_group_id ON recommendations(resource_group_id);
CREATE INDEX IF NOT EXISTS idx_recommendations_status ON recommendations(status);
CREATE INDEX IF NOT EXISTS idx_recommendations_created_at ON recommendations(created_at);
