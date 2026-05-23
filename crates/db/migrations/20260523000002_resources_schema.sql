-- Resource and target schema
-- Tables for tracked resources (pods, VMs) and scaling targets

-- Resource groups (clusters, namespaces, etc.)
CREATE TABLE IF NOT EXISTS resource_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    provider_type TEXT NOT NULL CHECK(provider_type IN ('kubernetes', 'vm', 'cloud')),
    provider_config TEXT NOT NULL, -- JSON configuration
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Resources (pods, VMs, nodes)
CREATE TABLE IF NOT EXISTS resources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    namespace TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('pod', 'vm', 'node', 'deployment', 'statefulset')),
    labels TEXT, -- JSON labels
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive', 'deleted')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES resource_groups(id) ON DELETE CASCADE,
    UNIQUE(group_id, name, namespace)
);

-- Scaling targets (autoscaling configuration)
CREATE TABLE IF NOT EXISTS scaling_targets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_group_id INTEGER NOT NULL,
    metric_name TEXT NOT NULL,
    min_replicas INTEGER NOT NULL CHECK(min_replicas >= 0),
    max_replicas INTEGER NOT NULL CHECK(max_replicas >= min_replicas),
    target_value REAL NOT NULL CHECK(target_value > 0),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (resource_group_id) REFERENCES resource_groups(id) ON DELETE CASCADE,
    UNIQUE(resource_group_id, metric_name)
);

-- Indexes for query performance
CREATE INDEX IF NOT EXISTS idx_resources_group_id ON resources(group_id);
CREATE INDEX IF NOT EXISTS idx_resources_kind ON resources(kind);
CREATE INDEX IF NOT EXISTS idx_resources_status ON resources(status);
CREATE INDEX IF NOT EXISTS idx_scaling_targets_resource_group_id ON scaling_targets(resource_group_id);
