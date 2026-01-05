use ai_blame::extractor::extract_edit_history;
use ai_blame::models::FilterConfig;
use std::path::PathBuf;

#[test]
fn test_extract_edit_history_from_repo_fixtures() {
    let trace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("traces");

    // Clear cache to ensure fresh parsing
    let _ = std::fs::remove_file(trace_dir.join(".ai-blame.ddb"));

    let config = FilterConfig::default();
    let edits_by_file = extract_edit_history(&trace_dir, &config).unwrap();

    // Fixture contains edits for exactly two files.
    assert!(edits_by_file.contains_key("/repo/src/main.rs"));
    assert!(edits_by_file.contains_key("/repo/src/lib.rs"));

    // main.rs: two edits across two fixture files, sorted by timestamp ascending.
    let main_edits = &edits_by_file["/repo/src/main.rs"];
    assert_eq!(main_edits.len(), 2);
    assert_eq!(main_edits[0].model, "claude-test-model");
    assert_eq!(main_edits[0].session_id, "s1");
    assert!(!main_edits[0].is_create);
    assert_eq!(main_edits[0].old_string.as_deref(), Some("a"));
    assert_eq!(main_edits[0].new_string.as_deref(), Some("b"));
    assert_eq!(main_edits[1].model, "claude-test-agent");
    assert_eq!(main_edits[1].session_id, "s2");
    assert_eq!(main_edits[1].old_string.as_deref(), Some("b"));
    assert_eq!(main_edits[1].new_string.as_deref(), Some("B"));

    // lib.rs: a single create event with content.
    let lib_edits = &edits_by_file["/repo/src/lib.rs"];
    assert_eq!(lib_edits.len(), 1);
    assert!(lib_edits[0].is_create);
    assert_eq!(lib_edits[0].model, "claude-test-model");
    assert_eq!(
        lib_edits[0].create_content.as_deref(),
        Some("pub fn hello() {}\n")
    );
}
