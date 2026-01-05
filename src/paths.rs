use std::path::{Path, PathBuf};

/// Encode an absolute project path into the directory name Claude Code uses under:
/// `~/.claude/projects/<encoded>/`.
///
/// Observed behavior (and goal here):
/// - Path separators (`/` on Unix, `\` on Windows) become `-`
/// - Punctuation like `.` becomes `-` (e.g. `ai-blame.rs` -> `ai-blame-rs`)
/// - Any non `[A-Za-z0-9_-]` character becomes `-` for filesystem safety
pub fn encode_claude_project_dir_name(project_dir: &Path) -> String {
    let s = project_dir.to_string_lossy();
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let mapped = match ch {
            '/' | '\\' => '-',
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => c,
            _ => '-',
        };
        out.push(mapped);
    }
    out
}

fn encode_claude_project_dir_name_legacy_slashes_only(project_dir: &Path) -> String {
    project_dir
        .to_string_lossy()
        .to_string()
        .replace(['/', '\\'], "-")
}

/// Resolve the expected Claude trace directory for a project.
///
/// This prefers the modern encoding (which matches Claude Code's observed behavior), but
/// falls back to a legacy encoding (slashes only) if that directory exists.
pub fn resolve_claude_trace_dir(home_dir: &Path, project_dir: &Path) -> PathBuf {
    let projects_root = home_dir.join(".claude").join("projects");

    let encoded = encode_claude_project_dir_name(project_dir);
    let candidate = projects_root.join(&encoded);
    if candidate.exists() {
        return candidate;
    }

    let legacy = encode_claude_project_dir_name_legacy_slashes_only(project_dir);
    let legacy_candidate = projects_root.join(legacy);
    if legacy_candidate.exists() {
        return legacy_candidate;
    }

    candidate
}
