//! Unified transcript data model and parsers for AI agent conversations.
//!
//! This module provides a unified representation of AI agent conversations
//! that works across different agent formats (Claude Code, Codex CLI, etc.).
//! The model is designed for both CLI display and UI rendering.

pub mod claude;
pub mod codex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents the role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// The type of content in a message block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    Text { text: String },

    /// Thinking/reasoning content (chain-of-thought).
    Thinking { thinking: String },

    /// A tool use request from the assistant.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// The result of a tool execution.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    /// Code block with optional language.
    Code {
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },

    /// File operation result (create, edit, read).
    FileOperation {
        operation: FileOpType,
        file_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        old_content: Option<String>,
    },

    /// Command execution (bash, shell).
    Command {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
}

/// Type of file operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileOpType {
    Create,
    Edit,
    Read,
    Delete,
}

impl std::fmt::Display for FileOpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOpType::Create => write!(f, "create"),
            FileOpType::Edit => write!(f, "edit"),
            FileOpType::Read => write!(f, "read"),
            FileOpType::Delete => write!(f, "delete"),
        }
    }
}

/// A single message in a transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMessage {
    /// Unique identifier for this message.
    pub id: String,

    /// Role of the message sender.
    pub role: Role,

    /// Timestamp when the message was created.
    pub timestamp: DateTime<Utc>,

    /// Content blocks in this message.
    pub content: Vec<ContentBlock>,

    /// The model that generated this message (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Cost/token information for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// Token usage information for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u64>,
}

/// Metadata about a transcript session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMeta {
    /// Session ID (from trace file or generated).
    pub session_id: String,

    /// Agent tool that generated this transcript.
    pub agent_tool: String,

    /// Agent version (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,

    /// Working directory during the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Git branch during the session (if in a git repo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Session slug/name (Claude Code feature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,

    /// Start time of the session.
    pub start_time: DateTime<Utc>,

    /// End time of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Source file path for this transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
}

/// A complete transcript representing a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Metadata about the session.
    pub meta: TranscriptMeta,

    /// All messages in the conversation, ordered by timestamp.
    pub messages: Vec<TranscriptMessage>,

    /// Summary statistics.
    pub stats: TranscriptStats,
}

/// Statistics about a transcript.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptStats {
    /// Total number of messages.
    pub message_count: usize,

    /// Number of user messages.
    pub user_message_count: usize,

    /// Number of assistant messages.
    pub assistant_message_count: usize,

    /// Number of tool uses.
    pub tool_use_count: usize,

    /// Number of files touched (created, edited).
    pub files_touched: usize,

    /// Total input tokens (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_tokens: Option<u64>,

    /// Total output tokens (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<u64>,
}

/// A summary of a transcript for listing purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSummary {
    /// Session ID.
    pub session_id: String,

    /// Agent tool.
    pub agent_tool: String,

    /// Session slug (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,

    /// Start time.
    pub start_time: DateTime<Utc>,

    /// End time (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Number of messages.
    pub message_count: usize,

    /// Number of files touched.
    pub files_touched: usize,

    /// Primary model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_model: Option<String>,

    /// All unique models used in this transcript.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub all_models: Vec<String>,

    /// Last message preview (truncated for display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,

    /// Source file path.
    pub source_file: String,
}

impl Transcript {
    /// Create a new empty transcript with the given metadata.
    pub fn new(meta: TranscriptMeta) -> Self {
        Self {
            meta,
            messages: Vec::new(),
            stats: TranscriptStats::default(),
        }
    }

