-- Database schema for the standalone Falcon Bridge service

CREATE TABLE IF NOT EXISTS protocol_mappings (
    did TEXT PRIMARY KEY,
    actor_uri TEXT NOT NULL,
    protocol TEXT NOT NULL,
    last_resolved DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS bridged_activities (
    id TEXT PRIMARY KEY,
    source_protocol TEXT NOT NULL,
    target_protocol TEXT NOT NULL,
    external_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
