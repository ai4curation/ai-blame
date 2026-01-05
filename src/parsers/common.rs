/// Shared parsing utilities used by multiple parsers
use serde_json::Value;

/// Extract model from a JSON record
pub fn extract_model_from_record(record: &Value) -> Option<&str> {
    // Common shapes:
    // - {"message":{"model":"..."}}
    // - {"model":"..."}
    // - {"toolUseResult":{"model":"..."}} (rare, but cheap to support)
    record
        .get("message")
        .and_then(|msg| msg.get("model"))
        .and_then(|m| m.as_str())
        .or_else(|| record.get("model").and_then(|m| m.as_str()))
        .or_else(|| {
            record
                .get("toolUseResult")
                .and_then(|tr| tr.get("model"))
                .and_then(|m| m.as_str())
        })
}

/// Check if a model string is a Codex/OpenAI model
pub fn is_codex_model(model: &str) -> bool {
    // Check for known OpenAI Codex and GPT model prefixes used by GitHub Copilot
    model.starts_with("codex-")
        || model.starts_with("gpt-4")
        || model.starts_with("gpt-3.5")
        || model.starts_with("gpt-35")
}

/// Extract tool use IDs from an assistant message
pub fn extract_tool_use_ids_from_record(record: &Value) -> Vec<String> {
    // Assistant messages can contain tool_use blocks like:
    // {"message":{"content":[{"type":"tool_use","id":"toolu_..."}]}}
    let mut out = Vec::new();
    let Some(content) = record
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return out;
    };
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
            continue;
        }
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            out.push(id.to_string());
        }
    }
    out
}

/// Normalize file paths to be relative to repository root
pub fn normalize_path(abs_path: &str, repo_root: Option<&str>) -> String {
    let repo_root = if let Some(r) = repo_root {
        r
    } else {
        // Get current dir as string at call site
        if let Ok(cwd) = std::env::current_dir() {
            if abs_path.starts_with(cwd.to_str().unwrap_or("")) {
                return abs_path[cwd.to_str().unwrap_or("").len()..]
                    .trim_start_matches('/')
                    .to_string();
            }
        }
        return abs_path.to_string();
    };

    if let Some(stripped) = abs_path.strip_prefix(repo_root) {
        stripped.trim_start_matches('/').to_string()
    } else {
        abs_path.to_string()
    }
}
