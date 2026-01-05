use crate::git_batch::BatchGitReader;
use crate::models::EditRecord;
use crate::parsers::{ParserInfo, TraceParser};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Parser for GitHub Copilot/Codex trace files
pub struct CodexParser;

impl CodexParser {
    pub fn new() -> Self {
        Self
    }

    /// Check if directory is a Codex CLI sessions directory
    fn is_codex_sessions_dir(&self, path: &Path) -> bool {
        path.to_string_lossy().contains("/.codex/sessions")
            || path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "codex-sessions")
                .unwrap_or(false)
    }

    /// Check if this file is a Codex CLI session (contains ghost snapshots)
    fn is_cli_session_file(path: &Path) -> Result<bool> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for (line_count, result) in reader.lines().enumerate() {
            if line_count >= 100 {
                break;
            }

            let line = result?;
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            if let Ok(record) = serde_json::from_str::<Value>(line) {
                if is_ghost_snapshot(&record)
                    || record.get("type").and_then(|t| t.as_str()) == Some("turn_context")
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Parse a standard Codex trace file (single-pass)
    fn parse_standard_codex(&self, path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut edits = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            let record = match serde_json::from_str::<Value>(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Skip non-edit records
            if !is_successful_codex_edit(&record) {
                continue;
            }

            // Extract file path (multiple field names possible)
            let file_path = record
                .get("file")
                .or_else(|| record.get("file_path"))
                .or_else(|| record.get("filePath"))
                .and_then(|f| f.as_str())
                .unwrap_or("");

            if file_path.is_empty() {
                continue;
            }

            // Apply file pattern filter
            if !file_pattern.is_empty() && !file_path.contains(file_pattern) {
                continue;
            }

            // Extract model
            let model = crate::parsers::common::extract_model_from_record(&record)
                .unwrap_or("unknown")
                .to_string();

            // Parse timestamp
            let timestamp = record
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            // Extract session ID from record or use default
            let session_id = record
                .get("sessionId")
                .or_else(|| record.get("session_id"))
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Determine if create or edit
            let is_create = record
                .get("action")
                .and_then(|a| a.as_str())
                .map(|a| a == "create")
                .unwrap_or(false)
                || record
                    .get("event")
                    .and_then(|e| e.as_str())
                    .map(|e| e == "create")
                    .unwrap_or(false);

            // Extract content
            let content = record
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());

            let old_content = record
                .get("old_content")
                .or_else(|| record.get("oldContent"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());

            let new_content = record
                .get("new_content")
                .or_else(|| record.get("newContent"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());

            // Calculate change size
            let change_size = if is_create {
                content.as_ref().map(|s| s.len()).unwrap_or(0)
            } else if let (Some(old), Some(new)) = (&old_content, &new_content) {
                let len_diff = (new.len() as i64 - old.len() as i64).unsigned_abs() as usize;
                len_diff + old.len().max(new.len())
            } else {
                0
            };

            let edit = EditRecord {
                file_path: file_path.to_string(),
                timestamp,
                model,
                session_id,
                is_create,
                change_size,
                agent_tool: "github-copilot".to_string(),
                agent_version: record
                    .get("agent_version")
                    .or_else(|| record.get("agentVersion"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                old_string: old_content,
                new_string: new_content,
                structured_patch: None,
                create_content: content,
            };

            edits.push(edit);
        }

        Ok(edits)
    }

    /// Parse a Codex CLI session file (single-pass with git lookups)
    fn parse_cli_session(
        &self,
        path: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
        batch_reader: Option<&BatchGitReader>,
        verbose: u8,
    ) -> Result<Vec<EditRecord>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Collect ghost snapshots with state
        struct SnapshotState {
            timestamp: DateTime<Utc>,
            commit_id: String,
            files: Vec<String>,
        }

        let mut snapshots: Vec<SnapshotState> = Vec::new();
        let mut model = "gpt-5.2-codex".to_string();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            let record = match serde_json::from_str::<Value>(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract model from turn_context
            if record.get("type").and_then(|t| t.as_str()) == Some("turn_context") {
                if let Some(m) = record
                    .get("payload")
                    .and_then(|p| p.get("model"))
                    .and_then(|m| m.as_str())
                {
                    model = m.to_string();
                }
            }

            // Extract ghost snapshots
            if is_ghost_snapshot(&record) {
                if let Some(commit_id) = extract_ghost_commit_id(&record) {
                    let files = extract_ghost_commit_files(&record);
                    let timestamp = record
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    snapshots.push(SnapshotState {
                        timestamp,
                        commit_id,
                        files,
                    });
                }
            }
        }

        if verbose >= 2 {
            eprintln!(
                "      → Found {} ghost snapshots, processing changes...",
                snapshots.len()
            );
        }

        // Compare successive snapshots to find changes
        let mut edits: Vec<EditRecord> = Vec::new();
        let mut git_time = std::time::Duration::ZERO;

        // Create or reuse batch reader if repo_root is available
        let owned_reader;
        let batch_reader = if let Some(reader) = batch_reader {
            reader
        } else if let Some(root) = repo_root {
            owned_reader = BatchGitReader::new(root)?;
            &owned_reader
        } else {
            // Can't proceed without either batch_reader or repo_root
            return Err(anyhow::anyhow!("Need repo_root for git operations"));
        };

        // Helper function to get file content using batch reader with filesystem fallback
        let get_content = |commit_id: &str, file_path: &str| -> Result<String> {
            // Try batch reader first
            match batch_reader.get_file_content(commit_id, file_path) {
                Ok(content) => Ok(content),
                Err(_) => {
                    // Fallback to filesystem for ghost commits
                    let root = repo_root.ok_or_else(|| {
                        anyhow::anyhow!(
                            "Batch reader failed and repo_root unavailable for fallback"
                        )
                    })?;
                    let file_path_abs = root.join(file_path);

                    // Security: validate path
                    if file_path.contains("..") || file_path.starts_with('/') {
                        anyhow::bail!("Invalid file path: {}", file_path);
                    }

                    let canonical_repo = root.canonicalize()?;
                    let canonical_file = file_path_abs.canonicalize()?;

                    if !canonical_file.starts_with(&canonical_repo) {
                        anyhow::bail!("File path outside repository: {}", file_path);
                    }

                    std::fs::read_to_string(&file_path_abs)
                        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file_path, e))
                }
            }
        };

        for i in 1..snapshots.len() {
            let prev = &snapshots[i - 1];
            let curr = &snapshots[i];
            let snapshot_start = Instant::now();
            let mut snapshot_git_calls = 0;

            if repo_root.is_some() {
                // Find added files
                for file in &curr.files {
                    if !prev.files.contains(file) {
                        if !file_pattern.is_empty() && !file.contains(file_pattern) {
                            continue;
                        }

                        let git_start = Instant::now();
                        if let Ok(content) = get_content(&curr.commit_id, file) {
                            git_time += git_start.elapsed();
                            snapshot_git_calls += 1;
                            let edit = EditRecord {
                                file_path: file.to_string(),
                                timestamp: curr.timestamp,
                                model: model.clone(),
                                session_id: path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                is_create: true,
                                change_size: content.len(),
                                agent_tool: "codex-cli".to_string(),
                                agent_version: None,
                                old_string: None,
                                new_string: None,
                                structured_patch: None,
                                create_content: Some(content),
                            };
                            edits.push(edit);
                        } else {
                            git_time += git_start.elapsed();
                            snapshot_git_calls += 1;
                        }
                    }
                }

                // Find modified files
                for file in &curr.files {
                    if prev.files.contains(file) {
                        if !file_pattern.is_empty() && !file.contains(file_pattern) {
                            continue;
                        }

                        let git_start = Instant::now();
                        let prev_content = get_content(&prev.commit_id, file).ok();
                        git_time += git_start.elapsed();
                        snapshot_git_calls += 1;

                        let git_start = Instant::now();
                        let curr_content = get_content(&curr.commit_id, file).ok();
                        git_time += git_start.elapsed();
                        snapshot_git_calls += 1;

                        if let (Some(prev_text), Some(curr_text)) = (prev_content, curr_content) {
                            if prev_text != curr_text {
                                let change_size = (curr_text.len() as i64 - prev_text.len() as i64)
                                    .unsigned_abs()
                                    as usize
                                    + prev_text.len().max(curr_text.len());

                                let edit = EditRecord {
                                    file_path: file.to_string(),
                                    timestamp: curr.timestamp,
                                    model: model.clone(),
                                    session_id: path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    is_create: false,
                                    change_size,
                                    agent_tool: "codex-cli".to_string(),
                                    agent_version: None,
                                    old_string: Some(prev_text),
                                    new_string: Some(curr_text),
                                    structured_patch: None,
                                    create_content: None,
                                };
                                edits.push(edit);
                            }
                        }
                    }
                }

                let snapshot_elapsed = snapshot_start.elapsed();
                if snapshot_git_calls > 0 && verbose >= 2 {
                    eprintln!(
                        "        Snapshot {}: {} git calls in {:.2}s",
                        i,
                        snapshot_git_calls,
                        snapshot_elapsed.as_secs_f64()
                    );
                }
            } else {
                // No repo root, can't fetch content
            }
        }

        if git_time.as_secs_f64() > 0.0 && verbose >= 2 {
            eprintln!("      → Total git time: {:.2}s", git_time.as_secs_f64());
        }

        Ok(edits)
    }
}

impl Default for CodexParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceParser for CodexParser {
    fn info(&self) -> ParserInfo {
        ParserInfo {
            name: "codex",
            description: "Parser for GitHub Copilot/Codex trace files",
            file_extensions: vec!["jsonl"],
        }
    }

    fn can_parse(&self, path: &Path) -> Result<Option<bool>> {
        // Check for Codex sessions directory structure
        if let Some(parent) = path.parent() {
            if self.is_codex_sessions_dir(parent) {
                return Ok(Some(true));
            }
        }

        // Quick peek at first few records
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        for _ in 0..10 {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }

            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            let Ok(record) = serde_json::from_str::<Value>(line) else {
                continue;
            };

            // Check for Codex-specific markers
            if is_codex_format_check(&record) {
                return Ok(Some(true));
            }

            // Check for ghost snapshots (Codex CLI feature)
            if is_ghost_snapshot(&record) {
                return Ok(Some(true));
            }
        }

        Ok(None)
    }

    fn parse_file(&self, path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
        // Fast path: check directory structure first to avoid expensive file peeking
        let is_cli_session_from_dir = path
            .parent()
            .map(|p| p.to_string_lossy().contains("/.codex/sessions"))
            .unwrap_or(false);

        // Determine if this is a CLI session or standard trace
        if is_cli_session_from_dir || Self::is_cli_session_file(path)? {
            // For CLI sessions without repo root, return empty (standard parse_file doesn't have context)
            self.parse_cli_session(path, file_pattern, None, None, 0)
        } else {
            self.parse_standard_codex(path, file_pattern)
        }
    }

    fn parse_file_with_context(
        &self,
        path: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
    ) -> Result<Vec<EditRecord>> {
        // Fast path: check directory structure first to avoid expensive file peeking
        let is_cli_session_from_dir = path
            .parent()
            .map(|p| p.to_string_lossy().contains("/.codex/sessions"))
            .unwrap_or(false);

        // Determine if this is a CLI session or standard trace
        if is_cli_session_from_dir || Self::is_cli_session_file(path)? {
            // For CLI sessions, use the provided repo_root for git operations (batch_reader will be created)
            self.parse_cli_session(path, file_pattern, repo_root, None, 0)
        } else {
            self.parse_standard_codex(path, file_pattern)
        }
    }

    fn parse_file_with_batch(
        &self,
        path: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
        batch_reader: Option<&BatchGitReader>,
    ) -> Result<Vec<EditRecord>> {
        // Fast path: check directory structure first to avoid expensive file peeking
        let is_cli_session_from_dir = path
            .parent()
            .map(|p| p.to_string_lossy().contains("/.codex/sessions"))
            .unwrap_or(false);

        // Determine if this is a CLI session or standard trace
        if is_cli_session_from_dir || Self::is_cli_session_file(path)? {
            // For CLI sessions, use the provided batch_reader for git operations
            // Note: verbose is currently always 0 here; for full verbose support would need to thread through trait
            self.parse_cli_session(path, file_pattern, repo_root, batch_reader, 0)
        } else {
            self.parse_standard_codex(path, file_pattern)
        }
    }

    fn parse_directory_with_context_verbose(
        &self,
        dir: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
        verbose: u8,
    ) -> Result<Vec<EditRecord>> {
        // Override default to provide per-file progress for Codex
        let mut all_edits = Vec::new();
        let files = self.collect_trace_files(dir)?;

        for (idx, file) in files.iter().enumerate() {
            let start_time = Instant::now();

            if verbose >= 3 {
                eprintln!(
                    "[*] Parsing codex file {}/{}: {:?}",
                    idx + 1,
                    files.len(),
                    file.file_name().unwrap_or_default()
                );
            }

            match self.parse_file_with_context(file, file_pattern, repo_root) {
                Ok(mut edits) => {
                    if verbose >= 3 {
                        let elapsed = start_time.elapsed();
                        eprintln!(
                            "    ✓ Parsed in {:.2}s ({} edits)",
                            elapsed.as_secs_f64(),
                            edits.len()
                        );
                    }
                    all_edits.append(&mut edits);
                }
                Err(e) => {
                    if verbose >= 1 {
                        eprintln!("Warning: Failed to parse {:?}: {}", file, e);
                    }
                }
            }
        }

        Ok(all_edits)
    }

    /// Override collect_trace_files to only return Codex-parseable files
    ///
    /// This ensures we don't track metadata for files we can't parse (like Claude files).
    fn collect_trace_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        crate::extractor::collect_jsonl_files(dir, &mut files)?;
        self.filter_parseable_files(files)
    }
}

