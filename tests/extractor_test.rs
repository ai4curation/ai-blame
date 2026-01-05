use ai_blame::extractor::extract_edit_history;
use ai_blame::extractor::parse_trace_file;
use chrono::{TimeZone, Utc};
use std::io::Write;

#[test]
fn test_parse_trace_file_parses_rfc3339_z_timestamp() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    // Parent message provides the model.
    writeln!(
        tmp,
        r#"{{"uuid":"parent","message":{{"model":"claude-test"}}}}"#
    )
    .unwrap();

    // Successful edit message.
    writeln!(
        tmp,
        r#"{{"uuid":"child","parentUuid":"parent","type":"user","timestamp":"2025-12-01T08:03:42Z","sessionId":"s1","toolUseResult":{{"filePath":"/repo/src/main.rs","structuredPatch":"@@ -1 +1 @@","oldString":"a","newString":"b"}}}}"#
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].model, "claude-test");
    assert_eq!(edits[0].old_string.as_deref(), Some("a"));
    assert_eq!(edits[0].new_string.as_deref(), Some("b"));
    assert_eq!(edits[0].structured_patch.as_deref(), Some("@@ -1 +1 @@"));
    assert_eq!(
        edits[0].timestamp,
        Utc.with_ymd_and_hms(2025, 12, 1, 8, 3, 42).unwrap()
    );
}

#[test]
fn test_parse_trace_file_skips_invalid_timestamp() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    writeln!(
        tmp,
        r#"{{"uuid":"parent","message":{{"model":"claude-test"}}}}"#
    )
    .unwrap();
    writeln!(
        tmp,
        r#"{{"uuid":"child","parentUuid":"parent","type":"user","timestamp":"not-a-timestamp","sessionId":"s1","toolUseResult":{{"filePath":"/repo/src/main.rs","structuredPatch":"@@ -1 +1 @@","oldString":"a","newString":"b"}}}}"#
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert!(edits.is_empty());
}

#[test]
fn test_extract_edit_history_resolves_model_across_trace_files() {
    let dir = tempfile::tempdir().unwrap();

    // Parent UUID + model in one trace file.
    let mut main_trace = std::fs::File::create(dir.path().join("main.jsonl")).unwrap();
    writeln!(
        main_trace,
        r#"{{"uuid":"parent-x","message":{{"model":"claude-cross-file"}}}}"#
    )
    .unwrap();

    // Edit in a separate agent trace file that references the parent UUID.
    let mut agent_trace = std::fs::File::create(dir.path().join("agent-sub.jsonl")).unwrap();
    writeln!(
        agent_trace,
        r#"{{"uuid":"child-x","parentUuid":"parent-x","type":"user","timestamp":"2025-12-21T01:40:00Z","sessionId":"s-x","toolUseResult":{{"filePath":"/repo/src/peel.py","structuredPatch":"@@ -1 +1 @@","oldString":"a","newString":"b"}}}}"#
    )
    .unwrap();

    let edits_by_file = extract_edit_history(dir.path(), &Default::default()).unwrap();
    let edits = edits_by_file.get("/repo/src/peel.py").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].model, "claude-cross-file");
    assert_eq!(edits[0].agent_tool, "claude-code-agent");
}

#[test]
fn test_parse_trace_file_treats_content_without_explicit_type_as_create() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    writeln!(
        tmp,
        r#"{{"uuid":"child","type":"user","timestamp":"2025-12-01T08:05:00Z","sessionId":"s1","toolUseResult":{{"filePath":"/repo/src/lib.rs","content":"pub fn hello() {{}}\n"}}}}"#
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 1);
    assert!(edits[0].is_create);
    assert_eq!(
        edits[0].create_content.as_deref(),
        Some("pub fn hello() {}\n")
    );
}

#[test]
fn test_parse_trace_file_resolves_model_via_tool_use_id() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    // Assistant message with a tool_use id and model.
    writeln!(
        tmp,
        r#"{{"uuid":"assistant-1","type":"assistant","message":{{"model":"claude-tooluse-model","content":[{{"type":"tool_use","id":"toolu_abc","name":"Write","input":{{"file_path":"/repo/main.py","content":"x"}}}}]}}}}"#
    )
    .unwrap();

    // Tool result message that references tool_use_id but has a parentUuid that doesn't map to an assistant model.
    writeln!(
        tmp,
        r#"{{"uuid":"tool-result-1","parentUuid":"some-nonassistant","type":"user","timestamp":"2025-12-01T08:03:42Z","sessionId":"s1","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"toolu_abc","content":"ok"}}]}},"toolUseResult":{{"type":"create","filePath":"/repo/main.py","content":"print(1)\n"}}}}"#
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].model, "claude-tooluse-model");
    assert!(edits[0].is_create);
}

#[test]
fn test_parse_codex_format_create() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    // Codex-style create record
    writeln!(
        tmp,
        "{{\"event\":\"create\",\"file\":\"/repo/src/lib.rs\",\"model\":\"gpt-4\",\"timestamp\":\"2025-12-01T08:05:00Z\",\"session_id\":\"codex-session-1\",\"content\":\"pub fn hello() {{}}\\n\"}}"
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].model, "gpt-4");
    assert_eq!(edits[0].agent_tool, "github-copilot");
    assert!(edits[0].is_create);
    assert_eq!(
        edits[0].create_content.as_deref(),
        Some("pub fn hello() {}\n")
    );
}

#[test]
fn test_parse_codex_format_edit() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    // Codex-style edit record
    writeln!(
        tmp,
        "{{\"event\":\"edit\",\"file_path\":\"/repo/src/main.rs\",\"model\":\"codex-davinci-002\",\"timestamp\":\"2025-12-01T10:30:00Z\",\"session_id\":\"codex-session-2\",\"old_content\":\"fn main() {{}}\",\"new_content\":\"fn main() {{\\n    println!(\\\"Hello\\\");\\n}}\"}}"
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].model, "codex-davinci-002");
    assert_eq!(edits[0].agent_tool, "github-copilot");
    assert!(!edits[0].is_create);
    assert_eq!(edits[0].old_string.as_deref(), Some("fn main() {}"));
    assert!(edits[0].new_string.as_ref().unwrap().contains("println!"));
}

#[test]
fn test_parse_mixed_claude_and_codex_traces() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();

    // Claude-style record
    writeln!(
        tmp,
        r#"{{"uuid":"parent","message":{{"model":"claude-test"}}}}"#
    )
    .unwrap();
    writeln!(
        tmp,
        r#"{{"uuid":"child","parentUuid":"parent","type":"user","timestamp":"2025-12-01T08:00:00Z","sessionId":"s1","toolUseResult":{{"filePath":"/repo/file1.rs","structuredPatch":"@@ -1 +1 @@","oldString":"a","newString":"b"}}}}"#
    )
    .unwrap();

    // Codex-style record
    writeln!(
        tmp,
        "{{\"event\":\"create\",\"file\":\"/repo/file2.py\",\"model\":\"gpt-4\",\"timestamp\":\"2025-12-01T08:10:00Z\",\"session_id\":\"codex-1\",\"content\":\"# test\"}}"
    )
    .unwrap();

    let edits = parse_trace_file(tmp.path(), "").unwrap();
    assert_eq!(edits.len(), 2);

    // First edit should be Claude
    assert_eq!(edits[0].model, "claude-test");
    assert_eq!(edits[0].agent_tool, "claude-code");

    // Second edit should be Codex
    assert_eq!(edits[1].model, "gpt-4");
    assert_eq!(edits[1].agent_tool, "github-copilot");
    assert!(edits[1].is_create);
}
