use ai_blame::extractor::collect_jsonl_files;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Create a minimal valid Claude trace file with an edit
fn create_trace_file(trace_dir: &Path, filename: &str) -> std::io::Result<PathBuf> {
    let path = trace_dir.join(filename);
    let mut file = File::create(&path)?;

    // Parent message with model
    writeln!(
        file,
        r#"{{"uuid":"parent-1","message":{{"model":"claude-opus-4-5-20251101"}}}}"#
    )?;

    // Successful edit
    writeln!(
        file,
        r#"{{"uuid":"edit-1","parentUuid":"parent-1","type":"user","timestamp":"2025-12-01T08:03:42Z","sessionId":"session-1","toolUseResult":{{"filePath":"src/main.rs","structuredPatch":"@@ -1 +1 @@","oldString":"fn main() {{","newString":"fn main() {{\n    println!(\"updated\");"}} }}"#
    )?;

    Ok(path)
}

/// Test that collect_jsonl_files handles symlink cycles without hanging
#[test]
fn test_collect_jsonl_files_detects_symlink_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    create_trace_file(&trace_dir, "session.jsonl").unwrap();

    // Create a symlink cycle: link back to the trace_dir itself
    let link_path = trace_dir.join("cyclic_link");
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        // This creates a cycle: trace_dir/cyclic_link -> trace_dir
        let _ = unix_fs::symlink(&trace_dir, &link_path);
    }

    // This should not hang - the fix should detect and skip the cycle
    let mut files = Vec::new();
    let result = collect_jsonl_files(&trace_dir, &mut files);

    assert!(
        result.is_ok(),
        "collect_jsonl_files should not error on symlink cycles"
    );
    assert_eq!(
        files.len(),
        1,
        "Should find exactly one .jsonl file (the real session.jsonl)"
    );
    assert!(
        files[0].ends_with("session.jsonl"),
        "Should find the session.jsonl file"
    );
}

/// Test that collect_jsonl_files finds files in nested directories
#[test]
fn test_collect_jsonl_files_finds_nested_files() {
    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    let projects_dir = trace_dir.join("projects");
    fs::create_dir(&projects_dir).unwrap();

    let sessions_dir = projects_dir.join("sessions");
    fs::create_dir(&sessions_dir).unwrap();

    // Create trace files at different levels
    create_trace_file(&trace_dir, "root.jsonl").unwrap();
    create_trace_file(&projects_dir, "project.jsonl").unwrap();
    create_trace_file(&sessions_dir, "session.jsonl").unwrap();

    let mut files = Vec::new();
    let result = collect_jsonl_files(&trace_dir, &mut files);

    assert!(result.is_ok(), "collect_jsonl_files should succeed");
    assert_eq!(files.len(), 3, "Should find all three .jsonl files");
}

/// Test that collect_jsonl_files handles empty directories
#[test]
fn test_collect_jsonl_files_handles_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    let mut files = Vec::new();
    let result = collect_jsonl_files(&trace_dir, &mut files);

    assert!(
        result.is_ok(),
        "collect_jsonl_files should handle empty directory"
    );
    assert_eq!(files.len(), 0, "Should find no files in empty directory");
}

/// Test that collect_jsonl_files ignores non-jsonl files
#[test]
fn test_collect_jsonl_files_ignores_other_extensions() {
    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    create_trace_file(&trace_dir, "session.jsonl").unwrap();

    // Create some non-jsonl files
    File::create(trace_dir.join("readme.md")).unwrap();
    File::create(trace_dir.join("data.json")).unwrap();
    File::create(trace_dir.join("log.txt")).unwrap();

    let mut files = Vec::new();
    let result = collect_jsonl_files(&trace_dir, &mut files);

    assert!(result.is_ok(), "collect_jsonl_files should succeed");
    assert_eq!(files.len(), 1, "Should only find the .jsonl file");
    assert!(
        files[0].ends_with("session.jsonl"),
        "Should find the session.jsonl file"
    );
}

