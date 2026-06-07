-- Add DaemonSet as a first-class resource kind.
-- SQLite cannot alter CHECK constraints in place, so rebuild the table.

CREATE TABLE IF NOT EXISTS resources_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    namespace TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('pod', 'vm', 'node', 'deployment', 'statefulset', 'daemonset')),
    labels TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive', 'deleted')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES resource_groups(id) ON DELETE CASCADE,
    UNIQUE(group_id, name, namespace)
);

INSERT INTO resources_new (id, group_id, name, namespace, kind, labels, status, created_at, updated_at)
SELECT id, group_id, name, namespace, kind, labels, status, created_at, updated_at
FROM resources;

DROP TABLE resources;
ALTER TABLE resources_new RENAME TO resources;

CREATE INDEX IF NOT EXISTS idx_resources_group_id ON resources(group_id);
CREATE INDEX IF NOT EXISTS idx_resources_kind ON resources(kind);
CREATE INDEX IF NOT EXISTS idx_resources_status ON resources(status);
