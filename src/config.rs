use crate::models::*;
use anyhow::bail;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const CONFIG_FILENAME: &str = ".ai-blame.yaml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedFlavor {
    /// Write provenance to sidecar files by default (minimizes edits to existing files).
    Sidecar,
    /// Prefer in-place annotation when possible (append for YAML/JSON, comments for common code/doc types).
    InPlace,
}

/// Get the default configuration for ai-blame output.
///
/// Returns a configuration that uses sidecar files for most files, with special handling
/// for YAML/JSON files that get appended to directly.
///
/// # Returns
///
/// An `OutputConfig` with sensible defaults for most projects
///
/// # Examples
///
/// ```rust
/// use ai_blame::config::get_default_config;
///
/// let config = get_default_config();
/// // Default uses sidecar files with pattern "{stem}.history.yaml"
/// assert_eq!(config.defaults.unwrap().sidecar_pattern, Some("{stem}.history.yaml".to_string()));
/// ```
pub fn get_default_config() -> OutputConfig {
    OutputConfig {
        defaults: Some(FileRule {
            pattern: "*".to_string(),
            policy: OutputPolicy::Sidecar,
            format: "yaml".to_string(),
            comment_syntax: None,
            sidecar_pattern: Some("{stem}.history.yaml".to_string()),
        }),
        rules: vec![
            FileRule {
                pattern: "*.yaml".to_string(),
                policy: OutputPolicy::Append,
                format: "yaml".to_string(),
                comment_syntax: None,
                sidecar_pattern: None,
            },
            FileRule {
                pattern: "*.yml".to_string(),
                policy: OutputPolicy::Append,
                format: "yaml".to_string(),
                comment_syntax: None,
                sidecar_pattern: None,
            },
            FileRule {
                pattern: "*.json".to_string(),
                policy: OutputPolicy::Append,
                format: "json".to_string(),
                comment_syntax: None,
                sidecar_pattern: None,
            },
        ],
    }
}

pub fn seed_config_contents(flavor: SeedFlavor) -> String {
    // Note: we render a human-friendly YAML template (with comments) rather than
    // serializing `OutputConfig`, so the generated file is easy to read and edit.
    match flavor {
        SeedFlavor::Sidecar => r#"# ai-blame configuration
# This file controls how `ai-blame annotate` writes provenance.
#
# Safety note:
# - `ai-blame init` writes ONLY this config file.
# - `ai-blame annotate` WILL write to files unless you pass `--dry-run`.

defaults:
  # Sidecar mode: keep your source files untouched by writing companion history files.
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  # Example: don't touch test outputs/fixtures
  - pattern: "tests/**"
    policy: skip
"#
        .to_string(),
        SeedFlavor::InPlace => r#"# ai-blame configuration
# This file controls how `ai-blame annotate` writes provenance.
#
# Safety note:
# - `ai-blame init` writes ONLY this config file.
# - `ai-blame annotate` WILL write to files unless you pass `--dry-run`.
#
# In-place flavor:
# - YAML/JSON: add an `edit_history` key directly to the file
# - Common code/docs: append a comment block at end of file
# - Everything else: fall back to sidecar (to avoid corrupting unknown formats)

defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  # Structured data formats: write in-place as `edit_history`.
  - pattern: "*.yaml"
    policy: append
    format: yaml
  - pattern: "*.yml"
    policy: append
    format: yaml
  - pattern: "*.json"
    policy: append
    format: json

  # Code/docs: append provenance as a comment block.
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash
  - pattern: "*.sh"
    policy: comment
    comment_syntax: hash
  - pattern: "*.toml"
    policy: comment
    comment_syntax: hash
  - pattern: "*.md"
    policy: comment
    comment_syntax: html
  - pattern: "*.rs"
    policy: comment
    comment_syntax: slash
  - pattern: "*.js"
    policy: comment
    comment_syntax: slash
  - pattern: "*.jsx"
    policy: comment
    comment_syntax: slash
  - pattern: "*.ts"
    policy: comment
    comment_syntax: slash
  - pattern: "*.tsx"
    policy: comment
    comment_syntax: slash

  # Example: don't touch test outputs/fixtures
  - pattern: "tests/**"
    policy: skip
"#
        .to_string(),
    }
}

pub fn write_seed_config(dir: &Path, flavor: SeedFlavor, force: bool) -> Result<PathBuf> {
    let config_path = dir.join(CONFIG_FILENAME);
    if config_path.exists() && !force {
        bail!(
            "Config already exists at {:?} (use --force to overwrite)",
            config_path
        );
    }
    std::fs::write(&config_path, seed_config_contents(flavor))
        .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
    Ok(config_path)
}

/// Find the ai-blame configuration file by searching up the directory tree.
///
/// Searches for `.ai-blame.yaml` starting from `start_dir` (or current directory if None),
/// walking up parent directories until a config file is found or the root is reached.
///
/// # Arguments
///
/// * `start_dir` - Directory to start searching from, or None for current directory
///
/// # Returns
///
/// Path to the config file if found, None otherwise
///
/// # Examples
///
/// ```rust
/// use ai_blame::config::find_config;
/// use std::path::Path;
///
/// // Search from current directory
/// if let Some(config_path) = find_config(None) {
///     println!("Found config at: {:?}", config_path);
/// }
///
/// // Search from specific directory
/// let project_dir = Path::new("/path/to/project");
/// if let Some(config_path) = find_config(Some(project_dir)) {
///     println!("Found config at: {:?}", config_path);
/// }
/// ```
pub fn find_config(start_dir: Option<&Path>) -> Option<PathBuf> {
    let start_dir = start_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut current = start_dir.as_path();

    loop {
        let config_path = current.join(CONFIG_FILENAME);
        if config_path.exists() {
            return Some(config_path);
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

pub fn load_config(path: &Path) -> Result<OutputConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let config: OutputConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {:?}", path))?;

    Ok(config)
}

/// Resolve a sidecar file path using a pattern template.
///
/// Takes a source file path and applies a pattern template to generate the sidecar file path.
/// The pattern supports placeholders like `{stem}` for the filename without extension.
///
/// # Arguments
///
/// * `source_path` - Path to the original file
/// * `pattern` - Template pattern for the sidecar file (e.g., "{stem}.history.yaml")
///
/// # Returns
///
/// Path to the resolved sidecar file
///
/// # Examples
///
/// ```rust
/// use ai_blame::config::resolve_sidecar_path;
/// use std::path::Path;
///
/// let source = Path::new("/project/src/main.rs");
/// let pattern = "{stem}.history.yaml";
/// let sidecar = resolve_sidecar_path(source, pattern);
/// assert_eq!(sidecar, Path::new("/project/src/main.history.yaml"));
///
/// // For files without extension
/// let source = Path::new("/project/README");
/// let sidecar = resolve_sidecar_path(source, pattern);
/// assert_eq!(sidecar, Path::new("/project/README.history.yaml"));
/// ```
pub fn resolve_sidecar_path(source_path: &Path, pattern: &str) -> PathBuf {
    let parent = source_path.parent().unwrap_or_else(|| Path::new(""));
    let name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let result = pattern
        .replace("{dir}", &parent.to_string_lossy())
        .replace("{name}", name)
        .replace("{stem}", stem)
        .replace("{ext}", ext);

    // If pattern starts with a path component, join with parent
    if result.starts_with('.') || !result.contains('/') {
        parent.join(result)
    } else {
        PathBuf::from(result)
    }
}
