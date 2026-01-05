use crate::models::EditRecord;
use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameMeta {
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub session_id: String,
    pub agent_tool: String,
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBlame {
    /// 1-based line number in the current file.
    pub line_no: usize,
    pub text: String,
    /// Attribution for this line (None means unknown/unattributed).
    pub meta: Option<BlameMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub meta: Option<BlameMeta>,
}

fn normalize_lines(s: &str) -> Vec<String> {
    let s = s.replace("\r\n", "\n");
    let s = s.trim_end_matches('\n');
    if s.is_empty() {
        return Vec::new();
    }
    s.split('\n').map(|l| l.to_string()).collect()
}

fn parse_new_start_line(structured_patch: &Option<String>) -> Option<usize> {
    let patch = structured_patch.as_deref()?;
    // Try to extract "+<start>[,<count>]" from a unified diff hunk header.
    // Example: "@@ -12,3 +45,7 @@"
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"@@\s*-\d+(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s*@@")
            .expect("static regex must compile")
    });
    re.captures(patch)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}

fn find_subslice(
    haystack: &[String],
    needle: &[String],
    hint_line: Option<usize>,
) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    let mut matches = Vec::new();
    for i in 0..=(haystack.len() - needle.len()) {
        if haystack[i..i + needle.len()] == *needle {
            matches.push(i);
        }
    }
    if matches.is_empty() {
        return None;
    }
    if matches.len() == 1 || hint_line.is_none() {
        return Some(matches[0]);
    }

    let hint = hint_line.unwrap_or(1).saturating_sub(1);
    matches.into_iter().min_by_key(|i| i.abs_diff(hint))
}

fn to_meta(edit: &EditRecord) -> BlameMeta {
    BlameMeta {
        timestamp: edit.timestamp,
        model: edit.model.clone(),
        session_id: edit.session_id.clone(),
        agent_tool: edit.agent_tool.clone(),
        agent_version: edit.agent_version.clone(),
    }
}

/// Compute per-line blame for `current_content` using edits (typically from traces) via a
/// reverse-apply strategy. This approximates `git blame` for the current working tree.
pub fn compute_line_blame(current_content: &str, edits: &[EditRecord]) -> Result<Vec<LineBlame>> {
    let original_lines = normalize_lines(current_content);
    let mut blame: Vec<Option<BlameMeta>> = vec![None; original_lines.len()];

    // Working lines and a mapping from working line index -> original current line index.
    let mut working_lines = original_lines.clone();
    let mut mapping: Vec<Option<usize>> = (0..original_lines.len()).map(Some).collect();

    // Process newest -> oldest.
    let mut edits_desc: Vec<&EditRecord> = edits.iter().collect();
    edits_desc.sort_by_key(|e| e.timestamp);
    edits_desc.reverse();

    for edit in edits_desc {
        let meta = to_meta(edit);

        if edit.is_create {
            // Assign any remaining unknown lines to the create event.
            for slot in blame.iter_mut() {
                if slot.is_none() {
                    *slot = Some(meta.clone());
                }
            }
            break;
        }

        let new_text = match edit.new_string.as_deref() {
            Some(s) => s,
            None => continue,
        };
        if new_text.trim().is_empty() {
            // Deletions / empty inserts don't affect blame for existing lines; skip.
            continue;
        }
        let new_lines = normalize_lines(new_text);
        if new_lines.is_empty() {
            continue;
        }

        let hint_line = parse_new_start_line(&edit.structured_patch);
        let start_idx = match find_subslice(&working_lines, &new_lines, hint_line) {
            Some(i) => i,
            None => continue,
        };

        // Assign blame to the corresponding original current lines, but only if still unknown.
        for j in start_idx..start_idx + new_lines.len() {
            if let Some(orig_idx) = mapping.get(j).copied().flatten() {
                if orig_idx < blame.len() && blame[orig_idx].is_none() {
                    blame[orig_idx] = Some(meta.clone());
                }
            }
        }

        // Reverse-apply by replacing new_lines with old_lines.
        let old_lines = normalize_lines(edit.old_string.as_deref().unwrap_or(""));
        working_lines.splice(
            start_idx..start_idx + new_lines.len(),
            old_lines.iter().cloned(),
        );
        mapping.splice(
            start_idx..start_idx + new_lines.len(),
            std::iter::repeat_n(None, old_lines.len()),
        );
    }

    Ok(original_lines
        .into_iter()
        .enumerate()
        .map(|(i, text)| LineBlame {
            line_no: i + 1,
            text,
            meta: blame[i].clone(),
        })
        .collect())
}

pub fn group_blocks(lines: &[LineBlame]) -> Vec<BlameBlock> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();
    let mut start = 1;
    let mut current_meta = lines[0].meta.clone();

    for i in 2..=lines.len() {
        let meta = lines[i - 1].meta.clone();
        if meta != current_meta {
            blocks.push(BlameBlock {
                start_line: start,
                end_line: i - 1,
                meta: current_meta,
            });
            start = i;
            current_meta = meta;
        }
    }
    blocks.push(BlameBlock {
        start_line: start,
        end_line: lines.len(),
        meta: current_meta,
    });
    blocks
}
