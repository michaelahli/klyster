-- Agent API Keys table
CREATE TABLE IF NOT EXISTS agent_api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME,
    is_active BOOLEAN NOT NULL DEFAULT 1
);

CREATE INDEX idx_agent_api_keys_key_hash ON agent_api_keys(key_hash);
CREATE INDEX idx_agent_api_keys_is_active ON agent_api_keys(is_active);
