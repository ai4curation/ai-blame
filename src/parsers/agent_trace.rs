//! Parser for [agent-trace.dev](https://agent-trace.dev/) v1 sidecar records.
//!
//! Unlike the other parsers in this directory, this parser does not target a
//! single agent's native log format. Instead it reads the open, vendor-neutral
//! agent-trace.dev v1 JSON spec, which any producer can emit. That makes
//! ai-blame instantly compatible with every agent that has either:
//!
//!   * a first-party emitter for the spec (today: experimental in some tools),
//!     or
//!   * a third-party exporter that converts the agent's native logs to the
//!     spec.
//!
//! ## File layout
//!
//! Records live as individual `*.json` files under an `.agent-trace/`
//! directory. Each record corresponds to one `(repo, revision)` snapshot and
//! contains nested `files[].conversations[].ranges[]` with attribution
//! metadata. See the [v1 schema](https://agent-trace.dev/schemas/v1/trace-record.json).
//!
//! ## Mapping to [`EditRecord`]
//!
//! The spec is attribution-oriented (line ranges + `content_hash`), not
//! patch-oriented. agent-trace records do not carry the original
//! `old_string` / `new_string` text, so the [`crate::blame`] reverse-apply
//! algorithm cannot recover line-level blame from these records on its own.
//!
//! Each spec range becomes one [`EditRecord`] with:
//!
//!   * `file_path` — the path field of the enclosing file entry (already
//!     repo-relative per the spec).
//!   * `timestamp` — the trace's top-level timestamp (the spec is per-revision,
//!     not per-edit).
//!   * `model` — the contributor `model_id` (e.g. `anthropic/claude-opus-4-5`),
//!     falling back to `unknown`.
//!   * `session_id` — derived from the conversation's `url` or its
//!     `related[type=session]` entry.
//!   * `agent_tool` / `agent_version` — the record's top-level `tool` block.
//!   * `is_create` / `change_size` — best-effort: ranges that start at line 1
//!     are treated as "create-shaped" with `change_size = end_line` (number of
//!     lines covered). Other ranges are reported as edits with `change_size`
//!     equal to the line span.
//!   * `old_string` / `new_string` / `structured_patch` / `create_content` —
//!     `None`. agent-trace records do not contain this raw text.
//!
//! As a result, the `stats`, `timeline`, `report`, and `transcript` commands
//! work fully against agent-trace sidecars; `blame` and `annotate` will mark
//! the touched lines with metadata when paired with a producer that emits one
//! range per edit, but cannot perform the patch-walk reconstruction that
//! Claude / Codex parsers do.

use crate::models::EditRecord;
use crate::parsers::{ParserInfo, TraceParser};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Parser for agent-trace.dev v1 JSON sidecar records.
pub struct AgentTraceParser;

impl AgentTraceParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentTraceParser {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Spec types (subset of agent-trace.dev v1 we consume).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TraceRecord {
    #[allow(dead_code)] // captured for forward compat / debugging
    version: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    tool: Option<Tool>,
    #[serde(default)]
    files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize)]
