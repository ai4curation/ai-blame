//! Claude Code transcript parser.
//!
//! Parses Claude Code trace files (.jsonl) into the unified transcript format.

use crate::transcript::{
    ContentBlock, FileOpType, Role, TokenUsage, Transcript, TranscriptMessage, TranscriptMeta,
    TranscriptParser,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parser for Claude Code trace files.
pub struct ClaudeTranscriptParser;

impl ClaudeTranscriptParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract content blocks from a Claude message content array.
    fn extract_content_blocks(content: &Value) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();

        let content_array = match content.as_array() {
            Some(arr) => arr,
            None => {
                // Single string content
                if let Some(text) = content.as_str() {
                    blocks.push(ContentBlock::Text {
                        text: text.to_string(),
                    });
                }
                return blocks;
            }
        };

        for item in content_array {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match item_type {
                "text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        blocks.push(ContentBlock::Text {
                            text: text.to_string(),
                        });
                    }
                }
                "thinking" => {
                    if let Some(thinking) = item.get("thinking").and_then(|t| t.as_str()) {
                        blocks.push(ContentBlock::Thinking {
                            thinking: thinking.to_string(),
                        });
                    }
                }
                "tool_use" => {
                    let id = item
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input = item.get("input").cloned().unwrap_or(Value::Null);

                    blocks.push(ContentBlock::ToolUse { id, name, input });
                }
                "tool_result" => {
                    let tool_use_id = item
                        .get("tool_use_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content = item
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    let is_error = item
                        .get("is_error")
                        .and_then(|e| e.as_bool())
                        .unwrap_or(false);

                    blocks.push(ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    });
                }
                _ => {}
            }
        }

        blocks
    }

    /// Extract a file operation from a toolUseResult record.
    fn extract_file_operation(tool_result: &Value) -> Option<ContentBlock> {
        let file_path = tool_result.get("filePath").and_then(|f| f.as_str())?;

        let op_type = tool_result
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let operation = match op_type {
            "create" => FileOpType::Create,
            "text" => {
                // Check if it's a read operation
                if tool_result.get("file").is_some() {
                    FileOpType::Read
                } else {
                    return None;
                }
            }
            _ => {
                // Could be an edit
                if tool_result.get("oldString").is_some() && tool_result.get("newString").is_some()
                {
                    FileOpType::Edit
                } else if tool_result.get("content").is_some() {
                    FileOpType::Create
                } else {
                    return None;
                }
            }
        };

        let content = tool_result
            .get("content")
            .or_else(|| tool_result.get("newString"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let old_content = tool_result
            .get("oldString")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Some(ContentBlock::FileOperation {
            operation,
            file_path: file_path.to_string(),
            content,
            old_content,
        })
    }

    /// Extract token usage from a Claude message.
    fn extract_usage(message: &Value) -> Option<TokenUsage> {
        let usage = message.get("usage")?;

        Some(TokenUsage {
            input_tokens: usage.get("input_tokens").and_then(|t| t.as_u64()),
            output_tokens: usage.get("output_tokens").and_then(|t| t.as_u64()),
            cache_read_tokens: usage
                .get("cache_read_input_tokens")
                .and_then(|t| t.as_u64()),
            cache_creation_tokens: usage
                .get("cache_creation_input_tokens")
                .and_then(|t| t.as_u64()),
        })
    }
}

impl Default for ClaudeTranscriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptParser for ClaudeTranscriptParser {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn can_parse(&self, path: &Path) -> Result<bool> {
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            return Ok(false);
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        // Check first few lines for Claude format markers
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
                // Claude traces have specific markers
                let has_uuid = record.get("uuid").is_some();
                let has_type = record.get("type").is_some();
                let has_message = record.get("message").is_some();
                let has_session_id = record.get("sessionId").is_some();

                // Also check for Codex format to avoid false positives
                let is_codex = record.get("event").is_some()
                    || (record.get("session_id").is_some() && !has_session_id);

                if has_uuid && (has_type || has_message) && !is_codex {
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
        let mut seen_uuids: HashSet<String> = HashSet::new();

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

            // Skip file-history-snapshot records
            if record.get("type").and_then(|t| t.as_str()) == Some("file-history-snapshot") {
                continue;
            }

            // Extract metadata from first user message if not set
            if meta.is_none() {
                if let Some(session_id) = record.get("sessionId").and_then(|s| s.as_str()) {
                    let version = record
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let cwd = record
                        .get("cwd")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string());
                    let git_branch = record
                        .get("gitBranch")
                        .and_then(|b| b.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    let slug = record
                        .get("slug")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    let timestamp = record
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    meta = Some(TranscriptMeta {
                        session_id: session_id.to_string(),
                        agent_tool: "claude-code".to_string(),
                        agent_version: version,
                        cwd,
                        git_branch,
                        slug,
                        start_time: timestamp,
                        end_time: None,
                        source_file: Some(path.to_string_lossy().to_string()),
                    });
                }
            }

            // Get UUID and skip duplicates
            let uuid = match record.get("uuid").and_then(|u| u.as_str()) {
                Some(u) => u.to_string(),
                None => continue,
            };

            if seen_uuids.contains(&uuid) {
                continue;
            }
            seen_uuids.insert(uuid.clone());

            // Parse timestamp
            let timestamp = match record.get("timestamp").and_then(|t| t.as_str()) {
                Some(ts_str) => match DateTime::parse_from_rfc3339(ts_str) {
                    Ok(dt) => dt.with_timezone(&Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            // Determine role
            let record_type = record.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let role = match record_type {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => continue,
            };

            // Get message content
            let msg = match record.get("message") {
                Some(m) => m,
                None => continue,
            };

            // Extract model for assistant messages
            let model = msg
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            // Extract content blocks
            let mut content_blocks = Vec::new();

            if let Some(msg_content) = msg.get("content") {
                content_blocks.extend(Self::extract_content_blocks(msg_content));
            }

            // For user messages with tool results, add file operation info
            if role == Role::User {
                if let Some(tool_result) = record.get("toolUseResult") {
                    if let Some(file_op) = Self::extract_file_operation(tool_result) {
                        // Track file touched
                        if let ContentBlock::FileOperation { file_path, .. } = &file_op {
                            files_touched.insert(file_path.clone());
                        }
                        content_blocks.push(file_op);
                    }
                }
            }

            // Skip empty messages
            if content_blocks.is_empty() {
                continue;
            }

            // Extract usage
            let usage = Self::extract_usage(msg);

            messages.push(TranscriptMessage {
                id: uuid,
                role,
                timestamp,
                content: content_blocks,
                model,
                usage,
            });
        }

        // Create transcript with default meta if none found
        let meta = meta.unwrap_or_else(|| {
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            TranscriptMeta {
                session_id: filename.to_string(),
                agent_tool: "claude-code".to_string(),
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
    fn test_extract_content_blocks() {
        let content = serde_json::json!([
            {"type": "text", "text": "Hello"},
            {"type": "thinking", "thinking": "Let me think..."},
            {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "/test.rs"}}
        ]);

        let blocks = ClaudeTranscriptParser::extract_content_blocks(&content);
        assert_eq!(blocks.len(), 3);

        assert!(matches!(&blocks[0], ContentBlock::Text { text } if text == "Hello"));
        assert!(
            matches!(&blocks[1], ContentBlock::Thinking { thinking } if thinking == "Let me think...")
        );
        assert!(matches!(&blocks[2], ContentBlock::ToolUse { name, .. } if name == "Read"));
    }

    #[test]
    fn test_extract_file_operation() {
        let create_result = serde_json::json!({
            "type": "create",
            "filePath": "/test/file.rs",
            "content": "fn main() {}"
        });

        let op = ClaudeTranscriptParser::extract_file_operation(&create_result);
        assert!(op.is_some());
        if let Some(ContentBlock::FileOperation {
            operation,
            file_path,
            ..
        }) = op
        {
            assert_eq!(operation, FileOpType::Create);
            assert_eq!(file_path, "/test/file.rs");
        }
    }
}
