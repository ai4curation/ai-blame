-- Cache metadata and versioning
CREATE TABLE IF NOT EXISTS cache_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Initialize metadata if table is new
INSERT OR IGNORE INTO cache_metadata (key, value) VALUES ('version', '1');
INSERT OR IGNORE INTO cache_metadata (key, value) VALUES ('created_at', CAST(NOW() AS VARCHAR));

-- Per-file metadata for staleness detection
CREATE TABLE IF NOT EXISTS trace_files (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL,                   -- 'claude' or 'codex'
    file_mtime_ns INTEGER NOT NULL,          -- File modification time (nanoseconds since epoch)
    file_size_bytes INTEGER NOT NULL,        -- File size for quick change detection
    last_parsed_at TEXT NOT NULL,            -- ISO8601 timestamp
    record_count INTEGER NOT NULL DEFAULT 0, -- Number of EditRecords extracted
    parse_duration_ms INTEGER                -- How long parsing took (for metrics)
);

CREATE INDEX IF NOT EXISTS idx_trace_files_provider ON trace_files(provider);
CREATE INDEX IF NOT EXISTS idx_trace_files_mtime ON trace_files(file_mtime_ns);

-- Cached EditRecords (denormalized for fast queries)
CREATE TABLE IF NOT EXISTS edit_records (
    id INTEGER PRIMARY KEY,
    trace_file_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,                 -- File that was edited (NOT trace file)
    timestamp TEXT NOT NULL,                 -- ISO8601 timestamp
    model TEXT NOT NULL,
    session_id TEXT NOT NULL,
    is_create BOOLEAN NOT NULL,
    change_size INTEGER NOT NULL,
    agent_tool TEXT NOT NULL,
    agent_version TEXT,
    old_string TEXT,
    new_string TEXT,
    structured_patch TEXT,
    create_content TEXT,
    FOREIGN KEY (trace_file_id) REFERENCES trace_files(id)
);

CREATE INDEX IF NOT EXISTS idx_edits_file_path ON edit_records(file_path);
CREATE INDEX IF NOT EXISTS idx_edits_timestamp ON edit_records(timestamp);
CREATE INDEX IF NOT EXISTS idx_edits_session ON edit_records(session_id);
CREATE INDEX IF NOT EXISTS idx_edits_trace_file ON edit_records(trace_file_id);
