use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CurationAction {
    Created,
    Edited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationEvent {
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<CurationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EditRecord {
    pub file_path: String,
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub session_id: String,
    pub is_create: bool,
    pub change_size: usize,
    pub agent_tool: String,
    pub agent_version: Option<String>,
    /// For edit operations: the exact string that was replaced.
    pub old_string: Option<String>,
    /// For edit operations: the replacement string that was inserted.
    pub new_string: Option<String>,
    /// Optional structured patch information from the trace (often unified-diff-like).
    pub structured_patch: Option<String>,
    /// For create operations: the file content at creation time (if present in trace).
    pub create_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub action: String, // "CREATED" or "EDITED"
    pub file_path: String,
    pub model: String,
    pub agent_tool: String,
    pub agent_version: Option<String>,
    pub change_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    pub initial_and_recent_only: bool,
    pub min_change_size: usize,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub file_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistory {
    pub file_path: String,
    #[serde(default)]
    pub events: Vec<CurationEvent>,
}

impl FileHistory {
    pub fn first_edit(&self) -> Option<DateTime<Utc>> {
        self.events.iter().map(|e| e.timestamp).min()
    }

    pub fn last_edit(&self) -> Option<DateTime<Utc>> {
        self.events.iter().map(|e| e.timestamp).max()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputPolicy {
    Append,
    Sidecar,
    Comment,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommentSyntax {
    Hash,
    Slash,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRule {
    #[serde(default = "default_pattern")]
    pub pattern: String,
    #[serde(default = "default_policy")]
    pub policy: OutputPolicy,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_syntax: Option<CommentSyntax>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecar_pattern: Option<String>,
}

fn default_pattern() -> String {
    "*".to_string()
}

fn default_policy() -> OutputPolicy {
    OutputPolicy::Append
}

fn default_format() -> String {
    "yaml".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<FileRule>,
    #[serde(default)]
    pub rules: Vec<FileRule>,
}

impl OutputConfig {
    pub fn get_rule_for_file(&self, path: &str) -> Option<FileRule> {
        let filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        for rule in &self.rules {
            // Use full path for patterns with path separators or **
            if rule.pattern.contains('/') || rule.pattern.contains("**") {
                if glob::Pattern::new(&rule.pattern)
                    .ok()
                    .map(|p| p.matches(path))
                    .unwrap_or(false)
                {
                    return Some(rule.clone());
                }
            } else {
                // Match against filename only
                if glob::Pattern::new(&rule.pattern)
                    .ok()
                    .map(|p| p.matches(filename))
                    .unwrap_or(false)
                {
                    return Some(rule.clone());
                }
            }
        }

        self.defaults.clone()
    }
}

pub type EditsByFile = HashMap<String, Vec<EditRecord>>;
pub type HistoriesByFile = HashMap<String, FileHistory>;