struct Tool {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileEntry {
    path: String,
    #[serde(default)]
    conversations: Vec<Conversation>,
}

#[derive(Debug, Deserialize)]
struct Conversation {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    contributor: Option<Contributor>,
    #[serde(default)]
    ranges: Vec<Range>,
    #[serde(default)]
    related: Vec<Related>,
}

#[derive(Debug, Deserialize, Clone)]
struct Contributor {
    #[serde(default, rename = "type")]
    _ty: Option<String>,
    #[serde(default)]
    model_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Range {
    start_line: usize,
    end_line: usize,
    #[serde(default)]
    contributor: Option<Contributor>,
    // content_hash is captured by the spec but ai-blame's blame engine
    // does not yet use it; future work could match on this for
    // position-independent attribution per spec §6.3.
    #[serde(default)]
    #[allow(dead_code)]
    content_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Related {
    #[serde(rename = "type")]
    ty: String,
    url: String,
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Pull a useful session identifier out of a conversation. Prefer the
/// conversation `url` (e.g. `https://ampcode.com/threads/T-abc...`) since
/// that is the most stable cross-tool handle. Fall back to the first
/// `related[type=session]` entry, then to `unknown`.
fn session_id_for(conv: &Conversation) -> String {
    if let Some(url) = conv.url.as_ref().filter(|s| !s.is_empty()) {
        return url.clone();
    }
    for r in &conv.related {
        if r.ty == "session" {
            return r.url.clone();
        }
    }
    "unknown".to_string()
}

/// Resolve the model ID for a range, preferring the per-range contributor
/// override if present (spec §6.1, $defs/range.contributor).
fn model_for_range<'a>(range: &'a Range, conv: &'a Conversation) -> &'a str {
    if let Some(c) = range.contributor.as_ref() {
        if let Some(m) = c.model_id.as_deref() {
            if !m.is_empty() {
                return m;
            }
        }
    }
    if let Some(c) = conv.contributor.as_ref() {
        if let Some(m) = c.model_id.as_deref() {
            if !m.is_empty() {
                return m;
            }
        }
    }
    "unknown"
}

/// Recursively collect `*.json` files (the spec's sidecar extension) from a
/// directory tree. Mirrors the symlink-cycle protection in
/// [`crate::extractor::collect_jsonl_files`].
fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    use std::collections::HashSet;
    fn inner(
        dir: &Path,
        out: &mut Vec<PathBuf>,
        visited: &mut HashSet<PathBuf>,
    ) -> std::io::Result<()> {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return Ok(()),
        };
        if !visited.insert(canonical) {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let _ = inner(&path, out, visited);
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut visited = HashSet::new();
    inner(dir, out, &mut visited)
}

// ---------------------------------------------------------------------------
// TraceParser impl.
// ---------------------------------------------------------------------------

impl TraceParser for AgentTraceParser {
    fn info(&self) -> ParserInfo {
        ParserInfo {
            name: "agent-trace.dev",
            description: "agent-trace.dev v1 sidecar records (.agent-trace/*.json)",
            file_extensions: vec!["json"],
        }
    }

    fn can_parse(&self, path: &Path) -> Result<Option<bool>> {
        // Cheap structural check: must be `.json`, must be a JSON object that
        // has the spec's required `version`, `files`, and `timestamp` fields.
        // We don't peek at content for `.jsonl` files — those belong to other
        // parsers.
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            return Ok(Some(false));
        }
        let f = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Ok(Some(false)),
        };
        // Cap how much we read so a giant unrelated JSON file doesn't slow
        // down detection. Most spec records are well under 64 KiB.
        let mut reader = BufReader::new(f).take(64 * 1024);
        let mut buf = String::new();
        if reader.read_to_string(&mut buf).is_err() {
            return Ok(Some(false));
        }
        // Quick string check first to reject most non-spec files without a
        // full JSON parse.
        if !buf.contains("\"version\"") || !buf.contains("\"files\"") {
            return Ok(Some(false));
        }
        match serde_json::from_str::<TraceRecord>(&buf) {
            Ok(r) => Ok(Some(r.timestamp.is_some() && !r.files.is_empty())),
            Err(_) => Ok(Some(false)),
        }
    }

    fn parse_file(&self, path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
        let bytes = std::fs::read(path)?;
        let record: TraceRecord = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(e) => {
                anyhow::bail!("not a valid agent-trace.dev v1 record: {}", e);
            }
        };

        let timestamp = record.timestamp.unwrap_or_else(Utc::now);
        let (agent_tool, agent_version) = match record.tool {
            Some(t) => (
                t.name.unwrap_or_else(|| "agent-trace".to_string()),
                t.version,
            ),
            None => ("agent-trace".to_string(), None),
        };

        let mut edits = Vec::new();
        for file_entry in record.files {
            // Apply file pattern filter (substring match, matching the
            // semantics other parsers use).
            if !file_pattern.is_empty() && !file_entry.path.contains(file_pattern) {
                continue;
            }

            for conv in file_entry.conversations {
                let session_id = session_id_for(&conv);

                for range in &conv.ranges {
                    let model = model_for_range(range, &conv).to_string();
                    let line_span = range
                        .end_line
                        .saturating_sub(range.start_line.saturating_sub(1));

                    // Heuristic: a range starting at line 1 plausibly
                    // represents a file create or whole-file write. The spec
                    // does not distinguish create vs edit — the producer's
                    // intent is encoded in the range geometry.
                    let is_create = range.start_line == 1;

                    edits.push(EditRecord {
                        file_path: file_entry.path.clone(),
                        timestamp,
                        model,
                        session_id: session_id.clone(),
                        is_create,
                        change_size: line_span,
                        agent_tool: agent_tool.clone(),
                        agent_version: agent_version.clone(),
                        // The spec omits raw text. blame/annotate that need
                        // patch-walk reconstruction will degrade gracefully.
                        old_string: None,
                        new_string: None,
                        structured_patch: None,
                        create_content: None,
                    });
                }
            }
        }

        Ok(edits)
    }

