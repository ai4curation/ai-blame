use crate::cache::StalenessReport;
use crate::models::EditRecord;
use crate::parsers::{ParserInfo, TraceParser};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Parser for Claude Code trace files (.jsonl format)
pub struct ClaudeParser;

impl ClaudeParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract tool use ID from a tool result record
    fn extract_tool_use_id_from_tool_result_record(record: &Value) -> Option<String> {
        let content = record
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())?;
        for item in content {
            if item.get("type").and_then(|t| t.as_str()) != Some("tool_result") {
                continue;
            }
            if let Some(id) = item.get("tool_use_id").and_then(|v| v.as_str()) {
                return Some(id.to_string());
            }
        }
        None
    }

    /// Check if a tool result is a create operation
    fn tool_is_create(tool_result: &Value) -> bool {
        let tool_type = tool_result
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        let has_content = tool_result
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        tool_type == "create" || (tool_type.is_empty() && has_content)
    }

    /// Calculate the size of a change from tool result
    fn calculate_change_size(tool_result: &Value) -> usize {
        if Self::tool_is_create(tool_result) {
            let content = tool_result
                .get("content")
                .and_then(|c| c.as_str())
                .or_else(|| tool_result.get("newString").and_then(|c| c.as_str()))
                .unwrap_or("");
            return content.len();
        }

        let old_string = tool_result
            .get("oldString")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let new_string = tool_result
            .get("newString")
            .and_then(|s| s.as_str())
            .unwrap_or("");

        let len_diff = (new_string.len() as i64 - old_string.len() as i64).unsigned_abs() as usize;
        let max_len = old_string.len().max(new_string.len());
        len_diff + max_len
    }

    /// Parse a file with access to cross-file UUID resolution
    fn parse_file_with_cross_file_index(
        &self,
        path: &Path,
        file_pattern: &str,
        cross_file_models: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<EditRecord>> {
        let file =
            File::open(path).with_context(|| format!("Failed to open trace file: {:?}", path))?;
        let reader = BufReader::new(file);

        let mut models_by_uuid: HashMap<String, String> = HashMap::new();
        let mut models_by_tool_use_id: HashMap<String, String> = HashMap::new();

        struct PendingEdit {
            record: EditRecord,
            parent_uuid: Option<String>,
            tool_use_id: Option<String>,
        }

        let mut pending: Vec<PendingEdit> = Vec::new();

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

            // Capture model from parent messages (for cross-file UUID resolution)
            let model_for_record =
                crate::parsers::common::extract_model_from_record(&record).map(|s| s.to_string());
            if let Some(uuid) = record.get("uuid").and_then(|u| u.as_str()) {
                if let Some(model) = model_for_record.as_deref() {
                    models_by_uuid.insert(uuid.to_string(), model.to_string());
                }
            }
            if let Some(model) = model_for_record.as_deref() {
                for id in crate::parsers::common::extract_tool_use_ids_from_record(&record) {
                    models_by_tool_use_id.insert(id, model.to_string());
                }
            }

            // Check if this is a Codex-format edit (skip if so)
            if crate::parsers::codex::is_codex_format_check(&record) {
                continue;
            }

            if !Self::is_successful_edit(&record) {
                continue;
            }

            let tool_result = match record.get("toolUseResult") {
                Some(tr) => tr,
                None => continue,
            };

            let file_path = tool_result
                .get("filePath")
                .and_then(|fp| fp.as_str())
                .unwrap_or("");

            // Apply file pattern filter
            if !file_pattern.is_empty() && !file_path.contains(file_pattern) {
                continue;
            }

            let is_create = Self::tool_is_create(tool_result);
            let structured_patch = tool_result
                .get("structuredPatch")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            let old_string = tool_result
                .get("oldString")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let new_string = tool_result
                .get("newString")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let create_content = if is_create {
                tool_result
                    .get("content")
                    .and_then(|c| c.as_str())
                    .or_else(|| tool_result.get("newString").and_then(|c| c.as_str()))
                    .map(|s| s.to_string())
            } else {
                None
            };
            let change_size = Self::calculate_change_size(tool_result);

            let parent_uuid = record
                .get("parentUuid")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            let tool_use_id = Self::extract_tool_use_id_from_tool_result_record(&record);

            let agent_version = record
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse timestamp
            let timestamp = match record.get("timestamp").and_then(|t| t.as_str()) {
                Some(ts_str) => match DateTime::parse_from_rfc3339(ts_str) {
                    Ok(dt) => dt.with_timezone(&Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            let session_id = record
                .get("sessionId")
                .or_else(|| record.get("session_id"))
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();

            let mut model = "unknown".to_string();
            if let Some(m) = crate::parsers::common::extract_model_from_record(&record) {
                model = m.to_string();
            }

            let edit = EditRecord {
                file_path: file_path.to_string(),
                timestamp,
                model,
                session_id,
                is_create,
                change_size,
                agent_tool: Self::infer_agent_tool(path, &record),
                agent_version,
                old_string,
                new_string,
                structured_patch,
                create_content,
            };

            pending.push(PendingEdit {
                record: edit,
                parent_uuid,
                tool_use_id,
            });
        }

        // Post-pass: resolve unknown models using collected indices AND cross-file index
        let mut edits: Vec<EditRecord> = Vec::with_capacity(pending.len());
        for mut p in pending {
            if p.record.model == "unknown" {
                if let Some(pu) = p.parent_uuid.as_deref() {
                    if let Some(model) =
                        models_by_uuid.get(pu).or_else(|| cross_file_models.get(pu))
                    {
                        p.record.model = model.clone();
                    }
                } else if let Some(tu) = p.tool_use_id.as_deref() {
                    if let Some(model) = models_by_tool_use_id.get(tu) {
                        p.record.model = model.clone();
                    }
                }
            }
            edits.push(p.record);
        }

        Ok(edits)
    }

    /// Infer the agent tool from trace path and record
    fn infer_agent_tool(trace_path: &Path, record: &Value) -> String {
        if let Some(s) = record.get("agent_tool").and_then(|v| v.as_str()) {
            return s.to_string();
        }
        if let Some(s) = record.get("agentTool").and_then(|v| v.as_str()) {
            return s.to_string();
        }
        if let Some(s) = record.get("client").and_then(|v| v.as_str()) {
            return s.to_string();
        }

        let is_agent_file = trace_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("agent-"))
            .unwrap_or(false);
        if is_agent_file {
            "claude-code-agent".to_string()
        } else {
            "claude-code".to_string()
        }
    }

    /// Check if a record is a successful Claude edit
    fn is_successful_edit(record: &Value) -> bool {
        if record.get("type").and_then(|t| t.as_str()) != Some("user") {
            return false;
        }

        let tool_result = match record.get("toolUseResult") {
            Some(tr) => tr,
            None => return false,
        };

        if !tool_result.is_object() {
            return false;
        }

        if tool_result
            .get("is_error")
            .and_then(|e| e.as_bool())
            .unwrap_or(false)
        {
            return false;
        }
        if tool_result.get("error").is_some() {
            return false;
        }
        if tool_result
            .get("code")
            .and_then(|c| c.as_u64())
            .unwrap_or(200)
            >= 400
        {
            return false;
        }

        let file_path = tool_result
            .get("filePath")
            .and_then(|fp| fp.as_str())
            .unwrap_or("");
        if file_path.is_empty() {
            return false;
        }

        let has_patch = tool_result.get("structuredPatch").is_some();
        let has_old_new =
            tool_result.get("oldString").is_some() && tool_result.get("newString").is_some();
        let is_create = Self::tool_is_create(tool_result);

        has_patch || has_old_new || is_create
    }
}

impl Default for ClaudeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceParser for ClaudeParser {
    fn info(&self) -> ParserInfo {
        ParserInfo {
            name: "claude",
            description: "Parser for Claude Code trace files (.jsonl format)",
            file_extensions: vec!["jsonl"],
        }
    }

    fn can_parse(&self, path: &Path) -> Result<Option<bool>> {
        // Check file extension
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            return Ok(Some(false));
        }

        // Quick peek at first few records to determine format
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);
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

            // Claude traces have uuid and type/message fields
            if record.get("uuid").is_some()
                && (record.get("type").is_some() || record.get("message").is_some())
            {
                // Check it's NOT Codex format
                if crate::parsers::codex::is_codex_format_check(&record) {
                    return Ok(Some(false));
                }
                return Ok(Some(true));
            }
        }

        Ok(None)
    }

    fn parse_directory_with_context(
        &self,
        dir: &Path,
        file_pattern: &str,
        _repo_root: Option<&Path>,
    ) -> Result<Vec<EditRecord>> {
        // Override default to handle cross-file UUID resolution
        let mut all_edits = Vec::new();
        let files = self.collect_trace_files(dir)?;

        // First pass: collect cross-file UUID → model mappings
        let mut cross_file_models: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for file in &files {
            let Ok(file_obj) = std::fs::File::open(file) else {
                continue;
            };
            let reader = std::io::BufReader::new(file_obj);
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                let line = line.trim();
                if line.is_empty() || !line.starts_with('{') {
                    continue;
                }
                if let Ok(record) = serde_json::from_str::<Value>(line) {
                    if let Some(model) = crate::parsers::common::extract_model_from_record(&record)
                    {
                        if let Some(uuid) = record.get("uuid").and_then(|u| u.as_str()) {
                            cross_file_models.insert(uuid.to_string(), model.to_string());
                        }
                    }
                }
            }
        }

        // Second pass: parse each file with cross-file UUID index
        for file in files {
            match self.parse_file_with_cross_file_index(&file, file_pattern, &cross_file_models) {
                Ok(mut edits) => all_edits.append(&mut edits),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {:?}: {}", file, e);
                }
            }
        }

        Ok(all_edits)
    }

    fn parse_directory_with_context_verbose(
        &self,
        dir: &Path,
        file_pattern: &str,
        _repo_root: Option<&Path>,
        verbose: u8,
    ) -> Result<Vec<EditRecord>> {
        // Override default to handle cross-file UUID resolution with verbose output
        let mut all_edits = Vec::new();
        let files = self.collect_trace_files(dir)?;

        if verbose >= 3 {
            eprintln!("[*] Claude parser: Building cross-file UUID index...");
        }

        // First pass: collect cross-file UUID → model mappings
        let mut cross_file_models: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for file in &files {
            let Ok(file_obj) = std::fs::File::open(file) else {
                continue;
            };
            let reader = std::io::BufReader::new(file_obj);
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                let line = line.trim();
                if line.is_empty() || !line.starts_with('{') {
                    continue;
                }
                if let Ok(record) = serde_json::from_str::<Value>(line) {
                    if let Some(model) = crate::parsers::common::extract_model_from_record(&record)
                    {
                        if let Some(uuid) = record.get("uuid").and_then(|u| u.as_str()) {
                            cross_file_models.insert(uuid.to_string(), model.to_string());
                        }
                    }
                }
            }
        }

        // Second pass: parse each file with cross-file UUID index
        for (idx, file) in files.iter().enumerate() {
            let start_time = Instant::now();

            if verbose >= 3 {
                eprintln!(
                    "[*] Parsing claude file {}/{}: {:?}",
                    idx + 1,
                    files.len(),
                    file.file_name().unwrap_or_default()
                );
            }

            match self.parse_file_with_cross_file_index(file, file_pattern, &cross_file_models) {
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

    fn parse_file(&self, path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
        // Delegate to parse_file_with_cross_file_index with empty HashMap for consistency
        let empty_cross_file_models = std::collections::HashMap::new();
        self.parse_file_with_cross_file_index(path, file_pattern, &empty_cross_file_models)
    }

    /// Override collect_trace_files to only return Claude-parseable files
    ///
    /// This ensures we don't track metadata for files we can't parse (like Codex files).
    fn collect_trace_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        crate::extractor::collect_jsonl_files(dir, &mut files)?;
        self.filter_parseable_files(files)
    }

    /// Override directory staleness check for all-or-nothing invalidation
    ///
    /// Claude parser requires cross-file UUID resolution, so if ANY file in the directory
    /// is stale, we must reparse ALL files in that directory. This is conservative but correct.
    fn check_directory_staleness(
        &self,
        dir: &Path,
        cache: &crate::cache::CacheManager,
    ) -> Result<StalenessReport> {
        let files = self.collect_trace_files(dir)?;

        // Check if ANY file is stale
        let mut any_stale = false;
        for file in &files {
            let cached_meta = cache.get_file_metadata(file)?;
            if self.is_stale(file, cached_meta.as_ref())? {
                any_stale = true;
                break;
            }
        }

        if any_stale {
            // All files are stale (need to rebuild UUID index)
            Ok(StalenessReport {
                stale_files: files,
                fresh_files: vec![],
            })
        } else {
            // All files are fresh
            Ok(StalenessReport {
                stale_files: vec![],
                fresh_files: files,
            })
        }
    }
}
