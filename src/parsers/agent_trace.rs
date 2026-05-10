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
    /// Spec values: `"human" | "ai" | "mixed" | "unknown"`. We surface this
    /// when `model_id` is missing so downstream output isn't a flat
    /// `unknown`.
    #[serde(default, rename = "type")]
    ty: Option<String>,
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

/// Pull a useful session identifier out of a conversation.
///
/// Order of preference:
///   1. The trailing path segment of the conversation `url` (e.g. the
///      `T-019d…` thread id from `https://ampcode.com/threads/T-019d…`).
///      That is what other parsers use for `session_id`, and short ids are
///      easier to correlate across tools than full URLs.
///   2. The trailing segment of the first `related[type=session]` URN
///      (e.g. `T-019d…` from `urn:sq-agents:session:amp:T-019d…`).
///   3. The full URL / URN if no clean trailing segment can be extracted.
///   4. `"unknown"` if neither field is present.
fn session_id_for(conv: &Conversation) -> String {
    if let Some(url) = conv.url.as_deref().filter(|s| !s.is_empty()) {
        if let Some(id) = trailing_segment(url) {
            return id;
        }
        return url.to_string();
    }
    for r in &conv.related {
        if r.ty == "session" {
            if let Some(id) = trailing_segment(&r.url) {
                return id;
            }
            return r.url.clone();
        }
    }
    "unknown".to_string()
}

/// Resolve the model ID for a range, preferring the per-range contributor
/// override if present (spec §6.1, $defs/range.contributor).
///
/// When `model_id` is missing on both, fall back to the contributor `type`
/// (e.g. `ai`, `human`, `mixed`) so reports show *something* useful instead
/// of a flat `unknown`. Only `unknown` is returned when there is genuinely
/// no contributor information at all.
fn model_for_range(range: &Range, conv: &Conversation) -> String {
    for c in [range.contributor.as_ref(), conv.contributor.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some(m) = c.model_id.as_deref() {
            if !m.is_empty() {
                return m.to_string();
            }
        }
    }
    for c in [range.contributor.as_ref(), conv.contributor.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some(t) = c.ty.as_deref() {
            if !t.is_empty() {
                // Mark this as a degraded value so downstream consumers can
                // tell "we know it was AI but not which model" apart from a
                // genuine, fully-populated `model_id` like
                // `anthropic/claude-opus-4-5`.
                return format!("{} (model unspecified)", t);
            }
        }
    }
    "unknown".to_string()
}

/// Derive a best-effort agent identifier for a conversation, given the
/// top-level record `tool.name` as a fallback.
///
/// Producers SHOULD set `tool.name` at record level, but the spec also
/// requires `tool.version` whenever `tool` is present. Some emitters omit
/// the whole `tool` block when they don't have a version, which would
/// erase the agent identity entirely. To preserve information we look at:
///
///   1. The record-level `tool.name` (when present).
///   2. The conversation URL hostname (e.g. `ampcode.com` → `amp`,
///      `cursor.sh` / `cursor.com` → `cursor`, `claude.ai` → `claude-code`).
///   3. The session URN agent slug (`urn:<scheme>:session:<agent>:<id>`).
///   4. `"agent-trace"` if nothing else is available.
fn agent_for_conversation(conv: &Conversation, record_tool_name: Option<&str>) -> String {
    if let Some(n) = record_tool_name.filter(|s| !s.is_empty()) {
        return n.to_string();
    }
    if let Some(url) = conv.url.as_deref() {
        if let Some(slug) = agent_from_url_host(url) {
            return slug.to_string();
        }
    }
    for r in &conv.related {
        if r.ty == "session" {
            if let Some(slug) = agent_from_session_urn(&r.url) {
                return slug.to_string();
            }
        }
    }
    "agent-trace".to_string()
}

/// Map well-known conversation URL hosts to short agent slugs.
fn agent_from_url_host(url: &str) -> Option<&'static str> {
    // Cheap manual host extraction — no need to pull in a URL crate just
    // for hostname matching.
    let after_scheme = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = after_scheme.split('/').next().unwrap_or("");
    let host = host.split('@').next_back().unwrap_or(host); // strip any user-info
    let host = host.split(':').next().unwrap_or(host); // strip port

    match host {
        h if h.ends_with("ampcode.com") => Some("amp"),
        h if h.ends_with("cursor.sh") || h.ends_with("cursor.com") => Some("cursor"),
        h if h.ends_with("claude.ai") => Some("claude-code"),
        h if h.ends_with("openai.com") || h.ends_with("chatgpt.com") => Some("codex"),
        h if h.ends_with("block.xyz") => Some("goose"),
        _ => None,
    }
}

/// Parse the agent slug out of a session URN of the form
/// `urn:<namespace>:session:<agent>:<id>` (or any variation that puts the
/// agent immediately after a `session` segment).
fn agent_from_session_urn(urn: &str) -> Option<&str> {
    let parts: Vec<&str> = urn.split(':').collect();
    let idx = parts.iter().position(|p| *p == "session")?;
    parts.get(idx + 1).copied().filter(|s| !s.is_empty())
}

