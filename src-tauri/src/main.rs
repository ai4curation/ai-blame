#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state for caching across Tauri commands.
///
/// This infrastructure enables future optimizations:
/// - Shared cache managers avoid re-opening DuckDB on each command
/// - Pre-loaded edit histories can be reused across blame_file calls
///
/// Currently, each command uses the library's built-in caching via
/// `extract_edit_history`, which already persists to `.ai-blame.ddb`.
/// The async commands prevent UI freezing; this state enables further
/// optimization by keeping cache connections open in memory.
#[allow(dead_code)]
pub struct AppState {
    /// Cache managers keyed by trace directory path
    caches: Mutex<HashMap<PathBuf, Arc<ai_blame::cache::CacheManager>>>,
}

#[allow(dead_code)]
impl AppState {
    fn new() -> Self {
        Self {
            caches: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create a cache manager for the given trace directory
    async fn get_cache(&self, trace_dir: &Path) -> Option<Arc<ai_blame::cache::CacheManager>> {
        let mut caches = self.caches.lock().await;

        if let Some(cache) = caches.get(trace_dir) {
            return Some(Arc::clone(cache));
        }

        // Create new cache for this trace directory
        match ai_blame::cache::CacheManager::open(trace_dir) {
            Ok(cache) => {
                let cache = Arc::new(cache);
                caches.insert(trace_dir.to_path_buf(), Arc::clone(&cache));
                Some(cache)
            }
            Err(e) => {
                eprintln!("Warning: Failed to open cache for {:?}: {}", trace_dir, e);
                None
            }
        }
    }
}

#[derive(Serialize)]
struct AppInfo {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct ProjectFilesResult {
    project_dir: String,
    files: Vec<String>,
}

#[derive(Serialize)]
struct TraceScanResult {
    trace_dir: String,
    trace_count: usize,
}

#[derive(Serialize)]
struct UiBlameMeta {
    timestamp: String,
    model: String,
    session_id: String,
    agent_tool: String,
    agent_version: Option<String>,
}

#[derive(Serialize)]
struct UiLineBlame {
    line_no: usize,
    text: String,
    meta: Option<UiBlameMeta>,
}

#[derive(Serialize)]
struct BlameFileResult {
    file_path: String,
    line_count: usize,
    lines: Vec<UiLineBlame>,
}

#[derive(Serialize)]
struct AgentTouchedFilesResult {
    files: Vec<String>,
}

// Transcript-related structs
#[derive(Serialize)]
struct UiTranscriptSummary {
    session_id: String,
    agent_tool: String,
    slug: Option<String>,
    start_time: String,
    end_time: Option<String>,
    message_count: usize,
    files_touched: usize,
    primary_model: Option<String>,
    source_file: String,
}

#[derive(Serialize)]
struct ListTranscriptsResult {
    transcripts: Vec<UiTranscriptSummary>,
    total_count: usize,
}

#[derive(Serialize)]
struct UiMatchSnippet {
    role: String,
    timestamp: String,
    block_type: String,
    snippet: String,
}

#[derive(Serialize)]
struct UiSearchMatch {
    transcript: UiTranscriptSummary,
    matches: Vec<UiMatchSnippet>,
}

#[derive(Serialize)]
struct SearchTranscriptsResult {
    matching_transcripts: Vec<UiSearchMatch>,
    total_matches: usize,
}

#[derive(Serialize)]
struct UiTokenUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_creation_tokens: Option<u64>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Code {
        code: String,
        language: Option<String>,
    },
    FileOperation {
        operation: String,
        file_path: String,
        content: Option<String>,
        old_content: Option<String>,
    },
    Command {
        command: String,
        output: Option<String>,
        exit_code: Option<i32>,
    },
}

#[derive(Serialize)]
struct UiTranscriptMessage {
    id: String,
    role: String,
    timestamp: String,
    content: Vec<UiContentBlock>,
    model: Option<String>,
    usage: Option<UiTokenUsage>,
}

#[derive(Serialize)]
struct UiTranscriptMeta {
    session_id: String,
    agent_tool: String,
    agent_version: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    slug: Option<String>,
    start_time: String,
    end_time: Option<String>,
    source_file: Option<String>,
}

#[derive(Serialize)]
struct UiTranscriptStats {
    message_count: usize,
    user_message_count: usize,
    assistant_message_count: usize,
    tool_use_count: usize,
    files_touched: usize,
    total_input_tokens: Option<u64>,
    total_output_tokens: Option<u64>,
}

#[derive(Serialize)]
struct UiTranscript {
    meta: UiTranscriptMeta,
    messages: Vec<UiTranscriptMessage>,
    stats: UiTranscriptStats,
}

#[derive(Serialize)]
struct UiTimelineEvent {
    timestamp: String, // RFC3339 format
    action: String,    // "CREATED" or "EDITED"
    file_path: String,
    model: String,
    agent_tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_version: Option<String>,
    change_size: usize,
}

#[derive(Serialize)]
struct ListTimelineResult {
    events: Vec<UiTimelineEvent>,
    total_count: usize,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "AI Blame".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn resolve_trace_dir_for_project(project_dir: Option<PathBuf>) -> PathBuf {
    let resolved_target = project_dir
        .map(|p| p.canonicalize().unwrap_or(p))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let resolved_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    ai_blame::paths::resolve_claude_trace_dir(&resolved_home, &resolved_target)
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".venv" | ".idea" | ".vscode"
    )
}

fn list_files_recursive(project_dir: &Path, base_dir: &Path, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let entries = match std::fs::read_dir(base_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in entries.filter_map(Result::ok) {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if should_skip_dir(name) {
                    continue;
                }
            }
            list_files_recursive(project_dir, &path, out, limit);
        } else if file_type.is_file() {
            // Only show reasonably small text-like files in the prototype.
            let is_small = entry
                .metadata()
                .map(|m| m.len() <= 512 * 1024)
                .unwrap_or(false);
            if !is_small {
                continue;
            }
            let rel = match path.strip_prefix(project_dir) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => continue,
            };
            // Skip hidden files and vendored lockfiles by default.
            if rel.starts_with('.') || rel.contains("/.") || rel.ends_with(".lock") {
                continue;
            }
            out.push(rel);
        }
    }
}

