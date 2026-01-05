use crate::models::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn extract_model_from_record(record: &Value) -> Option<&str> {
    // Common shapes:
    // - {"message":{"model":"..."}}
    // - {"model":"..."}
    // - {"toolUseResult":{"model":"..."}} (rare, but cheap to support)
    record
        .get("message")
        .and_then(|msg| msg.get("model"))
        .and_then(|m| m.as_str())
        .or_else(|| record.get("model").and_then(|m| m.as_str()))
        .or_else(|| {
            record
                .get("toolUseResult")
                .and_then(|tr| tr.get("model"))
                .and_then(|m| m.as_str())
        })
}

fn extract_tool_use_ids_from_record(record: &Value) -> Vec<String> {
    // Assistant messages can contain tool_use blocks like:
    // {"message":{"content":[{"type":"tool_use","id":"toolu_..."}]}}
    let mut out = Vec::new();
    let Some(content) = record
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return out;
    };
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
            continue;
        }
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            out.push(id.to_string());
        }
    }
    out
}

fn extract_tool_use_id_from_tool_result_record(record: &Value) -> Option<String> {
    // Tool result messages often look like:
    // {"message":{"content":[{"type":"tool_result","tool_use_id":"toolu_..."}]}}
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

fn is_codex_model(model: &str) -> bool {
    // Check for known OpenAI Codex and GPT model prefixes used by GitHub Copilot
    model.starts_with("codex-")
        || model.starts_with("gpt-4")
        || model.starts_with("gpt-3.5")
        || model.starts_with("gpt-35")
}

fn infer_agent_tool(trace_path: &Path, record: &Value) -> String {
    // Prefer explicit fields when present.
    if let Some(s) = record.get("agent_tool").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(s) = record.get("agentTool").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(s) = record.get("client").and_then(|v| v.as_str()) {
        return s.to_string();
    }

    // Check for Codex/Copilot indicators
    if let Some(model) = extract_model_from_record(record) {
        if is_codex_model(model) {
            return "github-copilot".to_string();
        }
    }

    // Claude Code subagent sessions are typically stored in files prefixed with `agent-`.
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

pub fn get_default_trace_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    crate::paths::resolve_claude_trace_dir(&home, &cwd)
}

pub fn get_codex_trace_dirs() -> Vec<PathBuf> {
    // Codex traces may be in various locations depending on the client:
    // - Codex Direct: ~/.codex/ (may have history.jsonl or sessions/ subdirectory)
    // - GitHub Copilot: typically stores telemetry in VS Code / IDE extensions
    // - Cursor IDE: ~/.cursor/traces or similar
    // - OpenAI API: custom locations depending on implementation
    let mut dirs = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // Common locations for Copilot/Codex traces
        let candidates = vec![
            home.join(".codex"),
            home.join(".codex").join("sessions"),
            home.join(".copilot").join("traces"),
            home.join(".config").join("github-copilot").join("traces"),
            home.join(".vscode").join("copilot").join("traces"),
            home.join(".cursor").join("traces"),
            home.join(".openai").join("traces"),
        ];

        for dir in candidates {
            if dir.exists() {
                dirs.push(dir);
            }
        }
    }

    dirs
}

pub fn get_all_trace_dirs(claude_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![claude_dir.to_path_buf()];
    dirs.extend(get_codex_trace_dirs());
    dirs
}

pub fn normalize_path(abs_path: &str, repo_root: Option<&str>) -> String {
    let repo_root = if let Some(r) = repo_root {
        r
    } else {
        // Get current dir as string at call site
        if let Ok(cwd) = std::env::current_dir() {
            if abs_path.starts_with(cwd.to_str().unwrap_or("")) {
                return abs_path[cwd.to_str().unwrap_or("").len()..]
                    .trim_start_matches('/')
                    .to_string();
            }
        }
        return abs_path.to_string();
    };

    if let Some(stripped) = abs_path.strip_prefix(repo_root) {
        stripped.trim_start_matches('/').to_string()
    } else {
        abs_path.to_string()
    }
}

