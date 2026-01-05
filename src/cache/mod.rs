pub mod types;
pub use types::{FileMetadata, StalenessReport};

use crate::models::EditRecord;
use anyhow::{Context, Result};
use chrono::Utc;
use duckdb::{Connection, OptionalExt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub type EditsByFile = HashMap<String, Vec<EditRecord>>;

/// Manages DuckDB cache for parsed trace files
pub struct CacheManager {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl CacheManager {
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Cache connection lock poisoned: {}", e))
    }

    /// Open or create cache database at project root.
    ///
    /// Creates a DuckDB cache file at `{project_root}/.ai-blame.ddb` to store
    /// parsed trace data. If the cache is corrupted, it will be automatically
    /// rebuilt.
    ///
    /// # Arguments
    ///
    /// * `project_root` - Directory where the cache file should be created
    ///
    /// # Returns
    ///
    /// A `CacheManager` instance ready for caching operations
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created or opened
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ai_blame::cache::CacheManager;
    /// use std::path::Path;
    ///
    /// let project_root = Path::new("/path/to/project");
    /// let cache = CacheManager::open(project_root)?;
    ///
    /// // Cache file will be created at /path/to/project/.ai-blame.ddb
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open(project_root: &Path) -> Result<Self> {
        let db_path = project_root.join(".ai-blame.ddb");

        let conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Cache corrupted ({}), rebuilding...", e);
                let _ = std::fs::remove_file(&db_path);
                Connection::open(&db_path)?
            }
        };

        // Initialize schema if needed
        Self::initialize_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        })
    }

    /// Initialize database schema if not already present
    fn initialize_schema(conn: &Connection) -> Result<()> {
        // Try to get version
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM cache_metadata WHERE key = 'version'",
                [],
                |row| row.get(0),
            )
            .ok();

        match version.as_deref() {
            Some("1") => {
                // Current version, schema already exists
                Ok(())
            }
            Some(v) => {
                // Unknown version, rebuild cache
                Err(anyhow::anyhow!("Unsupported cache version: {}", v))
            }
            None => {
                // Fresh database, create schema
                // Create cache_metadata table
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS cache_metadata (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    )",
                    [],
                )?;

                // Insert version (safe because table just created)
                conn.execute(
                    "INSERT INTO cache_metadata (key, value) VALUES ('version', '1')",
                    [],
                )?;

                // Create trace_files table
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS trace_files (
                        file_path TEXT PRIMARY KEY,
                        provider TEXT NOT NULL,
                        file_mtime_ns TEXT NOT NULL,
                        file_size_bytes TEXT NOT NULL,
                        last_parsed_at TEXT NOT NULL,
                        record_count INTEGER NOT NULL DEFAULT 0,
                        parse_duration_ms INTEGER
                    )",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_trace_files_provider ON trace_files(provider)",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_trace_files_mtime ON trace_files(file_mtime_ns)",
                    [],
                )?;

                // Create edit_records table
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS edit_records (
                        trace_file_path TEXT NOT NULL,
                        file_path TEXT NOT NULL,
                        timestamp TEXT NOT NULL,
                        model TEXT NOT NULL,
                        session_id TEXT NOT NULL,
                        is_create BOOLEAN NOT NULL,
                        change_size INTEGER NOT NULL,
                        agent_tool TEXT NOT NULL,
                        agent_version TEXT,
                        old_string TEXT,
                        new_string TEXT,
                        structured_patch TEXT,
                        create_content TEXT
                    )",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_edits_file_path ON edit_records(file_path)",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_edits_timestamp ON edit_records(timestamp)",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_edits_session ON edit_records(session_id)",
                    [],
                )?;

                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_edits_trace_file ON edit_records(trace_file_path)",
                    [],
                )?;

                Ok(())
            }
        }
    }

    /// Get cached edits for a specific trace file (if fresh)
    pub fn get_cached_edits(&self, trace_file: &Path) -> Result<Option<Vec<EditRecord>>> {
        let conn = self.lock_conn()?;

        // Check if file exists in cache
        let exists = conn
            .query_row(
                "SELECT 1 FROM trace_files WHERE file_path = ?1",
                [trace_file.to_string_lossy().as_ref()],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|e| anyhow::anyhow!("Failed to query trace file: {}", e))?;

        if exists.is_none() {
            return Ok(None);
        }

        // Fetch all EditRecords for this file
        let mut stmt = conn.prepare(
            "SELECT file_path, timestamp, model, session_id, is_create, change_size,
                    agent_tool, agent_version, old_string, new_string,
                    structured_patch, create_content
             FROM edit_records
             WHERE trace_file_path = ?1
             ORDER BY timestamp",
        )?;

        struct CachedEditRow {
            file_path: String,
            timestamp: String,
            model: String,
            session_id: String,
            is_create: bool,
            change_size: i64,
            agent_tool: String,
            agent_version: Option<String>,
            old_string: Option<String>,
            new_string: Option<String>,
            structured_patch: Option<String>,
            create_content: Option<String>,
        }

        let rows = stmt.query_map([trace_file.to_string_lossy().as_ref()], |row| {
            Ok(CachedEditRow {
                file_path: row.get(0)?,
                timestamp: row.get(1)?,
                model: row.get(2)?,
                session_id: row.get(3)?,
                is_create: row.get(4)?,
                change_size: row.get(5)?,
                agent_tool: row.get(6)?,
                agent_version: row.get(7)?,
                old_string: row.get(8)?,
                new_string: row.get(9)?,
                structured_patch: row.get(10)?,
                create_content: row.get(11)?,
            })
        })?;

        let mut edits = Vec::new();
        for row in rows {
            let row = row?;
            let timestamp = row.timestamp.parse().map_err(|e| {
                anyhow::anyhow!("Invalid cached timestamp '{}': {}", row.timestamp, e)
            })?;
            edits.push(EditRecord {
                file_path: row.file_path,
                timestamp,
                model: row.model,
                session_id: row.session_id,
                is_create: row.is_create,
                change_size: row.change_size as usize,
                agent_tool: row.agent_tool,
                agent_version: row.agent_version,
                old_string: row.old_string,
                new_string: row.new_string,
                structured_patch: row.structured_patch,
                create_content: row.create_content,
            });
        }

        Ok(Some(edits))
    }

    /// Store parsed edits for a trace file in the cache
    pub fn store_edits(
        &self,
        trace_file: &Path,
        provider: &str,
        edits: &[EditRecord],
        parse_duration_ms: u64,
    ) -> Result<()> {
        let conn = self.lock_conn()?;

        // Get file metadata
        let meta = std::fs::metadata(trace_file)?;
        let mtime_ns = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as i64;
        let size_bytes = meta.len() as i64;

        // Begin transaction
        conn.execute("BEGIN TRANSACTION", [])?;

        // Delete old trace_files entry if exists
        conn.execute(
            "DELETE FROM trace_files WHERE file_path = ?1",
            [trace_file.to_string_lossy().as_ref()],
        )?;

        // Insert/update trace_files
        conn.execute(
            "INSERT INTO trace_files
             (file_path, provider, file_mtime_ns, file_size_bytes, last_parsed_at, record_count, parse_duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            duckdb::params![
                trace_file.to_string_lossy(),
                provider,
                mtime_ns.to_string(),
                size_bytes.to_string(),
                Utc::now().to_rfc3339(),
                edits.len() as i64,
                parse_duration_ms as i64,
            ],
        )?;

        let trace_file_str = trace_file.to_string_lossy();

        // Delete old edits for this trace file
        conn.execute(
            "DELETE FROM edit_records WHERE trace_file_path = ?1",
            [trace_file_str.as_ref()],
        )?;

        // Insert new edits
        let mut stmt = conn.prepare(
            "INSERT INTO edit_records
             (trace_file_path, file_path, timestamp, model, session_id, is_create,
              change_size, agent_tool, agent_version, old_string, new_string,
              structured_patch, create_content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )?;

        for edit in edits {
            stmt.execute(duckdb::params![
                &trace_file_str,
                &edit.file_path,
                edit.timestamp.to_rfc3339(),
                &edit.model,
                &edit.session_id,
                edit.is_create,
                edit.change_size as i64,
                &edit.agent_tool,
                &edit.agent_version,
                &edit.old_string,
                &edit.new_string,
                &edit.structured_patch,
                &edit.create_content,
            ])?;
        }

        conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Get all cached edits across all trace files
    pub fn get_all_edits(&self, file_pattern: Option<&str>) -> Result<EditsByFile> {
        let conn = self.lock_conn()?;

        let query = if let Some(pattern) = file_pattern {
            format!(
                "SELECT file_path, timestamp, model, session_id, is_create, change_size,
                        agent_tool, agent_version, old_string, new_string,
                        structured_patch, create_content
                 FROM edit_records
                 WHERE file_path LIKE '%{}%'
                 ORDER BY file_path, timestamp",
                pattern.replace('\'', "''")
            )
        } else {
            "SELECT file_path, timestamp, model, session_id, is_create, change_size,
                    agent_tool, agent_version, old_string, new_string,
                    structured_patch, create_content
             FROM edit_records
             ORDER BY file_path, timestamp"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let mut edits_by_file: EditsByFile = HashMap::new();

        struct CachedEditRow {
            file_path: String,
            timestamp: String,
            model: String,
            session_id: String,
            is_create: bool,
            change_size: i64,
            agent_tool: String,
            agent_version: Option<String>,
            old_string: Option<String>,
            new_string: Option<String>,
            structured_patch: Option<String>,
            create_content: Option<String>,
        }

        let rows = stmt.query_map([], |row| {
            Ok(CachedEditRow {
                file_path: row.get(0)?,
                timestamp: row.get(1)?,
                model: row.get(2)?,
                session_id: row.get(3)?,
                is_create: row.get(4)?,
                change_size: row.get(5)?,
                agent_tool: row.get(6)?,
                agent_version: row.get(7)?,
                old_string: row.get(8)?,
                new_string: row.get(9)?,
                structured_patch: row.get(10)?,
                create_content: row.get(11)?,
            })
        })?;

        for result in rows {
            let row = result?;
            let timestamp = row.timestamp.parse().map_err(|e| {
                anyhow::anyhow!("Invalid cached timestamp '{}': {}", row.timestamp, e)
            })?;
            let edit = EditRecord {
                file_path: row.file_path.clone(),
                timestamp,
                model: row.model,
                session_id: row.session_id,
                is_create: row.is_create,
                change_size: row.change_size as usize,
                agent_tool: row.agent_tool,
                agent_version: row.agent_version,
                old_string: row.old_string,
                new_string: row.new_string,
                structured_patch: row.structured_patch,
                create_content: row.create_content,
            };
            edits_by_file.entry(row.file_path).or_default().push(edit);
        }

        Ok(edits_by_file)
    }

    /// Invalidate cache for specific trace files
    pub fn invalidate_files(&self, trace_files: &[PathBuf]) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute("BEGIN TRANSACTION", [])?;

        for file in trace_files {
            let file_path_str = file.to_string_lossy();

            // Delete edit records for this trace file
            conn.execute(
                "DELETE FROM edit_records WHERE trace_file_path = ?1",
                [file_path_str.as_ref()],
            )?;

            // Delete trace file
            conn.execute(
                "DELETE FROM trace_files WHERE file_path = ?1",
                [file_path_str.as_ref()],
            )?;
        }

        conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Get file metadata from cache
    pub fn get_file_metadata(&self, trace_file: &Path) -> Result<Option<FileMetadata>> {
        let conn = self.lock_conn()?;

        let result = conn
            .query_row(
                "SELECT file_mtime_ns, file_size_bytes, last_parsed_at
                 FROM trace_files WHERE file_path = ?1",
                [trace_file.to_string_lossy().as_ref()],
                |row| {
                    let mtime_ns_str: String = row.get(0)?;
                    let size_bytes_str: String = row.get(1)?;
                    let last_parsed_at: String = row.get(2)?;
                    Ok((mtime_ns_str, size_bytes_str, last_parsed_at))
                },
            )
            .optional()
            .map_err(|e| anyhow::anyhow!("Failed to query file metadata: {}", e))?;

        match result {
            Some((mtime_ns_str, size_bytes_str, last_parsed_at)) => {
                let file_mtime_ns = mtime_ns_str
                    .parse()
                    .with_context(|| format!("Invalid file mtime in cache: '{}'", mtime_ns_str))?;
                let file_size_bytes = size_bytes_str
                    .parse()
                    .with_context(|| format!("Invalid file size in cache: '{}'", size_bytes_str))?;
                Ok(Some(FileMetadata {
                    file_mtime_ns,
                    file_size_bytes,
                    last_parsed_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get cache database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_round_trip() -> Result<()> {
        let temp = TempDir::new()?;
        let cache = CacheManager::open(temp.path())?;

        let trace_file = temp.path().join("test.jsonl");
        std::fs::write(&trace_file, "{}")?;

        let edits = vec![EditRecord {
            file_path: "/test/file.rs".to_string(),
            timestamp: Utc::now(),
            model: "test-model".to_string(),
            session_id: "test-session".to_string(),
            is_create: true,
            change_size: 100,
            agent_tool: "test-tool".to_string(),
            agent_version: None,
            old_string: None,
            new_string: None,
            structured_patch: None,
            create_content: Some("test".to_string()),
        }];

        cache.store_edits(&trace_file, "test", &edits, 50)?;
        let retrieved = cache.get_cached_edits(&trace_file)?.unwrap();

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].file_path, "/test/file.rs");
        Ok(())
    }

    #[test]
    fn test_metadata_retrieval() -> Result<()> {
        let temp = TempDir::new()?;
        let cache = CacheManager::open(temp.path())?;

        let trace_file = temp.path().join("test.jsonl");
        std::fs::write(&trace_file, "{}")?;

        // No metadata initially
        assert!(cache.get_file_metadata(&trace_file)?.is_none());

        // Store edits
        cache.store_edits(&trace_file, "test", &[], 0)?;

        // Metadata should exist now
        let meta = cache.get_file_metadata(&trace_file)?.unwrap();
        assert!(meta.file_mtime_ns > 0);
        assert!(meta.file_size_bytes >= 0);

        Ok(())
    }
}