    /// Add a message to the transcript and update statistics.
    pub fn add_message(&mut self, message: TranscriptMessage) {
        // Update stats
        self.stats.message_count += 1;
        match message.role {
            Role::User => self.stats.user_message_count += 1,
            Role::Assistant => self.stats.assistant_message_count += 1,
            Role::System => {}
        }

        // Count tool uses
        for content in &message.content {
            if matches!(content, ContentBlock::ToolUse { .. }) {
                self.stats.tool_use_count += 1;
            }
        }

        // Update token counts
        if let Some(usage) = &message.usage {
            if let Some(input) = usage.input_tokens {
                *self.stats.total_input_tokens.get_or_insert(0) += input;
            }
            if let Some(output) = usage.output_tokens {
                *self.stats.total_output_tokens.get_or_insert(0) += output;
            }
        }

        // Update end time
        if self
            .meta
            .end_time
            .map(|t| message.timestamp > t)
            .unwrap_or(true)
        {
            self.meta.end_time = Some(message.timestamp);
        }

        self.messages.push(message);
    }

    /// Sort messages by timestamp.
    pub fn sort_messages(&mut self) {
        self.messages.sort_by_key(|m| m.timestamp);
    }

    /// Get a summary of this transcript.
    pub fn summary(&self) -> TranscriptSummary {
        let primary_model = self
            .messages
            .iter()
            .filter_map(|m| m.model.as_ref())
            .next()
            .cloned();

        // Collect all unique models
        let mut all_models_set = std::collections::HashSet::new();
        for msg in &self.messages {
            if let Some(model) = &msg.model {
                all_models_set.insert(model.clone());
            }
        }
        let mut all_models: Vec<String> = all_models_set.into_iter().collect();
        all_models.sort();

        // Get last message preview (first text block from last assistant message)
        let last_message_preview = self
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .and_then(|m| {
                m.content.iter().find_map(|block| match block {
                    ContentBlock::Text { text } => Some(text),
                    _ => None,
                })
            })
            .map(|text| {
                if text.len() > 80 {
                    format!("{}…", crate::utils::safe_truncate(text, 77))
                } else {
                    text.clone()
                }
            });

        TranscriptSummary {
            session_id: self.meta.session_id.clone(),
            agent_tool: self.meta.agent_tool.clone(),
            slug: self.meta.slug.clone(),
            start_time: self.meta.start_time,
            end_time: self.meta.end_time,
            message_count: self.stats.message_count,
            files_touched: self.stats.files_touched,
            primary_model,
            all_models,
            last_message_preview,
            source_file: self.meta.source_file.clone().unwrap_or_default(),
        }
    }

    /// Get messages in a given time range.
    pub fn messages_in_range(
        &self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Vec<&TranscriptMessage> {
        self.messages
            .iter()
            .filter(|m| {
                let after_start = start.map(|s| m.timestamp >= s).unwrap_or(true);
                let before_end = end.map(|e| m.timestamp <= e).unwrap_or(true);
                after_start && before_end
            })
            .collect()
    }
}

/// Trait for parsing transcripts from different formats.
pub trait TranscriptParser: Send + Sync {
    /// Get information about this parser.
    fn name(&self) -> &'static str;

    /// Check if this parser can handle the given file.
    fn can_parse(&self, path: &Path) -> anyhow::Result<bool>;

    /// Parse a transcript from a file.
    fn parse(&self, path: &Path) -> anyhow::Result<Transcript>;