/// Test that collect_jsonl_files handles multiple levels of symlinks correctly
#[test]
fn test_collect_jsonl_files_handles_complex_symlink_scenarios() {
    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    let subdir1 = trace_dir.join("subdir1");
    fs::create_dir(&subdir1).unwrap();

    let subdir2 = trace_dir.join("subdir2");
    fs::create_dir(&subdir2).unwrap();

    create_trace_file(&subdir1, "trace1.jsonl").unwrap();
    create_trace_file(&subdir2, "trace2.jsonl").unwrap();

    // Create a symlink from subdir2 to subdir1 (not a cycle yet)
    let link_path = subdir2.join("link_to_subdir1");
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        let _ = unix_fs::symlink(&subdir1, &link_path);
    }

    let mut files = Vec::new();
    let result = collect_jsonl_files(&trace_dir, &mut files);

    assert!(result.is_ok(), "collect_jsonl_files should succeed");
    // Should find trace2.jsonl and trace1.jsonl (possibly twice if symlink is followed)
    // but should not hang
    assert!(!files.is_empty(), "Should find at least one .jsonl file");
}

/// Test that timeline command can extract edits from trace files
#[test]
fn test_timeline_command_extracts_edits() {
    use ai_blame::extractor::extract_edit_history_from_dirs;
    use ai_blame::models::FilterConfig;

    let temp_dir = TempDir::new().unwrap();
    let trace_dir = temp_dir.path().join(".claude");
    fs::create_dir(&trace_dir).unwrap();

    // Create a trace file with multiple edits
    let trace_path = trace_dir.join("session.jsonl");
    let mut file = File::create(&trace_path).unwrap();

    // Parent message with model
    writeln!(
        &mut file,
        r#"{{"uuid":"parent-1","message":{{"model":"claude-opus-4-5-20251101"}}}}"#
    )
    .unwrap();

    // First edit
    writeln!(
        &mut file,
        r#"{{"uuid":"edit-1","parentUuid":"parent-1","type":"user","timestamp":"2025-12-01T08:00:00Z","sessionId":"session-1","toolUseResult":{{"filePath":"src/main.rs","structuredPatch":"@@ -1 +1 @@","oldString":"fn main() {{","newString":"fn main() {{\n    println!(\"v1\");"}} }}"#
    )
    .unwrap();

    // Second edit (later timestamp)
    writeln!(
        &mut file,
        r#"{{"uuid":"edit-2","parentUuid":"parent-1","type":"user","timestamp":"2025-12-01T09:00:00Z","sessionId":"session-1","toolUseResult":{{"filePath":"src/lib.rs","structuredPatch":"@@ -1 +1 @@","oldString":"pub fn test() {{","newString":"pub fn test() {{\n    println!(\"v2\");"}} }}"#
    )
    .unwrap();

    drop(file);

    // Extract edits
    let config = FilterConfig::default();
    let trace_dirs = vec![trace_dir.as_path()];
    let edits_by_file = extract_edit_history_from_dirs(&trace_dirs, &config, None).unwrap();

    // Collect all edits
    let mut all_edits: Vec<_> = edits_by_file
        .values()
        .flat_map(|edits| edits.iter())
        .collect();

    // Sort by timestamp (most recent first)
    all_edits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Should find both edits
    assert_eq!(all_edits.len(), 2, "Should find 2 edits");

    // Most recent edit should be the lib.rs edit from 09:00
    assert_eq!(
        all_edits[0].file_path, "src/lib.rs",
        "Most recent edit should be to lib.rs"
    );
    assert_eq!(
        all_edits[0]
            .timestamp
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        "2025-12-01 09:00:00"
    );

    // Older edit should be the main.rs edit from 08:00
    assert_eq!(
        all_edits[1].file_path, "src/main.rs",
        "Older edit should be to main.rs"
    );
    assert_eq!(
        all_edits[1]
            .timestamp
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        "2025-12-01 08:00:00"
    );
}
