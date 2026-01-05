//! Codex CLI transcript parser.
//!
//! Parses Codex CLI trace files (.jsonl) into the unified transcript format.

use crate::transcript::{
    ContentBlock, Role, TokenUsage, Transcript, TranscriptMessage, TranscriptMeta, TranscriptParser,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parser for Codex CLI trace files.
pub struct CodexTranscriptParser;

impl CodexTranscriptParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract content blocks from Codex message content.
    fn extract_content_from_message(payload: &Value) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();

        let content = match payload.get("content").and_then(|c| c.as_array()) {
            Some(arr) => arr,
            None => return blocks,
        };

        for item in content {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match item_type {
                "input_text" | "output_text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        blocks.push(ContentBlock::Text {
                            text: text.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        blocks
    }

    /// Extract token usage from a token_count event.
    fn extract_usage(info: &Value) -> Option<TokenUsage> {
        let usage = info.get("total_token_usage")?;

        Some(TokenUsage {
            input_tokens: usage.get("input_tokens").and_then(|t| t.as_u64()),
            output_tokens: usage.get("output_tokens").and_then(|t| t.as_u64()),
            cache_read_tokens: usage.get("cached_input_tokens").and_then(|t| t.as_u64()),
            cache_creation_tokens: None,
        })
    }
}

impl Default for CodexTranscriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptParser for CodexTranscriptParser {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn can_parse(&self, path: &Path) -> Result<bool> {
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            return Ok(false);
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        // Check first few lines for Codex format markers
        for _ in 0..10 {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }

            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            if let Ok(record) = serde_json::from_str::<Value>(line) {
                // Codex format markers
                let has_type = record.get("type").is_some();
                let has_payload = record.get("payload").is_some();
                let has_timestamp = record.get("timestamp").is_some();

                // Check for Codex-specific types
                let record_type = record.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let is_codex_type = matches!(
                    record_type,
                    "session_meta"
                        | "response_item"
                        | "event_msg"
                        | "turn_context"
                        | "ghost_snapshot"
                );

                // Also check for old-style Codex format
                let has_event = record.get("event").is_some();
                let has_session_id =
                    record.get("session_id").is_some() && record.get("sessionId").is_none();

                if (has_type && has_payload && has_timestamp && is_codex_type)
                    || has_event
                    || has_session_id
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn parse(&self, path: &Path) -> Result<Transcript> {
        let file =
            File::open(path).with_context(|| format!("Failed to open trace file: {:?}", path))?;
        let reader = BufReader::new(file);

        let mut meta: Option<TranscriptMeta> = None;
        let mut messages: Vec<TranscriptMessage> = Vec::new();
        let mut files_touched: HashSet<String> = HashSet::new();
        let mut message_id_counter = 0;
        let mut pending_tool_calls: Vec<(String, ContentBlock)> = Vec::new();
        let mut last_usage: Option<TokenUsage> = None;

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

            let record_type = record.get("type").and_then(|t| t.as_str()).unwrap_or("");

            // Parse timestamp
            let timestamp = record
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            match record_type {
                "session_meta" => {
                    if let Some(payload) = record.get("payload") {
                        let session_id = payload
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let version = payload
                            .get("cli_version")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let cwd = payload
                            .get("cwd")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());
                        let originator = payload
                            .get("originator")
                            .and_then(|o| o.as_str())
                            .unwrap_or("codex-cli");

                        meta = Some(TranscriptMeta {
                            session_id,
                            agent_tool: originator.to_string(),
                            agent_version: version,
                            cwd,
                            git_branch: None,
                            slug: None,
                            start_time: timestamp.unwrap_or_else(Utc::now),
                            end_time: None,
                            source_file: Some(path.to_string_lossy().to_string()),
                        });
                    }
                }
                "response_item" => {
                    if let Some(payload) = record.get("payload") {
                        let payload_type =
                            payload.get("type").and_then(|t| t.as_str()).unwrap_or("");

                        match payload_type {
                            "message" => {
                                let role_str =
                                    payload.get("role").and_then(|r| r.as_str()).unwrap_or("");
                                let role = match role_str {
                                    "user" => Role::User,
                                    "assistant" => Role::Assistant,
                                    "system" => Role::System,
                                    _ => continue,
                                };

                                let content_blocks = Self::extract_content_from_message(payload);

                                if !content_blocks.is_empty() {
                                    message_id_counter += 1;
                                    messages.push(TranscriptMessage {
                                        id: format!("msg_{}", message_id_counter),
                                        role,
                                        timestamp: timestamp.unwrap_or_else(Utc::now),
                                        content: content_blocks,
                                        model: None,
                                        usage: last_usage.take(),
                                    });
                                }
                            }
                            "function_call" => {
                                let name = payload
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let call_id = payload
                                    .get("call_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let arguments = payload
                                    .get("arguments")
                                    .and_then(|a| a.as_str())
                                    .and_then(|s| serde_json::from_str(s).ok())
                                    .unwrap_or(Value::Null);

                                let tool_use = ContentBlock::ToolUse {
                                    id: call_id.clone(),
                                    name,
                                    input: arguments,
                                };
                                pending_tool_calls.push((call_id, tool_use));
                            }
                            "function_call_output" => {
                                let call_id = payload
                                    .get("call_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let output = payload
                                    .get("output")
                                    .and_then(|o| o.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Find the matching tool call and create a combined assistant message
                                if let Some(idx) =
                                    pending_tool_calls.iter().position(|(id, _)| id == &call_id)
                                {
                                    let (_, tool_use) = pending_tool_calls.remove(idx);

                                    // Create assistant message with tool use
                                    message_id_counter += 1;
                                    messages.push(TranscriptMessage {
                                        id: format!("msg_{}", message_id_counter),
                                        role: Role::Assistant,
                                        timestamp: timestamp.unwrap_or_else(Utc::now),
                                        content: vec![tool_use],
                                        model: None,
                                        usage: None,
                                    });

                                    // Create tool result
                                    message_id_counter += 1;
                                    messages.push(TranscriptMessage {
                                        id: format!("msg_{}", message_id_counter),
                                        role: Role::User,
                                        timestamp: timestamp.unwrap_or_else(Utc::now),
                                        content: vec![ContentBlock::ToolResult {
                                            tool_use_id: call_id,
                                            content: output,
                                            is_error: false,
                                        }],
                                        model: None,
                                        usage: None,
                                    });
                                }
                            }
                            "reasoning" => {
                                let summary = payload
                                    .get("summary")
                                    .and_then(|s| s.as_array())
                                    .and_then(|arr| arr.first())
                                    .and_then(|s| s.get("text"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("");

                                if !summary.is_empty() {
                                    message_id_counter += 1;
                                    messages.push(TranscriptMessage {
                                        id: format!("msg_{}", message_id_counter),
                                        role: Role::Assistant,
                                        timestamp: timestamp.unwrap_or_else(Utc::now),
                                        content: vec![ContentBlock::Thinking {
                                            thinking: summary.to_string(),
                                        }],
                                        model: None,
                                        usage: None,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "event_msg" => {
                    if let Some(payload) = record.get("payload") {
                        let event_type = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");

                        if event_type == "token_count" {
                            if let Some(info) = payload.get("info") {
                                last_usage = Self::extract_usage(info);
                            }
                        }
                    }
                }
                // Handle old-style Codex format (from test files)
                "" => {
                    let event = record.get("event").and_then(|e| e.as_str()).unwrap_or("");

                    match event {
                        "create" => {
                            let file_path = record
                                .get("file")
                                .and_then(|f| f.as_str())
                                .unwrap_or("")
                                .to_string();
                            let content = record
                                .get("content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());

                            if !file_path.is_empty() {
                                files_touched.insert(file_path.clone());
                                message_id_counter += 1;
                                messages.push(TranscriptMessage {
                                    id: format!("msg_{}", message_id_counter),
                                    role: Role::Assistant,
                                    timestamp: timestamp.unwrap_or_else(Utc::now),
                                    content: vec![ContentBlock::FileOperation {
                                        operation: super::FileOpType::Create,
                                        file_path,
                                        content,
                                        old_content: None,
                                    }],
                                    model: record
                                        .get("model")
                                        .and_then(|m| m.as_str())
                                        .map(|s| s.to_string()),
                                    usage: None,
                                });
                            }
                        }
                        "edit" => {
                            let file_path = record
                                .get("file_path")
                                .or_else(|| record.get("file"))
                                .and_then(|f| f.as_str())
                                .unwrap_or("")
                                .to_string();
                            let old_content = record
                                .get("old_content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());
                            let new_content = record
                                .get("new_content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());

                            if !file_path.is_empty() {
                                files_touched.insert(file_path.clone());
                                message_id_counter += 1;
                                messages.push(TranscriptMessage {
                                    id: format!("msg_{}", message_id_counter),
                                    role: Role::Assistant,
                                    timestamp: timestamp.unwrap_or_else(Utc::now),
                                    content: vec![ContentBlock::FileOperation {
                                        operation: super::FileOpType::Edit,
                                        file_path,
                                        content: new_content,
                                        old_content,
                                    }],
                                    model: record
                                        .get("model")
                                        .and_then(|m| m.as_str())
                                        .map(|s| s.to_string()),
                                    usage: None,
                                });
                            }
                        }
                        _ => {}
                    }

                    // Extract session metadata from old format
                    if meta.is_none() {
                        if let Some(session_id) = record.get("session_id").and_then(|s| s.as_str())
                        {
                            meta = Some(TranscriptMeta {
                                session_id: session_id.to_string(),
                                agent_tool: "codex-cli".to_string(),
                                agent_version: None,
                                cwd: None,
                                git_branch: None,
                                slug: None,
                                start_time: timestamp.unwrap_or_else(Utc::now),
                                end_time: None,
                                source_file: Some(path.to_string_lossy().to_string()),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Create transcript with default meta if none found
        let meta = meta.unwrap_or_else(|| {
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            TranscriptMeta {
                session_id: filename.to_string(),
                agent_tool: "codex-cli".to_string(),
                agent_version: None,
                cwd: None,
                git_branch: None,
                slug: None,
                start_time: Utc::now(),
                end_time: None,
                source_file: Some(path.to_string_lossy().to_string()),
            }
        });

        let mut transcript = Transcript::new(meta);
        for msg in messages {
            transcript.add_message(msg);
        }
        transcript.stats.files_touched = files_touched.len();
        transcript.sort_messages();

        Ok(transcript)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_parser_creation() {
        let parser = CodexTranscriptParser::new();
        assert_eq!(parser.name(), "codex");
    }
}
