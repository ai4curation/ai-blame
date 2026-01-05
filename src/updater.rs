use crate::config::resolve_sidecar_path;
use crate::models::*;
use anyhow::{Context, Result};
use regex::Regex;
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub fn generate_curation_yaml(history: &FileHistory) -> Result<String> {
    let events_data: Vec<_> = history.events.iter().collect();
    let mut map = HashMap::new();
    map.insert("edit_history", events_data);

    let yaml = serde_yaml::to_string(&map)?;
    Ok(yaml)
}

pub fn generate_curation_json(history: &FileHistory) -> Result<String> {
    let events_data: Vec<_> = history.events.iter().collect();
    let mut map = HashMap::new();
    map.insert("edit_history", events_data);

    let json = serde_json::to_string_pretty(&map)?;
    Ok(json)
}

pub fn append_yaml(
    file_path: &Path,
    history: &FileHistory,
    dry_run: bool,
) -> Result<(bool, String)> {
    if !file_path.exists() {
        return Ok((false, format!("File not found: {:?}", file_path)));
    }

    let mut content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    // Check if edit_history already exists and remove it
    if content.contains("edit_history:") {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut new_lines = Vec::new();
        let mut in_curation = false;

        for line in lines {
            if line.starts_with("edit_history:") {
                in_curation = true;
                continue;
            }
            if in_curation {
                // Check if this is a new top-level key (not indented and not empty)
                if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('-') {
                    in_curation = false;
                    new_lines.push(line);
                }
                continue;
            }
            new_lines.push(line);
        }
        content = new_lines.join("\n");
    }

    // Generate new curation history
    let curation_yaml = generate_curation_yaml(history)?;

    // Ensure content ends with newline
    if !content.ends_with('\n') {
        content.push('\n');
    }

    // Add blank line before edit_history if not already there
    if !content.ends_with("\n\n") {
        content.push('\n');
    }

    let new_content = format!("{}{}", content, curation_yaml);

    if dry_run {
        return Ok((true, new_content));
    }

    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;
    Ok((true, format!("Updated: {:?}", file_path)))
}

pub fn append_json(
    file_path: &Path,
    history: &FileHistory,
    dry_run: bool,
) -> Result<(bool, String)> {
    if !file_path.exists() {
        return Ok((false, format!("File not found: {:?}", file_path)));
    }

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let mut data: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON file: {:?}", file_path))?;

    // Add/replace edit_history
    let events_data = serde_json::to_value(&history.events)?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("edit_history".to_string(), events_data);
    }

    let new_content = format!("{}\n", serde_json::to_string_pretty(&data)?);

    if dry_run {
        return Ok((true, new_content));
    }

    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;
    Ok((true, format!("Updated: {:?}", file_path)))
}

pub fn write_sidecar(
    file_path: &Path,
    history: &FileHistory,
    sidecar_pattern: &str,
    dry_run: bool,
) -> Result<(bool, String)> {
    let sidecar_path = resolve_sidecar_path(file_path, sidecar_pattern);

    // Merge with existing sidecar if it exists
    let mut existing_events: Vec<CurationEvent> = Vec::new();
    if sidecar_path.exists() {
        let existing_content = fs::read_to_string(&sidecar_path)?;
        if let Ok(existing_data) = serde_yaml::from_str::<serde_yaml::Value>(&existing_content) {
            if let Some(edit_history) = existing_data.get("edit_history") {
                if let Ok(events) =
                    serde_yaml::from_value::<Vec<CurationEvent>>(edit_history.clone())
                {
                    existing_events = events;
                }
            }
        }
    }

    // Merge and deduplicate by timestamp
    let mut all_events = existing_events;
    all_events.extend(history.events.clone());

    let mut seen_timestamps = std::collections::HashSet::new();
    let mut merged_events = Vec::new();
    for event in all_events {
        let ts = event.timestamp.to_rfc3339();
        if !seen_timestamps.contains(&ts) {
            seen_timestamps.insert(ts);
            merged_events.push(event);
        }
    }

    // Sort by timestamp
    merged_events.sort_by_key(|e| e.timestamp);

    // Include source file reference
    let mut sidecar_data = serde_yaml::Mapping::new();
    sidecar_data.insert(
        serde_yaml::Value::String("source_file".to_string()),
        serde_yaml::Value::String(
            file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
        ),
    );
    sidecar_data.insert(
        serde_yaml::Value::String("edit_history".to_string()),
        serde_yaml::to_value(&merged_events)?,
    );

    let new_content = serde_yaml::to_string(&sidecar_data)?;

    if dry_run {
        return Ok((
            true,
            format!("Would write sidecar: {:?}\n{}", sidecar_path, new_content),
        ));
    }

    // Create parent directories if needed
    if let Some(parent) = sidecar_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&sidecar_path, new_content)?;
    Ok((true, format!("Wrote sidecar: {:?}", sidecar_path)))
}