/// Return the trailing non-empty path / URN segment, splitting on both `/`
/// and `:` so it works for HTTPS URLs and `urn:` identifiers alike.
fn trailing_segment(s: &str) -> Option<String> {
    let trimmed = s.trim_end_matches(['/', ':']);
    let last = trimmed.rsplit(['/', ':']).find(|seg| !seg.is_empty())?;
    Some(last.to_string())
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
        // Pull what we can out of the (optional) record-level `tool` block.
        // Per-conversation derivation below will use `record_tool_name` as
        // its preferred source, then fall back to URL/URN sniffing.
        let (record_tool_name, agent_version) = match record.tool {
            Some(t) => (t.name, t.version),
            None => (None, None),
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
                // Derive per-conversation so a single record covering
                // multiple agents (e.g. a hand-off) attributes each
                // conversation correctly.
                let agent_tool = agent_for_conversation(&conv, record_tool_name.as_deref());

                for range in &conv.ranges {
                    let model = model_for_range(range, &conv);
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
            first.session_id, "T-019d872e-ff8e-760c-8686-6850d686a3f5",
            "session_id is the trailing path segment of the conversation URL"
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
                    "related": [{"type": "session", "url": "urn:agent:session:foo:abc-123"}]
                }]
            }]
        }"#;
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), body);
        let edits = AgentTraceParser::new().parse_file(&path, "").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(
            edits[0].session_id, "abc-123",
            "trailing URN segment becomes the session id"
        );
        assert_eq!(edits[0].model, "unknown");
        assert_eq!(
            edits[0].agent_tool, "foo",
            "agent slug is recovered from the URN even with no top-level tool block"
        );
        assert!(edits[0].agent_version.is_none());
    }

    #[test]
    fn derives_agent_from_amp_url_when_tool_block_absent() {
        // Real-world shape: producer omitted the tool block (spec requires
        // both name AND version, so emitters that lack version drop the
        // whole block) but the conversation URL identifies the agent.
        let body = r#"{
            "version": "1.0",
            "id": "x",
            "timestamp": "2026-04-13T14:12:21Z",
            "files": [{
                "path": "main.rs",
                "conversations": [{
                    "url": "https://ampcode.com/threads/T-019d872e",
                    "contributor": {"type": "ai"},
                    "ranges": [{"start_line": 1, "end_line": 1}]
                }]
            }]
        }"#;
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), body);
        let edits = AgentTraceParser::new().parse_file(&path, "").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].agent_tool, "amp");
        assert_eq!(edits[0].session_id, "T-019d872e");
        assert_eq!(
            edits[0].model, "ai (model unspecified)",
            "contributor.type fills in when model_id is missing"
        );
    }

    #[test]
    fn agent_url_host_mapping() {
        assert_eq!(
            agent_from_url_host("https://ampcode.com/threads/T-1"),
            Some("amp")
        );
        assert_eq!(
            agent_from_url_host("https://app.cursor.com/x"),
            Some("cursor")
        );
        assert_eq!(agent_from_url_host("https://cursor.sh/x"), Some("cursor"));
        assert_eq!(
            agent_from_url_host("https://claude.ai/chat/x"),
            Some("claude-code")
        );
        assert_eq!(agent_from_url_host("https://chatgpt.com/x"), Some("codex"));
        assert_eq!(
            agent_from_url_host("https://goose.block.xyz/x"),
            Some("goose")
        );
        assert_eq!(agent_from_url_host("https://example.invalid/x"), None);
    }

    #[test]
    fn agent_session_urn_extraction() {
        assert_eq!(
            agent_from_session_urn("urn:sq-agents:session:amp:T-019d872e"),
            Some("amp")
        );
        assert_eq!(
            agent_from_session_urn("urn:agent:session:cursor:abc"),
            Some("cursor")
        );
        assert_eq!(agent_from_session_urn("urn:foo:bar:baz"), None);
    }

    #[test]
    fn record_tool_name_takes_priority_over_url_sniffing() {
        let body = r#"{
            "version": "1.0",
            "id": "x",
            "timestamp": "2026-04-13T14:12:21Z",
            "tool": {"name": "bespoke-agent", "version": "9.9"},
            "files": [{
                "path": "a.rs",
                "conversations": [{
                    "url": "https://ampcode.com/threads/T-1",
                    "ranges": [{"start_line": 1, "end_line": 1}]
                }]
            }]
        }"#;
        let dir = tempdir().unwrap();
        let path = write_sample(dir.path(), body);
        let edits = AgentTraceParser::new().parse_file(&path, "").unwrap();
        assert_eq!(edits[0].agent_tool, "bespoke-agent");
        assert_eq!(edits[0].agent_version.as_deref(), Some("9.9"));
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
