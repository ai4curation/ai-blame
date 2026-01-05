use ai_blame::blame::{compute_line_blame, group_blocks};
use ai_blame::models::EditRecord;
use chrono::{TimeZone, Utc};

fn mk_edit(
    ts: (i32, u32, u32, u32, u32, u32),
    old: &str,
    new_: &str,
    structured_patch: Option<&str>,
) -> EditRecord {
    EditRecord {
        file_path: "src/main.rs".to_string(),
        timestamp: Utc
            .with_ymd_and_hms(ts.0, ts.1, ts.2, ts.3, ts.4, ts.5)
            .unwrap(),
        model: "claude-test".to_string(),
        session_id: "s1".to_string(),
        is_create: false,
        change_size: 1,
        agent_tool: "claude-code".to_string(),
        agent_version: None,
        old_string: Some(old.to_string()),
        new_string: Some(new_.to_string()),
        structured_patch: structured_patch.map(|s| s.to_string()),
        create_content: None,
    }
}

#[test]
fn test_compute_line_blame_prefers_newest_edit() {
    let current = "a\nB\nc\n";
    let edits = vec![
        mk_edit((2025, 12, 1, 8, 0, 0), "x", "b", None),
        mk_edit((2025, 12, 1, 9, 0, 0), "b", "B", None),
    ];

    let blamed = compute_line_blame(current, &edits).unwrap();
    assert_eq!(blamed.len(), 3);
    assert!(blamed[0].meta.is_none());
    assert_eq!(blamed[1].text, "B");
    assert_eq!(
        blamed[1].meta.as_ref().unwrap().timestamp,
        Utc.with_ymd_and_hms(2025, 12, 1, 9, 0, 0).unwrap()
    );
    assert!(blamed[2].meta.is_none());
}

#[test]
fn test_compute_line_blame_multiline_replacement_assigns_span() {
    let current = "a\nb\nc\nd\n";
    let edits = vec![mk_edit(
        (2025, 12, 1, 9, 0, 0),
        "x\ny",
        "b\nc",
        Some("@@ -1,2 +2,2 @@"),
    )];

    let blamed = compute_line_blame(current, &edits).unwrap();
    assert_eq!(blamed.len(), 4);
    assert!(blamed[0].meta.is_none());
    assert!(blamed[1].meta.is_some());
    assert!(blamed[2].meta.is_some());
    assert!(blamed[3].meta.is_none());
}

#[test]
fn test_group_blocks_splits_on_meta_changes() {
    let current = "a\nb\nc\n";
    let edits = vec![mk_edit((2025, 12, 1, 9, 0, 0), "x", "b", None)];
    let blamed = compute_line_blame(current, &edits).unwrap();

    let blocks = group_blocks(&blamed);
    // a (unknown), b (blamed), c (unknown) => 3 blocks
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].start_line, 1);
    assert_eq!(blocks[0].end_line, 1);
    assert_eq!(blocks[1].start_line, 2);
    assert_eq!(blocks[1].end_line, 2);
    assert_eq!(blocks[2].start_line, 3);
    assert_eq!(blocks[2].end_line, 3);
}