fn is_codex_format(record: &Value) -> bool {
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
    if let Some(model) = extract_model_from_record(record) {
        if is_codex_model(model) {
            return true;
        }
    }
    false
}

fn is_successful_codex_edit(record: &Value) -> bool {
    // Check for Codex-style completion/edit records
    // Expected format (examples):
    // {"event": "edit", "file": "/path/to/file", "model": "gpt-4", "timestamp": "...", ...}
    // {"type": "completion", "file_path": "/path", "model": "codex", "action": "create", ...}

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

    // Fallback: require explicit edit-related content to treat as a successful edit
    let has_edit_content = record.get("content").is_some()
        || (record.get("old_content").is_some() && record.get("new_content").is_some())
        || (record.get("oldContent").is_some() && record.get("newContent").is_some());

    if has_edit_content {
        return true;
    }

    // Has a file but no clear edit indicators: do not treat as a successful edit
    false
}

fn is_ghost_snapshot(record: &Value) -> bool {
    record.get("type").and_then(|t| t.as_str()) == Some("response_item")
        && record
            .get("payload")
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str())
            == Some("ghost_snapshot")
}

fn extract_ghost_commit_id(record: &Value) -> Option<String> {
    record
        .get("payload")
        .and_then(|p| p.get("ghost_commit"))
        .and_then(|gc| gc.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
}

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

fn is_successful_edit(record: &Value) -> bool {
    if record.get("type").and_then(|t| t.as_str()) != Some("user") {
        return false;
    }

    let tool_result = match record.get("toolUseResult") {
        Some(tr) => tr,
        None => return false,
    };

    // toolUseResult can sometimes be a string (error message)
    if !tool_result.is_object() {
        return false;
    }

    // Check for error indicators
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

    // Must have a file path
    let file_path = tool_result
        .get("filePath")
        .and_then(|fp| fp.as_str())
        .unwrap_or("");
    if file_path.is_empty() {
        return false;
    }

    // Check if it's a create or edit
    let has_patch = tool_result.get("structuredPatch").is_some();
    let has_old_new =
        tool_result.get("oldString").is_some() && tool_result.get("newString").is_some();
    let is_create = tool_is_create(tool_result);

    has_patch || has_old_new || is_create
}

fn calculate_change_size(tool_result: &Value) -> usize {
    if tool_is_create(tool_result) {
        // For creates, use content length (fall back to newString when present).
        let content = tool_result
            .get("content")
            .and_then(|c| c.as_str())
            .or_else(|| tool_result.get("newString").and_then(|c| c.as_str()))
            .unwrap_or("");
        return content.len();
    }

    // For edits, calculate difference
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

fn calculate_change_size_from_strings(
    is_create: bool,
    content: Option<&str>,
    old_content: Option<&str>,
    new_content: Option<&str>,
) -> usize {
    if is_create {
        content.unwrap_or("").len()
    } else {
        let old_len = old_content.unwrap_or("").len();
        let new_len = new_content.unwrap_or("").len();
        (new_len as i64 - old_len as i64).unsigned_abs() as usize + old_len.max(new_len)
    }
}

fn parse_trace_file_with_model_index(
    trace_path: &Path,
    file_pattern: &str,
    model_index: Option<&HashMap<String, String>>,
) -> Result<Vec<EditRecord>> {
    let file = File::open(trace_path)
        .with_context(|| format!("Failed to open trace file: {:?}", trace_path))?;
    let reader = BufReader::new(file);

    // Keep a minimal index of parent UUID -> model.
    //
    // Previous implementations stored the full JSON record for every UUID, which can be
    // memory-heavy for large traces. We only need the parent model string for attribution.
    let mut models_by_uuid: HashMap<String, String> = HashMap::new();
    // Some tool result records don't link directly to the assistant UUID via parentUuid.
    // In those cases, we can link the tool_result.tool_use_id back to the assistant
    // tool_use.id, which lives in the assistant message content.
    let mut models_by_tool_use_id: HashMap<String, String> = HashMap::new();

    // Edits can reference parent messages that appear later in the file, so capture the parent
    // UUID and resolve the model in a post-pass.
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

        // Capture model from parent messages.
        let model_for_record = extract_model_from_record(&record).map(|s| s.to_string());
        if let Some(uuid) = record.get("uuid").and_then(|u| u.as_str()) {
            if let Some(model) = model_for_record.as_deref() {
                models_by_uuid.insert(uuid.to_string(), model.to_string());
            }
        }
        // Capture model for tool_use ids from assistant messages.
        if let Some(model) = model_for_record.as_deref() {
            for id in extract_tool_use_ids_from_record(&record) {
                models_by_tool_use_id.insert(id, model.to_string());
            }
        }

        // Check if this is a Codex-format edit
        let is_codex = is_codex_format(&record);

        if !is_codex && !is_successful_edit(&record) {
            continue;
        }
        if is_codex && !is_successful_codex_edit(&record) {
            continue;
        }

        let (
            file_path,
            is_create,
            structured_patch,
            old_string,
            new_string,
            create_content,
            change_size,
        ) = if is_codex {
            // Parse Codex-format record
            let file_path = record
                .get("file")
                .or_else(|| record.get("file_path"))
                .or_else(|| record.get("filePath"))
                .and_then(|fp| fp.as_str())
                .unwrap_or("");

            let action = record.get("action").and_then(|a| a.as_str());
            let event = record.get("event").and_then(|e| e.as_str());
            let is_create = action == Some("create")
                || event == Some("create")
                || record
                    .get("is_create")
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);

            let content = record
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            let old_content = record
                .get("old_content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            let new_content = record
                .get("new_content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            let patch = record
                .get("diff")
                .or_else(|| record.get("patch"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());

            let change_size = calculate_change_size_from_strings(
                is_create,
                content.as_deref(),
                old_content.as_deref(),
                new_content.as_deref(),
            );

            let create_content = if is_create { content } else { None };

            (
                file_path,
                is_create,
                patch,
                old_content,
                new_content,
                create_content,
                change_size,
            )
        } else {
            // Parse Claude-format record
            let tool_result = match record.get("toolUseResult") {
                Some(tr) => tr,
                None => continue,
            };
            let file_path = tool_result
                .get("filePath")
                .and_then(|fp| fp.as_str())
                .unwrap_or("");

            let is_create = tool_is_create(tool_result);
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
            let change_size = calculate_change_size(tool_result);

            (
                file_path,
                is_create,
                structured_patch,
                old_string,
                new_string,
                create_content,
                change_size,
            )
        };

        // Apply file pattern filter
        if !file_pattern.is_empty() && !file_path.contains(file_pattern) {
            continue;
        }

        let parent_uuid = record
            .get("parentUuid")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());
        let tool_use_id = if is_codex {
            None
        } else {
            extract_tool_use_id_from_tool_result_record(&record)
        };

        // Get agent version
        let agent_version = record
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse timestamp; skip records without a valid RFC3339 timestamp rather than
        // inventing "now" (which makes results nondeterministic and can reorder events).
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
        // If the record itself provides a model, prefer it; otherwise resolve via parent UUID.
        if let Some(m) = extract_model_from_record(&record) {
            model = m.to_string();
        } else if let Some(pu) = parent_uuid.as_deref() {
            if let Some(m) = models_by_uuid
                .get(pu)
                .or_else(|| model_index.and_then(|idx| idx.get(pu)))
            {
                model = m.clone();
            }
        } else if let Some(tu) = tool_use_id.as_deref() {
            if let Some(m) = models_by_tool_use_id.get(tu) {
                model = m.clone();
            }
        }

        let edit = EditRecord {
            file_path: file_path.to_string(),
            timestamp,
            model,
            session_id,
            is_create,
            change_size,
            agent_tool: infer_agent_tool(trace_path, &record),
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

    // Post-pass: fill in model strings from parent UUIDs (when available).
    let mut edits: Vec<EditRecord> = Vec::with_capacity(pending.len());
    for mut p in pending {
        if p.record.model == "unknown" {
            if let Some(pu) = p.parent_uuid.as_deref() {
                if let Some(model) = models_by_uuid
                    .get(pu)
                    .or_else(|| model_index.and_then(|idx| idx.get(pu)))
                {
                    p.record.model = model.clone();
                    edits.push(p.record);
                    continue;
                }
            }
            if let Some(tu) = p.tool_use_id.as_deref() {
                if let Some(model) = models_by_tool_use_id.get(tu) {
                    p.record.model = model.clone();
                }
            }
        }
        edits.push(p.record);
    }

    Ok(edits)
}

pub fn parse_trace_file(trace_path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
    parse_trace_file_with_model_index(trace_path, file_pattern, None)
}

pub fn parse_codex_cli_session(
    session_path: &Path,
    repo_root: Option<&Path>,
    file_pattern: &str,
) -> Result<Vec<EditRecord>> {
    let file = File::open(session_path)
        .with_context(|| format!("Failed to open Codex session file: {:?}", session_path))?;
    let reader = BufReader::new(file);

    // Collect all ghost snapshots with their file states
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

        // Extract model from turn_context if available
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

    // Compare successive snapshots to find file changes
    let mut edits: Vec<EditRecord> = Vec::new();

    for i in 1..snapshots.len() {
        let prev_snapshot = &snapshots[i - 1];
        let curr_snapshot = &snapshots[i];
        let prev_files = &prev_snapshot.files;
        let curr_files = &curr_snapshot.files;

        if let Some(repo_path) = repo_root {
            // Find added files
            for file in curr_files {
                if !prev_files.contains(file) {
                    // File was added
                    if !file_pattern.is_empty() && !file.contains(file_pattern) {
                        continue;
                    }

                    // Try to get file content from git
                    if let Ok(content) =
                        get_git_file_content(repo_path, &curr_snapshot.commit_id, file)
                    {
                        let edit = EditRecord {
                            file_path: file.to_string(),
                            timestamp: curr_snapshot.timestamp,
                            model: model.clone(),
                            session_id: session_path
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
                    }
                }
            }

            // Find modified files (exist in both snapshots but content changed)
            for file in curr_files {
                if prev_files.contains(file) {
                    // File existed before, check if it was modified
                    if !file_pattern.is_empty() && !file.contains(file_pattern) {
                        continue;
                    }

                    let prev_content =
                        get_git_file_content(repo_path, &prev_snapshot.commit_id, file).ok();
                    let curr_content =
                        get_git_file_content(repo_path, &curr_snapshot.commit_id, file).ok();

                    // If content differs, record the modification
                    if let (Some(prev), Some(curr)) = (prev_content, curr_content) {
                        if prev != curr {
                            let change_size = (curr.len() as i64 - prev.len() as i64).unsigned_abs()
                                as usize
                                + prev.len().max(curr.len());

                            let edit = EditRecord {
                                file_path: file.to_string(),
                                timestamp: curr_snapshot.timestamp,
                                model: model.clone(),
                                session_id: session_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                is_create: false,
                                change_size,
                                agent_tool: "codex-cli".to_string(),
                                agent_version: None,
                                old_string: Some(prev),
                                new_string: Some(curr),
                                structured_patch: None,
                                create_content: None,
                            };
                            edits.push(edit);
                        }
                    }
                }
            }
        }
    }

    Ok(edits)
}

fn get_git_file_content(repo_root: &Path, commit_id: &str, file_path: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("git")
        .arg("show")
        .arg(format!("{}:{}", commit_id, file_path))
        .current_dir(repo_root)
        .output()
        .context("Failed to run git show")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Fall back to reading from filesystem for ghost commits
        // Ghost commits from Codex CLI sessions don't exist in the git repository

        // Validate that the file path doesn't contain path traversal components
        // to prevent security vulnerabilities
        if file_path.contains("..") || file_path.starts_with('/') {
            anyhow::bail!(
                "Invalid file path contains path traversal components: {}",
                file_path
            );
        }

        let file_path_abs = repo_root.join(file_path);

        // Double-check that the joined path is actually within the repository
        // by comparing the canonicalized repository root with the parent directories
        let canonical_repo = repo_root
            .canonicalize()
            .context("Failed to canonicalize repository root")?;

        // For files that exist, we can validate using canonicalize
        if file_path_abs.exists() {
            let canonical_file = file_path_abs.canonicalize().with_context(|| {
                format!("Failed to canonicalize file path: {:?}", file_path_abs)
            })?;

            if !canonical_file.starts_with(&canonical_repo) {
                anyhow::bail!(
                    "Path traversal detected: file {:?} is outside repository root {:?}",
                    canonical_file,
                    canonical_repo
                );
            }

            std::fs::read_to_string(&canonical_file).with_context(|| {
                format!("Failed to read file from filesystem: {:?}", canonical_file)
            })
        } else {
            anyhow::bail!(
                "git show failed for {}:{} and file does not exist on filesystem",
                commit_id,
                file_path
            )
        }
    }
}

pub fn extract_edit_history(trace_dir: &Path, config: &FilterConfig) -> Result<EditsByFile> {
    extract_edit_history_from_dirs(&[trace_dir], config, None)
}

pub fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    collect_jsonl_files_internal(dir, files, &mut visited)
}

fn collect_jsonl_files_internal(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) -> std::io::Result<()> {
    // Detect symlink cycles by tracking canonicalized directory paths
    let canonical_dir = match std::fs::canonicalize(dir) {
        Ok(p) => p,
        Err(_) => {
            // If we can't canonicalize, skip to avoid infinite recursion
            return Ok(());
        }
    };

    if !visited.insert(canonical_dir) {
        // We've already visited this directory (symlink cycle detected)
        return Ok(());
    }

    // Recursively collect all .jsonl files in a directory tree
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively process subdirectories
            let _ = collect_jsonl_files_internal(&path, files, visited);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    Ok(())
}

pub fn extract_edit_history_from_dirs(
    trace_dirs: &[&Path],
    config: &FilterConfig,
    repo_root: Option<&Path>,
) -> Result<EditsByFile> {
    extract_edit_history_from_dirs_verbose(trace_dirs, config, repo_root, 0)
}

pub fn extract_edit_history_from_dirs_verbose(
    trace_dirs: &[&Path],
    config: &FilterConfig,
    repo_root: Option<&Path>,
    verbose: u8,
) -> Result<EditsByFile> {
    let mut edits_by_file: EditsByFile = HashMap::new();

    let file_pattern = config.file_pattern.as_deref().unwrap_or("");
    let registry = crate::parsers::ParserRegistry::new();
    let cache_enabled = std::env::var("AI_BLAME_NO_CACHE").is_err();

    for trace_dir in trace_dirs {
        if !trace_dir.exists() {
            continue;
        }

        // Open cache for this specific trace directory (one cache per agent's traces)
        let cache = if cache_enabled {
            crate::cache::CacheManager::open(trace_dir).ok()
        } else {
            None
        };

        // Detect which providers have traces in this directory
        let mut providers_in_dir = std::collections::HashSet::new();
        if verbose >= 2 {
            let mut trace_files = Vec::new();
            if collect_jsonl_files(trace_dir, &mut trace_files).is_ok() {
                let mut files_by_parser: std::collections::HashMap<&str, Vec<_>> =
                    std::collections::HashMap::new();

                for file in &trace_files {
                    if let Ok(Some(parser)) =
                        registry.find_parser(file).map(|p| p.map(|p| p.info().name))
                    {
                        files_by_parser
                            .entry(parser)
                            .or_insert_with(Vec::new)
                            .push(file);
                        providers_in_dir.insert(parser);
                    }
                }

                for (parser_name, files) in &files_by_parser {
                    eprintln!(
                        "[*] Found {} {} trace files in {:?}",
                        files.len(),
                        parser_name,
                        trace_dir
                    );
                }
            }
        } else {
            // Without verbose, still detect providers to only process relevant parsers
            let mut trace_files = Vec::new();
            if collect_jsonl_files(trace_dir, &mut trace_files).is_ok() {
                for file in &trace_files {
                    if let Ok(Some(parser)) =
                        registry.find_parser(file).map(|p| p.map(|p| p.info().name))
                    {
                        providers_in_dir.insert(parser);
                    }
                }
            }
        }

        // Only process providers that actually have traces in this directory
        for parser in registry.parsers() {
            let provider = parser.info().name;

            // Skip this provider if no traces for it in this directory
            if !providers_in_dir.is_empty() && !providers_in_dir.contains(provider) {
                continue;
            }

            if verbose >= 2 {
                eprintln!("[*] Parsing {} traces from {:?}...", provider, trace_dir);
            }

            // Check staleness and load from cache if available
            if let Some(ref cache) = cache {
                if verbose >= 2 {
                    eprintln!("[*] Checking {} cache...", provider);
                }

                // Check which files are stale vs fresh
                let staleness = match parser.check_directory_staleness(trace_dir, cache) {
                    Ok(report) => report,
                    Err(e) => {
                        if verbose >= 1 {
                            eprintln!(
                                "Warning: Cache staleness check failed ({}), reparsing all files",
                                e
                            );
                        }
                        // Fall back to reparsing all
                        let all_files = match parser.collect_trace_files(trace_dir) {
                            Ok(files) => files,
                            Err(_) => continue,
                        };
                        crate::cache::StalenessReport {
                            stale_files: all_files,
                            fresh_files: vec![],
                        }
                    }
                };

                if verbose >= 2 && (staleness.fresh_files.len() + staleness.stale_files.len() > 0) {
                    eprintln!(
                        "  {} fresh, {} stale",
                        staleness.fresh_files.len(),
                        staleness.stale_files.len()
                    );
                }

                // Load fresh files from cache
                // Detect caching strategy:
                // - All-or-nothing (Claude): All edits under first file, rest have empty edits
                // - Per-file (Codex): Each file has its own edits
                let is_all_or_nothing = if staleness.fresh_files.len() > 1 {
                    // Check if remaining files (after first) all have empty cached edits
                    staleness.fresh_files[1..].iter().all(|file| {
                        cache
                            .get_cached_edits(file)
                            .ok()
                            .and_then(|opt| opt)
                            .map(|edits| edits.is_empty())
                            .unwrap_or(false)
                    })
                } else {
                    // Only one file or no files - doesn't matter
                    false
                };

                if is_all_or_nothing {
                    // All-or-nothing: Load only from first file
                    if let Some(first_file) = staleness.fresh_files.first() {
                        if let Ok(Some(cached_edits)) = cache.get_cached_edits(first_file) {
                            for edit in cached_edits {
                                // Apply time filters
                                if let Some(since) = config.since {
                                    if edit.timestamp < since {
                                        continue;
                                    }
                                }
                                if let Some(until) = config.until {
                                    if edit.timestamp > until {
                                        continue;
                                    }
                                }

                                edits_by_file
                                    .entry(edit.file_path.clone())
                                    .or_default()
                                    .push(edit);
                            }
                        }
                    }
                } else {
                    // Per-file: Load from each fresh file independently
                    for file in &staleness.fresh_files {
                        if let Ok(Some(cached_edits)) = cache.get_cached_edits(file) {
                            for edit in cached_edits {
                                // Apply time filters
                                if let Some(since) = config.since {
                                    if edit.timestamp < since {
                                        continue;
                                    }
                                }
                                if let Some(until) = config.until {
                                    if edit.timestamp > until {
                                        continue;
                                    }
                                }

                                edits_by_file
                                    .entry(edit.file_path.clone())
                                    .or_default()
                                    .push(edit);
                            }
                        }
                    }
                }

                // Parse stale files
                // IMPORTANT: If ALL files are stale (all-or-nothing invalidation for Claude),
                // parse the whole directory at once to preserve cross-file UUID resolution.
                // Otherwise parse files individually (per-file invalidation for Codex).
                if !staleness.stale_files.is_empty() {
                    let all_files = parser.collect_trace_files(trace_dir)?;
                    let all_stale = staleness.stale_files.len() == all_files.len()
                        && staleness.fresh_files.is_empty();

                    if all_stale {
                        // All files are stale: parse entire directory for correct cross-file resolution.
                        // Cache the edits under the first stale file, and mark remaining files as checked with empty results.
                        let start = std::time::Instant::now();
                        match parser.parse_directory_with_context(
                            trace_dir,
                            file_pattern,
                            repo_root,
                        ) {
                            Ok(parsed_edits) => {
                                let elapsed = start.elapsed();

                                // Cache parsed edits under FIRST file only to avoid duplication
                                // When loading, we check the first file for all-or-nothing results
                                if let Some(first_file) = staleness.stale_files.first() {
                                    if let Err(e) = cache.store_edits(
                                        first_file,
                                        provider,
                                        &parsed_edits,
                                        elapsed.as_millis() as u64,
                                    ) {
                                        if verbose >= 1 {
                                            eprintln!(
                                                "Warning: Failed to cache {:?}: {}",
                                                first_file, e
                                            );
                                        }
                                    }
                                }
                                // For other stale files, store empty edits to mark them as checked
                                for file in staleness.stale_files.iter().skip(1) {
                                    if let Err(e) = cache.store_edits(
                                        file,
                                        provider,
                                        &[],
                                        elapsed.as_millis() as u64,
                                    ) {
                                        if verbose >= 1 {
                                            eprintln!(
                                                "Warning: Failed to update cache for {:?}: {}",
                                                file, e
                                            );
                                        }
                                    }
                                }

                                if verbose >= 2 && !parsed_edits.is_empty() {
                                    eprintln!(
                                        "  [Cached {} edits from all-or-nothing parse]",
                                        parsed_edits.len()
                                    );
                                }

                                // Add to results
                                for edit in parsed_edits {
                                    // Apply time filters
                                    if let Some(since) = config.since {
                                        if edit.timestamp < since {
                                            continue;
                                        }
                                    }
                                    if let Some(until) = config.until {
                                        if edit.timestamp > until {
                                            continue;
                                        }
                                    }

                                    edits_by_file
                                        .entry(edit.file_path.clone())
                                        .or_default()
                                        .push(edit);
                                }

                                if verbose >= 3 {
                                    eprintln!(
                                        "    ✓ Parsed directory in {:.2}s",
                                        elapsed.as_secs_f64()
                                    );
                                }
                            }
                            Err(e) => {
                                if verbose >= 1 {
                                    eprintln!(
                                        "Warning: Failed to parse directory {:?}: {}",
                                        trace_dir, e
                                    );
                                }
                            }
                        }
                    } else {
                        // Only some files are stale: parse them individually (Codex per-file staleness)
                        for file in &staleness.stale_files {
                            let start = std::time::Instant::now();

                            match parser.parse_file_with_context(file, file_pattern, repo_root) {
                                Ok(edits) => {
                                    let elapsed = start.elapsed();

                                    // Store in cache
                                    if let Err(e) = cache.store_edits(
                                        file,
                                        provider,
                                        &edits,
                                        elapsed.as_millis() as u64,
                                    ) {
                                        if verbose >= 1 {
                                            eprintln!("Warning: Failed to cache {:?}: {}", file, e);
                                        }
                                    }

                                    // Add to results
                                    for edit in edits {
                                        // Apply time filters
                                        if let Some(since) = config.since {
                                            if edit.timestamp < since {
                                                continue;
                                            }
                                        }
                                        if let Some(until) = config.until {
                                            if edit.timestamp > until {
                                                continue;
                                            }
                                        }

                                        edits_by_file
                                            .entry(edit.file_path.clone())
                                            .or_default()
                                            .push(edit);
                                    }

                                    if verbose >= 3 {
                                        eprintln!("    ✓ Parsed in {:.2}s", elapsed.as_secs_f64());
                                    }
                                }
                                Err(e) => {
                                    if verbose >= 1 {
                                        eprintln!("Warning: Failed to parse {:?}: {}", file, e);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // No cache available: parse all files normally
                // Use verbose method for detailed per-file progress
                let edits = if verbose >= 3 {
                    parser.parse_directory_with_context_verbose(
                        trace_dir,
                        file_pattern,
                        repo_root,
                        verbose,
                    )
                } else {
                    parser.parse_directory_with_context(trace_dir, file_pattern, repo_root)
                };

                if let Ok(edits) = edits {
                    if verbose >= 2 && !edits.is_empty() {
                        eprintln!("[✓] Found {} edits from {}", edits.len(), provider);
                    }

                    for edit in edits {
                        // Apply time filters
                        if let Some(since) = config.since {
                            if edit.timestamp < since {
                                continue;
                            }
                        }
                        if let Some(until) = config.until {
                            if edit.timestamp > until {
                                continue;
                            }
                        }

                        edits_by_file
                            .entry(edit.file_path.clone())
                            .or_default()
                            .push(edit);
                    }
                }
            }
        }
    }

    // Sort by timestamp within each file
    for edits in edits_by_file.values_mut() {
        edits.sort_by_key(|e| e.timestamp);
    }

    Ok(edits_by_file)
}

pub fn apply_filters(edits_by_file: EditsByFile, config: &FilterConfig) -> EditsByFile {
    let mut filtered: EditsByFile = HashMap::new();

    for (file_path, mut edits) in edits_by_file {
        if edits.is_empty() {
            continue;
        }

        // Apply size filter
        if config.min_change_size > 0 {
            edits.retain(|e| e.change_size >= config.min_change_size);
        }

        if edits.is_empty() {
            continue;
        }

        // Apply initial_and_recent_only filter
        if config.initial_and_recent_only && edits.len() > 2 {
            let first = edits[0].clone();
            let last = edits[edits.len() - 1].clone();
            edits = vec![first, last];
        }

        filtered.insert(file_path, edits);
    }

    filtered
}

pub fn convert_to_file_histories(
    edits_by_file: EditsByFile,
    repo_root: Option<&str>,
) -> HistoriesByFile {
    let mut histories: HistoriesByFile = HashMap::new();

    for (abs_path, edits) in edits_by_file {
        let rel_path = normalize_path(&abs_path, repo_root);

        let events: Vec<CurationEvent> = edits
            .iter()
            .enumerate()
            .map(|(i, edit)| {
                let action = if i == 0 && edit.is_create {
                    Some(CurationAction::Created)
                } else {
                    Some(CurationAction::Edited)
                };

                CurationEvent {
                    timestamp: edit.timestamp,
                    model: Some(edit.model.clone()),
                    action,
                    description: None,
                    agent_tool: Some(edit.agent_tool.clone()),
                    agent_version: edit.agent_version.clone(),
                }
            })
            .collect();

        histories.insert(
            rel_path.clone(),
            FileHistory {
                file_path: rel_path,
                events,
            },
        );
    }

    histories
}

/// Collect timeline events from trace directories
///
/// This gathers all edits, flattens them into TimelineEvent structs,
/// filters by criteria (skip_codex), sorts by timestamp descending,
/// and applies a limit.
pub fn collect_timeline_events(
    trace_dirs: &[&Path],
    config: &FilterConfig,
    skip_codex: bool,
    limit: usize,
) -> Result<Vec<TimelineEvent>> {
    // Get edits using existing library function
    let edits_by_file = extract_edit_history_from_dirs(trace_dirs, config, None)?;

    // Flatten to timeline events
    let mut events: Vec<TimelineEvent> = edits_by_file
        .values()
        .flat_map(|edits| {
            edits.iter().map(|edit| TimelineEvent {
                timestamp: edit.timestamp,
                action: if edit.is_create {
                    "CREATED".to_string()
                } else {
                    "EDITED".to_string()
                },
                file_path: edit.file_path.clone(),
                model: edit.model.clone(),
                agent_tool: edit.agent_tool.clone(),
                agent_version: edit.agent_version.clone(),
                change_size: edit.change_size,
            })
        })
        .collect();

    // Filter Codex/Copilot if requested
    if skip_codex {
        events.retain(|e| {
            !e.agent_tool.to_lowercase().contains("copilot")
                && !e.agent_tool.to_lowercase().contains("codex")
        });
    }

    // Sort by timestamp descending (most recent first)
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit (0 means no limit)
    if limit > 0 && events.len() > limit {
        events.truncate(limit);
    }

    Ok(events)
}
