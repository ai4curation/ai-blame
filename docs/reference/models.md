# Data Models Reference

Reference documentation for the core data models in the Rust `ai-blame` crate.

## Curation Models

### `CurationAction`

Action type for a curation event.

| Value | Meaning |
|------|---------|
| `CREATED` | File was created (first event came from a create operation) |
| `EDITED` | File was edited |

### `CurationEvent`

A single event in the audit trail.

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | `chrono::DateTime<chrono::Utc>` | When the edit occurred |
| `model` | `Option<String>` | Model identifier |
| `action` | `Option<CurationAction>` | `CREATED` or `EDITED` |
| `description` | `Option<String>` | Optional description |
| `agent_tool` | `Option<String>` | Tool that made the edit (e.g., `claude-code`) |
| `agent_version` | `Option<String>` | Version of the agent tool |

### `FileHistory`

Aggregated edit history for a single file.

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `String` | Path to the file (typically relative) |
| `events` | `Vec<CurationEvent>` | List of events |

## Extraction Models

### `EditRecord`

A record of a successful file edit extracted from traces (internal extraction representation).

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `String` | Absolute file path found in traces |
| `timestamp` | `DateTime<Utc>` | When the operation occurred |
| `model` | `String` | Model identifier |
| `session_id` | `String` | Claude Code session ID |
| `is_create` | `bool` | True if this was a file creation |
| `change_size` | `usize` | Approximate size of change in characters |
| `agent_tool` | `String` | Tool identifier |
| `agent_version` | `Option<String>` | Agent version (if present) |
| `old_string` | `Option<String>` | For edits: exact replaced string (if present) |
| `new_string` | `Option<String>` | For edits: replacement string (if present) |
| `structured_patch` | `Option<String>` | Structured patch data (often unified-diff-like) |
| `create_content` | `Option<String>` | For creates: file content at creation time (if present) |

### `FilterConfig`

Configuration for filtering edit records.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `initial_and_recent_only` | `bool` | `false` | Keep only first and last edit per file |
| `min_change_size` | `usize` | `0` | Minimum change size (chars) |
| `since` | `Option<DateTime<Utc>>` | `None` | Only include edits after this time |
| `until` | `Option<DateTime<Utc>>` | `None` | Only include edits before this time |
| `file_pattern` | `Option<String>` | `None` | Filter by path substring |

## Output Configuration Models

### `OutputPolicy`

| Value | Meaning |
|-------|---------|
| `append` | Add `edit_history` directly to the file (YAML/JSON) |
| `sidecar` | Write a companion history file |
| `comment` | Embed history as comments |
| `skip` | Do not process matching files |

### `CommentSyntax`

| Value | Meaning |
|-------|---------|
| `hash` | `# comment` |
| `slash` | `// comment` |
| `html` | `<!-- comment -->` |

### `FileRule`

| Field | Type | Description |
|-------|------|-------------|
| `pattern` | `String` | Glob pattern |
| `policy` | `OutputPolicy` | Output policy |
| `format` | `String` | Output format (e.g., `yaml` or `json`) |
| `comment_syntax` | `Option<CommentSyntax>` | Comment syntax for `comment` policy |
| `sidecar_pattern` | `Option<String>` | Sidecar filename pattern |

### `OutputConfig`

| Field | Type | Description |
|-------|------|-------------|
| `defaults` | `Option<FileRule>` | Default rule when no rules match |
| `rules` | `Vec<FileRule>` | Rules evaluated in order (first match wins) |