/// Check if a JSON record is in Codex format
pub fn is_codex_format_check(record: &Value) -> bool {
    // Codex traces might have a different structure:
    // - {"event": "completion", "model": "gpt-...", ...}
    // - {"type": "completion", "choices": [...], ...}
    // - Presence of "choices" or "completion" fields typical of OpenAI API
    if record.get("choices").is_some() {
        return true;
    }
    if let Some(event) = record.get("event").and_then(|e| e.as_str()) {
        if event == "completion" || event == "edit" || event == "create" {
            return true;
        }
    }
    if let Some(model) = crate::parsers::common::extract_model_from_record(record) {
        if crate::parsers::common::is_codex_model(model) {
            return true;
        }
    }
    false
}

/// Check if a JSON record is a successful Codex edit
fn is_successful_codex_edit(record: &Value) -> bool {
    let has_file = record.get("file").is_some()
        || record.get("file_path").is_some()
        || record.get("filePath").is_some();

    if !has_file {
        return false;
    }

    // Check for error indicators
    if record.get("error").is_some() {
        return false;
    }
    if let Some(status) = record.get("status").and_then(|s| s.as_str()) {
        if status == "error" || status == "failed" {
            return false;
        }
    }

    // Must be an edit or completion event or have explicit edit content
    if let Some(event) = record.get("event").and_then(|e| e.as_str()) {
        if event == "edit" || event == "completion" || event == "create" {
            return true;
        }
    }

    if let Some(action) = record.get("action").and_then(|a| a.as_str()) {
        if action == "edit" || action == "create" || action == "modify" {
            return true;
        }
    }

    // Check for edit content
    let has_edit_content = record.get("content").is_some()
        || (record.get("old_content").is_some() && record.get("new_content").is_some())
        || (record.get("oldContent").is_some() && record.get("newContent").is_some());

    has_edit_content
}

/// Check if a JSON record is a ghost snapshot
fn is_ghost_snapshot(record: &Value) -> bool {
    record.get("type").and_then(|t| t.as_str()) == Some("response_item")
        && record
            .get("payload")
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str())
            == Some("ghost_snapshot")
}

/// Extract ghost commit ID from a snapshot record
fn extract_ghost_commit_id(record: &Value) -> Option<String> {
    record
        .get("payload")
        .and_then(|p| p.get("ghost_commit"))
        .and_then(|gc| gc.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
}

/// Extract files from a ghost commit
fn extract_ghost_commit_files(record: &Value) -> Vec<String> {
    record
        .get("payload")
        .and_then(|p| p.get("ghost_commit"))
        .and_then(|gc| gc.get("preexisting_untracked_files"))
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