#[tauri::command]
fn pick_project_dir() -> Option<String> {
    tauri::api::dialog::blocking::FileDialogBuilder::new()
        .set_title("Select project directory")
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
async fn list_project_files(project_dir: String) -> Result<ProjectFilesResult, String> {
    // Move file system traversal to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = PathBuf::from(project_dir);
        let project_dir = project_dir
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;

        let mut files = Vec::new();
        list_files_recursive(&project_dir, &project_dir, &mut files, usize::MAX);
        files.sort();

        Ok(ProjectFilesResult {
            project_dir: project_dir.to_string_lossy().to_string(),
            files,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
fn scan_traces(project_dir: Option<String>) -> TraceScanResult {
    let project_dir = project_dir
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
        .map(PathBuf::from);
    let trace_dir = resolve_trace_dir_for_project(project_dir);

    let trace_count = std::fs::read_dir(&trace_dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jsonl"))
                .count()
        })
        .unwrap_or(0);

    TraceScanResult {
        trace_dir: trace_dir.to_string_lossy().to_string(),
        trace_count,
    }
}

#[tauri::command]
async fn list_agent_touched_files(project_dir: String) -> Result<AgentTouchedFilesResult, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = PathBuf::from(project_dir);
        let project_dir = project_dir
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;
        let project_dir_str = project_dir.to_string_lossy().to_string();

        let trace_dir = resolve_trace_dir_for_project(Some(project_dir.clone()));

        let filter = ai_blame::models::FilterConfig::default();
        let edits_by_file = ai_blame::extractor::extract_edit_history(&trace_dir, &filter)
            .map_err(|e| {
                format!(
                    "Failed to parse traces in {}: {e}",
                    trace_dir.to_string_lossy()
                )
            })?;

        // Extract all files that have edits, normalized to be relative to the project dir
        let mut files: Vec<String> = edits_by_file
            .keys()
            .map(|abs_path| ai_blame::extractor::normalize_path(abs_path, Some(&project_dir_str)))
            .collect();
        files.sort();
        files.dedup();

        Ok(AgentTouchedFilesResult { files })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn blame_file(project_dir: String, file_path: String) -> Result<BlameFileResult, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = PathBuf::from(project_dir);
        let project_dir = project_dir
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;
        let project_dir_str = project_dir.to_string_lossy().to_string();

        let abs_file = project_dir.join(&file_path);
        let current_content = std::fs::read_to_string(&abs_file)
            .map_err(|e| format!("Failed to read file {file_path}: {e}"))?;

        let trace_dir = resolve_trace_dir_for_project(Some(project_dir.clone()));

        let filter = ai_blame::models::FilterConfig::default();
        let edits_by_file = ai_blame::extractor::extract_edit_history(&trace_dir, &filter)
            .map_err(|e| {
                format!(
                    "Failed to parse traces in {}: {e}",
                    trace_dir.to_string_lossy()
                )
            })?;

        // Extract edits for the requested file by normalizing trace paths relative to the project dir.
        let mut edits_for_file = Vec::new();
        for (abs_path, edits) in edits_by_file {
            let rel = ai_blame::extractor::normalize_path(&abs_path, Some(&project_dir_str));
            if rel == file_path {
                edits_for_file = edits;
                break;
            }
        }

        let blamed = ai_blame::blame::compute_line_blame(&current_content, &edits_for_file)
            .map_err(|e| format!("Failed to compute blame for {file_path}: {e}"))?;

        let lines: Vec<UiLineBlame> = blamed
            .into_iter()
            .map(|l| UiLineBlame {
                line_no: l.line_no,
                text: l.text,
                meta: l.meta.map(|m| UiBlameMeta {
                    timestamp: m.timestamp.to_rfc3339(),
                    model: m.model,
                    session_id: m.session_id,
                    agent_tool: m.agent_tool,
                    agent_version: m.agent_version,
                }),
            })
            .collect();

        Ok(BlameFileResult {
            file_path,
            line_count: lines.len(),
            lines,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

fn convert_content_block(block: &ai_blame::transcript::ContentBlock) -> UiContentBlock {
    use ai_blame::transcript::ContentBlock;
    match block {
        ContentBlock::Text { text } => UiContentBlock::Text { text: text.clone() },
        ContentBlock::Thinking { thinking } => UiContentBlock::Thinking {
            thinking: thinking.clone(),
        },
        ContentBlock::ToolUse { id, name, input } => UiContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: input.clone(),
        },
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => UiContentBlock::ToolResult {
            tool_use_id: tool_use_id.clone(),
            content: content.clone(),
            is_error: *is_error,
        },
        ContentBlock::Code { code, language } => UiContentBlock::Code {
            code: code.clone(),
            language: language.clone(),
        },
        ContentBlock::FileOperation {
            operation,
            file_path,
            content,
            old_content,
        } => UiContentBlock::FileOperation {
            operation: operation.to_string(),
            file_path: file_path.clone(),
            content: content.clone(),
            old_content: old_content.clone(),
        },
        ContentBlock::Command {
            command,
            output,
            exit_code,
        } => UiContentBlock::Command {
            command: command.clone(),
            output: output.clone(),
            exit_code: *exit_code,
        },
    }
}

fn convert_transcript(transcript: ai_blame::transcript::Transcript) -> UiTranscript {
    use ai_blame::transcript::Role;

    let messages: Vec<UiTranscriptMessage> = transcript
        .messages
        .iter()
        .map(|m| UiTranscriptMessage {
            id: m.id.clone(),
            role: match m.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::System => "system".to_string(),
            },
            timestamp: m.timestamp.to_rfc3339(),
            content: m.content.iter().map(convert_content_block).collect(),
            model: m.model.clone(),
            usage: m.usage.as_ref().map(|u| UiTokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                cache_read_tokens: u.cache_read_tokens,
                cache_creation_tokens: u.cache_creation_tokens,
            }),
        })
        .collect();

    UiTranscript {
        meta: UiTranscriptMeta {
            session_id: transcript.meta.session_id,
            agent_tool: transcript.meta.agent_tool,
            agent_version: transcript.meta.agent_version,
            cwd: transcript.meta.cwd,
            git_branch: transcript.meta.git_branch,
            slug: transcript.meta.slug,
            start_time: transcript.meta.start_time.to_rfc3339(),
            end_time: transcript.meta.end_time.map(|t| t.to_rfc3339()),
            source_file: transcript.meta.source_file,
        },
        messages,
        stats: UiTranscriptStats {
            message_count: transcript.stats.message_count,
            user_message_count: transcript.stats.user_message_count,
            assistant_message_count: transcript.stats.assistant_message_count,
            tool_use_count: transcript.stats.tool_use_count,
            files_touched: transcript.stats.files_touched,
            total_input_tokens: transcript.stats.total_input_tokens,
            total_output_tokens: transcript.stats.total_output_tokens,
        },
    }
}

#[tauri::command]
async fn list_timeline(
    project_dir: Option<String>,
    skip_codex: Option<bool>,
    limit: Option<usize>,
    file_pattern: Option<String>,
) -> Result<ListTimelineResult, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = project_dir
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
            .map(PathBuf::from);
        let trace_dir = resolve_trace_dir_for_project(project_dir);

        let config = ai_blame::models::FilterConfig {
            file_pattern: file_pattern.filter(|s| !s.is_empty()),
            ..Default::default()
        };

        let events = ai_blame::extractor::collect_timeline_events(
            &[trace_dir.as_path()],
            &config,
            skip_codex.unwrap_or(false),
            limit.unwrap_or(50),
        )
        .map_err(|e| format!("Failed to collect timeline: {e}"))?;

        let total_count = events.len();

        let ui_events: Vec<UiTimelineEvent> = events
            .into_iter()
            .map(|e| UiTimelineEvent {
                timestamp: e.timestamp.to_rfc3339(),
                action: e.action,
                file_path: e.file_path,
                model: e.model,
                agent_tool: e.agent_tool,
                agent_version: e.agent_version,
                change_size: e.change_size,
            })
            .collect();

        Ok(ListTimelineResult {
            events: ui_events,
            total_count,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn list_transcripts(
    project_dir: Option<String>,
    limit: Option<usize>,
) -> Result<ListTranscriptsResult, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = project_dir
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
            .map(PathBuf::from);
        let trace_dir = resolve_trace_dir_for_project(project_dir);

        let transcripts = ai_blame::transcript::parse_transcripts_from_directory(&trace_dir)
            .map_err(|e| format!("Failed to parse transcripts: {e}"))?;

        let total_count = transcripts.len();
        let limit = limit.unwrap_or(50);
        let display_transcripts: Vec<_> = transcripts.into_iter().take(limit).collect();

        let summaries: Vec<UiTranscriptSummary> = display_transcripts
            .iter()
            .map(|t| {
                let summary = t.summary();
                UiTranscriptSummary {
                    session_id: summary.session_id,
                    agent_tool: summary.agent_tool,
                    slug: summary.slug,
                    start_time: summary.start_time.to_rfc3339(),
                    end_time: summary.end_time.map(|t| t.to_rfc3339()),
                    message_count: summary.message_count,
                    files_touched: summary.files_touched,
                    primary_model: summary.primary_model,
                    source_file: summary.source_file,
                }
            })
            .collect();

        Ok(ListTranscriptsResult {
            transcripts: summaries,
            total_count,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// Helper function to search for transcripts by session ID
/// Extracted to avoid code duplication between CLI and Tauri backend
fn find_transcript_by_session(
    session_or_path: &str,
    project_dir: Option<String>,
) -> Result<ai_blame::transcript::Transcript, String> {
    let project_dir = project_dir
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
        .map(PathBuf::from);
    let trace_dir = resolve_trace_dir_for_project(project_dir);

    let transcripts = ai_blame::transcript::parse_transcripts_from_directory(&trace_dir)
        .map_err(|e| format!("Failed to parse transcripts: {e}"))?;

    transcripts
        .into_iter()
        .find(|t| {
            t.meta.session_id.contains(session_or_path)
                || t.meta
                    .slug
                    .as_ref()
                    .map(|s| s.contains(session_or_path))
                    .unwrap_or(false)
                || t.meta
                    .source_file
                    .as_ref()
                    .map(|s| s.contains(session_or_path))
                    .unwrap_or(false)
        })
        .ok_or_else(|| format!("Transcript not found: {session_or_path}"))
}

#[tauri::command]
async fn get_transcript(
    session_or_path: String,
    project_dir: Option<String>,
) -> Result<UiTranscript, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        // First try to interpret as a direct file path (with validation)
        let transcript =
            if let Ok(safe_path) = ai_blame::utils::validate_safe_path(&session_or_path) {
                if safe_path.exists() {
                    ai_blame::transcript::parse_transcript(&safe_path)
                        .map_err(|e| format!("Failed to parse transcript: {e}"))?
                } else {
                    // Path doesn't exist, treat as session ID
                    find_transcript_by_session(&session_or_path, project_dir)?
                }
            } else {
                // Path validation failed, treat as session ID
                find_transcript_by_session(&session_or_path, project_dir)?
            };

        Ok(convert_transcript(transcript))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
async fn search_transcripts(
    project_dir: Option<String>,
    query: Option<String>,
    session_id_pattern: Option<String>,
    agent_tool: Option<String>,
    model: Option<String>,
    limit: Option<usize>,
) -> Result<SearchTranscriptsResult, String> {
    // Move CPU-bound work to a blocking thread
    tauri::async_runtime::spawn_blocking(move || {
        let project_dir = project_dir
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) })
            .map(PathBuf::from);
        let trace_dir = resolve_trace_dir_for_project(project_dir);

        let criteria = ai_blame::transcript::TranscriptSearchCriteria {
            query: query.filter(|s| !s.is_empty()),
            use_regex: false,
            case_sensitive: false,
            session_id_pattern: session_id_pattern.filter(|s| !s.is_empty()),
            agent_tool: agent_tool.filter(|s| !s.is_empty()),
            model: model.filter(|s| !s.is_empty()),
            since: None,
            until: None,
        };

        let search_result =
            ai_blame::transcript::search_transcripts(&trace_dir, &criteria, limit.unwrap_or(50))
                .map_err(|e| format!("Search failed: {e}"))?;

        let ui_matches: Vec<UiSearchMatch> = search_result
            .matching_transcripts
            .iter()
            .map(|search_match| {
                let summary = &search_match.transcript;
                let ui_summary = UiTranscriptSummary {
                    session_id: summary.session_id.clone(),
                    agent_tool: summary.agent_tool.clone(),
                    slug: summary.slug.clone(),
                    start_time: summary.start_time.to_rfc3339(),
                    end_time: summary.end_time.map(|t| t.to_rfc3339()),
                    message_count: summary.message_count,
                    files_touched: summary.files_touched,
                    primary_model: summary.primary_model.clone(),
                    source_file: summary.source_file.clone(),
                };
                let ui_snippets: Vec<UiMatchSnippet> = search_match
                    .matches
                    .iter()
                    .map(|m| UiMatchSnippet {
                        role: m.role.clone(),
                        timestamp: m.timestamp.to_rfc3339(),
                        block_type: m.block_type.clone(),
                        snippet: m.snippet.clone(),
                    })
                    .collect();
                UiSearchMatch {
                    transcript: ui_summary,
                    matches: ui_snippets,
                }
            })
            .collect();

        Ok(SearchTranscriptsResult {
            matching_transcripts: ui_matches,
            total_matches: search_result.total_matches,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            app_info,
            pick_project_dir,
            scan_traces,
            list_project_files,
            list_agent_touched_files,
            blame_file,
            list_timeline,
            list_transcripts,
            get_transcript,
            search_transcripts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
