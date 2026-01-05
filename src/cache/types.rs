use std::path::PathBuf;

/// Metadata about a cached trace file for staleness detection
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_mtime_ns: i64,     // Modification time (nanoseconds since epoch)
    pub file_size_bytes: i64,   // File size in bytes
    pub last_parsed_at: String, // ISO8601 timestamp of when it was cached
}

/// Report of which files are stale vs fresh
#[derive(Debug, Clone)]
pub struct StalenessReport {
    pub stale_files: Vec<PathBuf>, // Files that need reparsing
    pub fresh_files: Vec<PathBuf>, // Files that can use cached data
}