    /// Override the default `.jsonl`-only walker — the spec uses `.json`.
    fn collect_trace_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        collect_json_files(dir, &mut files)?;
        // Don't tag every random `.json` file; filter to ones we can actually
        // parse so cache/staleness tracking stays clean.
        self.filter_parseable_files(files)
    }
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const SAMPLE_RECORD: &str = r#"{
        "version": "1.0",
        "id": "049db150-b793-4c3c-9a47-541786cc4277",
        "timestamp": "2026-04-13T14:12:21.699Z",
        "vcs": {"type": "git", "revision": "f62ac99c"},
        "tool": {"name": "amp-cli", "version": "0.1.85"},
        "files": [
            {
                "path": "src/main.rs",
                "conversations": [
                    {
                        "url": "https://ampcode.com/threads/T-019d872e-ff8e-760c-8686-6850d686a3f5",
                        "contributor": {"type": "ai", "model_id": "anthropic/claude-opus-4-5"},
                        "ranges": [
                            {"start_line": 1, "end_line": 42, "content_hash": "abc"},
                            {"start_line": 100, "end_line": 105, "content_hash": "def"}
                        ],
                        "related": [
                            {"type": "session", "url": "urn:sq-agents:session:amp:T-019d872e"}
                        ]
                    }
                ]
            }
        ]
    }"#;

    fn write_sample(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("sample.json");
        let mut f = File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        path
    }

    #[test]
    fn parses_spec_record_into_edits() {
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), SAMPLE_RECORD);

        let parser = AgentTraceParser::new();
        let edits = parser.parse_file(&path, "").unwrap();

        assert_eq!(edits.len(), 2, "one EditRecord per range");

        let first = &edits[0];
        assert_eq!(first.file_path, "src/main.rs");
        assert_eq!(first.model, "anthropic/claude-opus-4-5");
        assert_eq!(first.agent_tool, "amp-cli");
        assert_eq!(first.agent_version.as_deref(), Some("0.1.85"));
        assert_eq!(
            first.session_id,
            "https://ampcode.com/threads/T-019d872e-ff8e-760c-8686-6850d686a3f5"
        );
        assert!(
            first.is_create,
            "range starting at line 1 is treated as create-shaped"
        );
        assert_eq!(first.change_size, 42);

        let second = &edits[1];
        assert!(!second.is_create, "interior range is an edit");
        assert_eq!(second.change_size, 6);
    }

    #[test]
    fn applies_file_pattern_filter() {
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), SAMPLE_RECORD);

        let parser = AgentTraceParser::new();
        let edits = parser.parse_file(&path, "no-match").unwrap();
        assert!(edits.is_empty());
    }

    #[test]
    fn falls_back_to_related_session_when_url_missing() {
        let body = r#"{
            "version": "1.0",
            "id": "x",
            "timestamp": "2026-04-13T14:12:21Z",
            "files": [{
                "path": "a.rs",
                "conversations": [{
                    "ranges": [{"start_line": 5, "end_line": 5}],
                    "related": [{"type": "session", "url": "urn:agent:session:foo"}]
                }]
            }]
        }"#;
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), body);
        let edits = AgentTraceParser::new().parse_file(&path, "").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].session_id, "urn:agent:session:foo");
        assert_eq!(edits[0].model, "unknown");
        assert_eq!(edits[0].agent_tool, "agent-trace");
        assert!(edits[0].agent_version.is_none());
    }

    #[test]
    fn can_parse_rejects_non_spec_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nope.json");
        std::fs::write(&path, r#"{"foo": "bar"}"#).unwrap();
        let parser = AgentTraceParser::new();
        assert_eq!(parser.can_parse(&path).unwrap(), Some(false));
    }

    #[test]
    fn can_parse_accepts_spec_record() {
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), SAMPLE_RECORD);
        let parser = AgentTraceParser::new();
        assert_eq!(parser.can_parse(&path).unwrap(), Some(true));
    }

    #[test]
    fn can_parse_rejects_jsonl_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("not-mine.jsonl");
        std::fs::write(&path, SAMPLE_RECORD).unwrap();
        let parser = AgentTraceParser::new();
        assert_eq!(
            parser.can_parse(&path).unwrap(),
            Some(false),
            "agent-trace parser must not steal .jsonl files from other parsers"
        );
    }

    #[test]
    fn collect_trace_files_finds_json_only() {
        let dir = tempdir().unwrap();
        let trace_dir = dir.path().join(".agent-trace");
        std::fs::create_dir(&trace_dir).unwrap();
        write_sample(&trace_dir, SAMPLE_RECORD);
        std::fs::write(trace_dir.join("ignored.jsonl"), "{}").unwrap();
        std::fs::write(trace_dir.join("also-ignored.txt"), "x").unwrap();

        let parser = AgentTraceParser::new();
        let files = parser.collect_trace_files(&trace_dir).unwrap();
        assert_eq!(files.len(), 1, "only the spec sidecar should be picked up");
    }
}
