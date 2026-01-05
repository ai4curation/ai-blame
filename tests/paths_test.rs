use std::path::Path;

#[test]
fn test_encode_claude_project_dir_name_replaces_dot() {
    let p = Path::new("/Users/cjm/repos/ai-blame.rs");
    let encoded = ai_blame::paths::encode_claude_project_dir_name(p);
    assert_eq!(encoded, "-Users-cjm-repos-ai-blame-rs");
}
