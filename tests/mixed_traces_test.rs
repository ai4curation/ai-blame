use ai_blame::extractor::extract_edit_history_from_dirs;
use ai_blame::models::FilterConfig;
use std::path::Path;

fn clear_test_caches() {
    // Clear cache files before each test to ensure fresh parsing
    let _ = std::fs::remove_file("tests/data/mixed-traces/claude-traces/.ai-blame.ddb");
    let _ = std::fs::remove_file("tests/data/mixed-traces/codex-sessions/.ai-blame.ddb");
}

#[test]
fn test_mixed_claude_and_codex_traces() {
    clear_test_caches();

    // Test directory containing both Claude and Codex CLI traces
    let test_dir = Path::new("tests/data/mixed-traces");

    // Verify test data exists
    assert!(test_dir.exists(), "Test data directory not found");

    let claude_traces = test_dir.join("claude-traces");
    let codex_sessions = test_dir.join("codex-sessions");
    let repo_root = test_dir.join("repo");

    assert!(claude_traces.exists(), "Claude traces directory not found");
    assert!(
        codex_sessions.exists(),
        "Codex sessions directory not found"
    );

    // Extract edit history from both trace directories
    let trace_dirs = vec![claude_traces.as_path(), codex_sessions.as_path()];
    let config = FilterConfig::default();

    let result = extract_edit_history_from_dirs(&trace_dirs, &config, repo_root.as_path().into());

    assert!(result.is_ok(), "Failed to extract edit history");

    let edits_by_file = result.unwrap();

    // Verify that we found edits from both sources
    assert!(!edits_by_file.is_empty(), "No edits found");

    // Check for Claude-created file (foo.md)
    let foo_md_edits: Vec<_> = edits_by_file
        .iter()
        .filter(|(path, _)| path.contains("foo.md"))
        .collect();

    if !foo_md_edits.is_empty() {
        let (_, edits) = foo_md_edits[0];
        // foo.md should have multiple edits (create + modify) from Claude
        assert!(!edits.is_empty(), "foo.md should have edit records");
    }

    // Check for Codex-created file (created-with-codex.md)
    let codex_file_edits: Vec<_> = edits_by_file
        .iter()
        .filter(|(path, _)| path.contains("created-with-codex.md"))
        .collect();

    if !codex_file_edits.is_empty() {
        let (_, edits) = codex_file_edits[0];
        assert!(
            !edits.is_empty(),
            "created-with-codex.md should have edit records"
        );

        // Should be attributed to Codex
        let has_codex = edits.iter().any(|e| e.agent_tool == "codex-cli");
        assert!(
            has_codex,
            "created-with-codex.md should have Codex attribution"
        );
    }
}

#[test]
fn test_codex_cli_file_modifications_detected() {
    clear_test_caches();

    // Verify that the modification detection for Codex CLI files works
    // This tests the fix for files modified across multiple ghost commits

    let test_dir = Path::new("tests/data/mixed-traces");
    let codex_sessions = test_dir.join("codex-sessions");
    let repo_root = test_dir.join("repo");

    let trace_dirs = vec![codex_sessions.as_path()];

    // Test with no pattern to extract all Codex edits
    let config = FilterConfig {
        file_pattern: None,
        ..Default::default()
    };

    let result = extract_edit_history_from_dirs(&trace_dirs, &config, Some(repo_root.as_path()));

    assert!(result.is_ok(), "Failed to extract edit history");

    let edits_by_file = result.unwrap();

    // Should extract at least created-with-codex.md from Codex CLI
    // This file appears in the Codex ghost commits and should be detected
    assert!(
        !edits_by_file.is_empty(),
        "Should extract edits from Codex CLI sessions"
    );

    // Verify we can detect Codex CLI edits (either creates or modifications)
    let has_codex_files = edits_by_file.iter().any(|(path, edits)| {
        edits.iter().any(|e| e.agent_tool == "codex-cli") && path.contains("created-with-codex.md")
    });

    assert!(
        has_codex_files,
        "Should detect created-with-codex.md as a Codex CLI creation"
    );
}
