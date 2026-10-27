CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); -- value = JSON
CREATE TABLE snapshots (source TEXT PRIMARY KEY, fetched_at_ms INTEGER NOT NULL, body TEXT NOT NULL);