    /// Parse all transcripts from a directory.
    fn parse_directory(&self, dir: &Path) -> anyhow::Result<Vec<Transcript>> {
        let mut transcripts = Vec::new();
        if !dir.is_dir() {
            return Ok(transcripts);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && self.can_parse(&path).unwrap_or(false) {
                match self.parse(&path) {
                    Ok(transcript) => transcripts.push(transcript),
                    Err(e) => {
                        // Log warning but continue processing other files
                        // Consider using a proper logging framework in the future
                        if std::env::var("RUST_LOG").is_ok() {
                            eprintln!("Warning: Failed to parse {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        transcripts.sort_by_key(|t| std::cmp::Reverse(t.meta.start_time));
        Ok(transcripts)
    }
}

/// Parse transcripts from a directory, automatically detecting the format.
pub fn parse_transcripts_from_directory(dir: &Path) -> anyhow::Result<Vec<Transcript>> {
    let claude_parser = claude::ClaudeTranscriptParser::new();
    let codex_parser = codex::CodexTranscriptParser::new();

    let mut all_transcripts = Vec::new();

    // Collect all .jsonl files
    fn collect_jsonl_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> anyhow::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_jsonl_files(&path, files)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect_jsonl_files(dir, &mut files)?;

    for file in files {
        // Try Claude parser first
        if claude_parser.can_parse(&file).unwrap_or(false) {
            if let Ok(transcript) = claude_parser.parse(&file) {
                all_transcripts.push(transcript);
                continue;
            }
        }

        // Try Codex parser
        if codex_parser.can_parse(&file).unwrap_or(false) {
            if let Ok(transcript) = codex_parser.parse(&file) {
                all_transcripts.push(transcript);
            }
        }
    }

    // Sort by start time, newest first
    all_transcripts.sort_by_key(|t| std::cmp::Reverse(t.meta.start_time));
    Ok(all_transcripts)
}

/// Parse a single transcript file, auto-detecting the format.
pub fn parse_transcript(path: &Path) -> anyhow::Result<Transcript> {
    let claude_parser = claude::ClaudeTranscriptParser::new();
    let codex_parser = codex::CodexTranscriptParser::new();

    if claude_parser.can_parse(path).unwrap_or(false) {
        return claude_parser.parse(path);
    }

    if codex_parser.can_parse(path).unwrap_or(false) {
        return codex_parser.parse(path);
    }

    anyhow::bail!("Unknown transcript format: {:?}", path)
}

/// Search criteria for transcript search
#[derive(Debug, Clone, Default)]
pub struct TranscriptSearchCriteria {
    /// Full-text search in message content
    pub query: Option<String>,
    /// Treat query as a regex pattern (default: substring match)
    pub use_regex: bool,
    /// Case-sensitive search (default: case-insensitive)
    pub case_sensitive: bool,
    /// Filter by session ID pattern
    pub session_id_pattern: Option<String>,
    /// Filter by agent tool name
    pub agent_tool: Option<String>,
    /// Filter by model name
    pub model: Option<String>,
    /// Filter by start time (after)
    pub since: Option<DateTime<Utc>>,
    /// Filter by start time (before)
    pub until: Option<DateTime<Utc>>,
}

/// A single match within a transcript
#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    /// The transcript summary
    pub transcript: TranscriptSummary,
    /// Matching snippets with context
    pub matches: Vec<MatchSnippet>,
}

/// A snippet showing matched content with context
#[derive(Debug, Clone, Serialize)]
pub struct MatchSnippet {
    /// Role of the message containing the match
    pub role: String,
    /// Timestamp of the message
    pub timestamp: chrono::DateTime<Utc>,
    /// Type of content block (text, code, command, etc.)
    pub block_type: String,
    /// The matching text with surrounding context
    pub snippet: String,
}

/// Result of a transcript search
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub matching_transcripts: Vec<SearchMatch>,
    pub total_matches: usize,
}

/// A matcher that supports both substring and regex matching
enum QueryMatcher {
    Substring { query: String, case_sensitive: bool },
    Regex(regex::Regex),
}

impl QueryMatcher {
    /// Create a new matcher from search criteria
    fn new(query: &str, use_regex: bool, case_sensitive: bool) -> anyhow::Result<Self> {
        if use_regex {
            let pattern = if case_sensitive {
                query.to_string()
            } else {
                format!("(?i){}", query)
            };
            let re = regex::Regex::new(&pattern)
                .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;
            Ok(QueryMatcher::Regex(re))
        } else {
            let normalized = if case_sensitive {
                query.to_string()
            } else {
                query.to_lowercase()
            };
            Ok(QueryMatcher::Substring {
                query: normalized,
                case_sensitive,
            })
        }
    }

    /// Check if text matches the query
    fn matches(&self, text: &str) -> bool {
        match self {
            QueryMatcher::Substring {
                query,
                case_sensitive,
                ..
            } => {
                if *case_sensitive {
                    text.contains(query)
                } else {
                    text.to_lowercase().contains(query)
                }
            }
            QueryMatcher::Regex(re) => re.is_match(text),
        }
    }

    /// Extract a snippet with context around the match
    fn extract_snippet(&self, text: &str, context_chars: usize) -> Option<String> {
        // Helper to find safe char boundary at or before a byte position
        fn floor_char_boundary(s: &str, mut byte_pos: usize) -> usize {
            byte_pos = byte_pos.min(s.len());
            while byte_pos > 0 && !s.is_char_boundary(byte_pos) {
                byte_pos -= 1;
            }
            byte_pos
        }

        // Helper to find safe char boundary at or after a byte position
        fn ceil_char_boundary(s: &str, mut byte_pos: usize) -> usize {
            byte_pos = byte_pos.min(s.len());
            while byte_pos < s.len() && !s.is_char_boundary(byte_pos) {
                byte_pos += 1;
            }
            byte_pos
        }

        match self {
            QueryMatcher::Substring {
                query,
                case_sensitive,
            } => {
                let search_text = if *case_sensitive {
                    text.to_string()
                } else {
                    text.to_lowercase()
                };
                let pos = search_text.find(query)?;

                // Find start and end positions with context, respecting char boundaries
                let start = floor_char_boundary(text, pos.saturating_sub(context_chars));
                let end =
                    ceil_char_boundary(text, (pos + query.len() + context_chars).min(text.len()));

                let snippet = &text[start..end];
                let prefix = if start > 0 { "..." } else { "" };
                let suffix = if end < text.len() { "..." } else { "" };

                Some(format!("{}{}{}", prefix, snippet.trim(), suffix))
            }
            QueryMatcher::Regex(re) => {
                let mat = re.find(text)?;

                // Find start and end positions with context, respecting char boundaries
                let start = floor_char_boundary(text, mat.start().saturating_sub(context_chars));
                let end = ceil_char_boundary(text, (mat.end() + context_chars).min(text.len()));

                let snippet = &text[start..end];
                let prefix = if start > 0 { "..." } else { "" };
                let suffix = if end < text.len() { "..." } else { "" };

                Some(format!("{}{}{}", prefix, snippet.trim(), suffix))
            }
        }
    }
}

/// Maximum number of snippets to collect per transcript
const MAX_SNIPPETS_PER_TRANSCRIPT: usize = 3;
/// Context characters around the match
const SNIPPET_CONTEXT_CHARS: usize = 60;

/// Extract matching snippets from a transcript
fn extract_matching_snippets(transcript: &Transcript, matcher: &QueryMatcher) -> Vec<MatchSnippet> {
    let mut snippets = Vec::new();

    for msg in &transcript.messages {
        if snippets.len() >= MAX_SNIPPETS_PER_TRANSCRIPT {
            break;
        }

        for block in &msg.content {
            if snippets.len() >= MAX_SNIPPETS_PER_TRANSCRIPT {
                break;
            }

            let (block_type, text_to_search) = match block {
                ContentBlock::Text { text } => ("text", text.clone()),
                ContentBlock::Thinking { thinking } => ("thinking", thinking.clone()),
                ContentBlock::Code { code, .. } => ("code", code.clone()),
                ContentBlock::ToolUse { name, input, .. } => {
                    // Search in both name and input
                    if matcher.matches(name) {
                        ("tool", name.clone())
                    } else {
                        ("tool", input.to_string())
                    }
                }
                ContentBlock::FileOperation { file_path, .. } => ("file", file_path.clone()),
                ContentBlock::Command {
                    command, output, ..
                } => {
                    if matcher.matches(command) {
                        ("command", command.clone())
                    } else if let Some(out) = output {
                        ("output", out.clone())
                    } else {
                        continue;
                    }
                }
                ContentBlock::ToolResult { content, .. } => ("result", content.clone()),
            };

            if let Some(snippet) = matcher.extract_snippet(&text_to_search, SNIPPET_CONTEXT_CHARS) {
                snippets.push(MatchSnippet {
                    role: msg.role.to_string(),
                    timestamp: msg.timestamp,
                    block_type: block_type.to_string(),
                    snippet,
                });
            }
        }
    }

    snippets
}

/// Search transcripts in a directory matching the given criteria
pub fn search_transcripts(
    dir: &Path,
    criteria: &TranscriptSearchCriteria,
    limit: usize,
) -> anyhow::Result<SearchResult> {
    // Build matcher if query is present
    let matcher = if let Some(ref query) = criteria.query {
        Some(QueryMatcher::new(
            query,
            criteria.use_regex,
            criteria.case_sensitive,
        )?)
    } else {
        None
    };

    // Parse all transcripts
    let mut transcripts = parse_transcripts_from_directory(dir)?;

    // Apply metadata filters first
    transcripts.retain(|t| {
        // Filter by session ID pattern
        if let Some(ref pattern) = criteria.session_id_pattern {
            if !t.meta.session_id.contains(pattern) {
                return false;
            }
        }

        // Filter by agent tool
        if let Some(ref tool) = criteria.agent_tool {
            if !t
                .meta
                .agent_tool
                .to_lowercase()
                .contains(&tool.to_lowercase())
            {
                return false;
            }
        }

        // Filter by timestamp range
        if let Some(since) = criteria.since {
            if t.meta.start_time < since {
                return false;
            }
        }
        if let Some(until) = criteria.until {
            if t.meta.start_time > until {
                return false;
            }
        }

        // Filter by model
        if let Some(ref model) = criteria.model {
            let model_lower = model.to_lowercase();
            let has_model = t.messages.iter().any(|msg| {
                msg.model
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(&model_lower))
                    .unwrap_or(false)
            });
            if !has_model {
                return false;
            }
        }

        true
    });

    // Now search for matches and collect snippets
    let mut matches: Vec<SearchMatch> = Vec::new();

    for transcript in transcripts {
        if let Some(ref matcher) = matcher {
            let snippets = extract_matching_snippets(&transcript, matcher);
            if !snippets.is_empty() {
                matches.push(SearchMatch {
                    transcript: transcript.summary(),
                    matches: snippets,
                });
            }
        } else {
            // No query, just return the transcript with no snippets
            matches.push(SearchMatch {
                transcript: transcript.summary(),
                matches: Vec::new(),
            });
        }
    }

    let total_matches = matches.len();

    // Apply limit (0 means no limit)
    let matching_transcripts = if limit == 0 {
        matches
    } else {
        matches.into_iter().take(limit).collect()
    };

    Ok(SearchResult {
        matching_transcripts,
        total_matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_stats() {
        let meta = TranscriptMeta {
            session_id: "test".to_string(),
            agent_tool: "claude-code".to_string(),
            agent_version: None,
            cwd: None,
            git_branch: None,
            slug: None,
            start_time: Utc::now(),
            end_time: None,
            source_file: None,
        };

        let mut transcript = Transcript::new(meta);

        transcript.add_message(TranscriptMessage {
            id: "1".to_string(),
            role: Role::User,
            timestamp: Utc::now(),
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            model: None,
            usage: None,
        });

        transcript.add_message(TranscriptMessage {
            id: "2".to_string(),
            role: Role::Assistant,
            timestamp: Utc::now(),
            content: vec![
                ContentBlock::Text {
                    text: "Hello!".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "t1".to_string(),
                    name: "Read".to_string(),
                    input: serde_json::json!({"file_path": "/test.rs"}),
                },
            ],
            model: Some("claude-3-sonnet".to_string()),
            usage: Some(TokenUsage {
                input_tokens: Some(100),
                output_tokens: Some(50),
                cache_read_tokens: None,
                cache_creation_tokens: None,
            }),
        });

        assert_eq!(transcript.stats.message_count, 2);
        assert_eq!(transcript.stats.user_message_count, 1);
        assert_eq!(transcript.stats.assistant_message_count, 1);
        assert_eq!(transcript.stats.tool_use_count, 1);
        assert_eq!(transcript.stats.total_input_tokens, Some(100));
        assert_eq!(transcript.stats.total_output_tokens, Some(50));
    }
}
