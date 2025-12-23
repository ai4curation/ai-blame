# Data Models Reference

Reference documentation for all data models in `ai-blame`.

## Curation Models

### `CurationEvent`

A single curation event in the audit trail.

```python
from ai_blame.models import CurationEvent, CurationAction
from datetime import datetime, timezone

event = CurationEvent(
    timestamp=datetime(2025, 12, 1, 8, 3, 42, tzinfo=timezone.utc),
    model="claude-opus-4-5-20251101",
    action=CurationAction.CREATED,
    agent_tool="claude-code",
    agent_version="2.0.75",
    description="Initial creation",
)
```

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | `datetime` | When the edit occurred |
| `model` | `str \| None` | Model identifier (e.g., `claude-opus-4-5-20251101`) |
| `action` | `CurationAction \| None` | Type of action (`CREATED` or `EDITED`) |
| `agent_tool` | `str \| None` | Tool that made the edit (e.g., `claude-code`) |
| `agent_version` | `str \| None` | Version of the agent tool |
| `description` | `str \| None` | Optional description of the change |

**Methods:**

```python
# Convert to dictionary (for YAML/JSON output)
event.to_dict()
# {'timestamp': '2025-12-01T08:03:42+00:00', 'model': 'claude-opus-4-5', ...}
```

### `CurationAction`

Enum for curation action types.

```python
from ai_blame.models import CurationAction

CurationAction.CREATED  # File was created
CurationAction.EDITED   # File was modified
```

### `FileHistory`

Aggregated edit history for a single file.

```python
from ai_blame.models import FileHistory

history = FileHistory(
    file_path="config.yaml",
    events=[event1, event2, event3],
)

# Properties
history.first_edit  # datetime of first event
history.last_edit   # datetime of last event
```

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `str` | Path to the file |
| `events` | `list[CurationEvent]` | List of curation events |

| Property | Type | Description |
|----------|------|-------------|
| `first_edit` | `datetime \| None` | Timestamp of first event |
| `last_edit` | `datetime \| None` | Timestamp of last event |

---

## Extraction Models

### `EditRecord`

A record of a successful file edit extracted from traces.

```python
from ai_blame.models import EditRecord
from datetime import datetime, timezone

record = EditRecord(
    file_path="/path/to/file.yaml",
    timestamp=datetime(2025, 12, 1, tzinfo=timezone.utc),
    model="claude-opus-4-5-20251101",
    session_id="abc123",
    is_create=True,
    change_size=150,
    agent_tool="claude-code",
    agent_version="2.0.75",
)
```

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `str` | Absolute path to the file |
| `timestamp` | `datetime` | When the edit occurred |
| `model` | `str` | Model that made the edit |
| `session_id` | `str` | Claude Code session ID |
| `is_create` | `bool` | True if this was a file creation |
| `change_size` | `int` | Approximate size of change in characters |
| `agent_tool` | `str` | Tool identifier (default: `claude-code`) |
| `agent_version` | `str \| None` | Version of the agent tool |

### `FilterConfig`

Configuration for filtering edit records.

```python
from ai_blame.models import FilterConfig
from datetime import datetime, timezone

config = FilterConfig(
    initial_and_recent_only=True,
    min_change_size=50,
    since=datetime(2025, 12, 1, tzinfo=timezone.utc),
    until=datetime(2025, 12, 31, tzinfo=timezone.utc),
    file_pattern=".yaml",
)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `initial_and_recent_only` | `bool` | `False` | Keep only first and last edit per file |
| `min_change_size` | `int` | `0` | Minimum change size in characters |
| `since` | `datetime \| None` | `None` | Only include edits after this time |
| `until` | `datetime \| None` | `None` | Only include edits before this time |
| `file_pattern` | `str \| None` | `None` | Filter by path substring |

---

## Output Configuration Models

### `OutputPolicy`

Enum for output policies.

```python
from ai_blame.models import OutputPolicy

OutputPolicy.APPEND   # Add to file directly
OutputPolicy.SIDECAR  # Write companion file
OutputPolicy.COMMENT  # Embed as comment block
OutputPolicy.SKIP     # Don't process
```

### `CommentSyntax`

Enum for comment syntax styles.

```python
from ai_blame.models import CommentSyntax

CommentSyntax.HASH   # # comment
CommentSyntax.SLASH  # // comment
CommentSyntax.HTML   # <!-- comment -->
```

### `FileRule`

Rule for handling files matching a pattern.

```python
from ai_blame.models import FileRule, OutputPolicy, CommentSyntax

# Append to YAML files
rule = FileRule(
    pattern="*.yaml",
    policy=OutputPolicy.APPEND,
)

# Sidecar for Python files
rule = FileRule(
    pattern="*.py",
    policy=OutputPolicy.SIDECAR,
    sidecar_pattern="{stem}.history.yaml",
)

# Comment in JavaScript
rule = FileRule(
    pattern="*.js",
    policy=OutputPolicy.COMMENT,
    comment_syntax=CommentSyntax.SLASH,
)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pattern` | `str` | `"*"` | Glob pattern to match |
| `policy` | `OutputPolicy` | `APPEND` | How to write history |
| `format` | `str` | `yaml` | Output format (`yaml` or `json`) |
| `comment_syntax` | `CommentSyntax \| None` | `None` | Comment style for `COMMENT` policy |
| `sidecar_pattern` | `str \| None` | `None` | Pattern for sidecar filenames |

### `OutputConfig`

Complete output configuration with defaults and rules.

```python
from ai_blame.models import OutputConfig, FileRule, OutputPolicy

config = OutputConfig(
    defaults=FileRule(
        pattern="*",
        policy=OutputPolicy.SIDECAR,
        sidecar_pattern="{stem}.history.yaml",
    ),
    rules=[
        FileRule(pattern="*.yaml", policy=OutputPolicy.APPEND),
        FileRule(pattern="*.json", policy=OutputPolicy.APPEND, format="json"),
    ],
)

# Get rule for a file
rule = config.get_rule_for_file("config.yaml")
print(rule.policy)  # OutputPolicy.APPEND
```

| Field | Type | Description |
|-------|------|-------------|
| `defaults` | `FileRule \| None` | Default rule when no rules match |
| `rules` | `list[FileRule]` | Rules evaluated in order (first match wins) |

**Methods:**

```python
# Get the applicable rule for a file path
config.get_rule_for_file("path/to/file.py") -> FileRule | None
```
