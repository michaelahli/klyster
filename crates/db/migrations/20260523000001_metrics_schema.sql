-- Metric storage schema
-- Tables for storing time-series metrics data

-- Metric sources (Prometheus, Agent, etc.)
CREATE TABLE IF NOT EXISTS metric_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL CHECK(type IN ('prometheus', 'agent')),
    config TEXT NOT NULL, -- JSON configuration
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Core metrics table (time-series data)
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_id) REFERENCES metric_sources(id) ON DELETE CASCADE
);

-- Metric labels (dimensional data)
CREATE TABLE IF NOT EXISTS metric_labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    FOREIGN KEY (metric_id) REFERENCES metrics(id) ON DELETE CASCADE
);

-- Indexes for query performance
CREATE INDEX IF NOT EXISTS idx_metrics_name_timestamp ON metrics(name, timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_source_timestamp ON metrics(source_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_metric_labels_metric_id ON metric_labels(metric_id);
CREATE INDEX IF NOT EXISTS idx_metric_labels_key_value ON metric_labels(key, value);