pub fn write_comment(
    file_path: &Path,
    history: &FileHistory,
    syntax: &CommentSyntax,
    dry_run: bool,
) -> Result<(bool, String)> {
    if !file_path.exists() {
        return Ok((false, format!("File not found: {:?}", file_path)));
    }

    let mut content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    // Generate the curation history YAML (without the outer key)
    let history_yaml = serde_yaml::to_string(&history.events)?;

    // Format as comment block based on syntax
    let comment_block = match syntax {
        CommentSyntax::Hash => {
            let commented: Vec<String> = history_yaml
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        "#".to_string()
                    } else {
                        format!("# {}", line)
                    }
                })
                .collect();
            format!(
                "# --- edit_history ---\n{}\n# --- end edit_history ---\n",
                commented.join("\n")
            )
        }
        CommentSyntax::Slash => {
            let commented: Vec<String> = history_yaml
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        "//".to_string()
                    } else {
                        format!("// {}", line)
                    }
                })
                .collect();
            format!(
                "// --- edit_history ---\n{}\n// --- end edit_history ---\n",
                commented.join("\n")
            )
        }
        CommentSyntax::Html => {
            format!("<!-- edit_history\n{}-->\n", history_yaml)
        }
    };

    // Remove existing edit_history comment block if present
    if content.contains("edit_history") {
        if matches!(syntax, CommentSyntax::Html) {
            // Remove <!-- edit_history ... -->
            static RE: OnceLock<Regex> = OnceLock::new();
            let re = RE.get_or_init(|| {
                Regex::new(r"<!--\s*edit_history.*?-->\n?")
                    .expect("HTML edit_history regex must compile")
            });
            content = re.replace_all(&content, "").to_string();
        } else {
            // Remove marker-based blocks
            let lines: Vec<&str> = content.split('\n').collect();
            let mut new_lines = Vec::new();
            let mut in_block = false;

            for line in lines {
                if line.contains("--- edit_history ---") {
                    in_block = true;
                    continue;
                }
                if line.contains("--- end edit_history ---") {
                    in_block = false;
                    continue;
                }
                if !in_block {
                    new_lines.push(line);
                }
            }
            content = new_lines.join("\n");
        }
    }

    // Ensure content ends with newline
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    let new_content = format!("{}\n{}", content, comment_block);

    if dry_run {
        return Ok((true, new_content));
    }

    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write file: {:?}", file_path))?;
    Ok((true, format!("Updated: {:?}", file_path)))
}

pub fn apply_rule(
    file_path: &Path,
    history: &FileHistory,
    rule: &FileRule,
    dry_run: bool,
) -> Result<(bool, String)> {
    match rule.policy {
        OutputPolicy::Skip => Ok((true, format!("Skipped (policy=skip): {:?}", file_path))),
        OutputPolicy::Append => {
            if dry_run {
                let res = if rule.format == "json" {
                    append_json(file_path, history, true)
                } else {
                    append_yaml(file_path, history, true)
                }?;
                Ok((res.0, format!("Would update: {:?}", file_path)))
            } else if rule.format == "json" {
                append_json(file_path, history, false)
            } else {
                append_yaml(file_path, history, false)
            }
        }
        OutputPolicy::Sidecar => {
            let pattern = rule
                .sidecar_pattern
                .as_deref()
                .unwrap_or("{stem}.history.yaml");
            if dry_run {
                let sidecar_path = resolve_sidecar_path(file_path, pattern);
                Ok((true, format!("Would write sidecar: {:?}", sidecar_path)))
            } else {
                write_sidecar(file_path, history, pattern, false)
            }
        }
        OutputPolicy::Comment => {
            if let Some(ref syntax) = rule.comment_syntax {
                if dry_run {
                    let res = write_comment(file_path, history, syntax, true)?;
                    Ok((res.0, format!("Would update: {:?}", file_path)))
                } else {
                    write_comment(file_path, history, syntax, false)
                }
            } else {
                Ok((
                    false,
                    format!("Comment policy requires comment_syntax for {:?}", file_path),
                ))
            }
        }
    }
}

pub fn preview_update(_file_path: &Path, history: &FileHistory) -> Result<String> {
    generate_curation_yaml(history)
}
