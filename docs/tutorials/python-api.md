# Python API

This tutorial shows how to use `ai-blame` programmatically in Python scripts.

## Basic Usage

### Extracting Edit History

```python
from pathlib import Path
from ai_blame.extractor import extract_edit_history, convert_to_file_histories
from ai_blame.models import FilterConfig

# Define the trace directory (or use default)
trace_dir = Path.home() / ".claude" / "projects" / "-Users-you-my-project"

# Extract all edits
config = FilterConfig()
edits_by_file = extract_edit_history(trace_dir, config)

# See what files were edited
for file_path, edits in edits_by_file.items():
    print(f"{file_path}: {len(edits)} edits")
```

### Filtering Results

```python
from datetime import datetime, timezone

# Filter by file pattern
config = FilterConfig(file_pattern=".yaml")

# Filter by time range
config = FilterConfig(
    since=datetime(2025, 12, 1, tzinfo=timezone.utc),
    until=datetime(2025, 12, 15, tzinfo=timezone.utc),
)

# Keep only first and last edit per file
config = FilterConfig(initial_and_recent_only=True)

# Skip small edits
config = FilterConfig(min_change_size=100)
```

### Converting to File Histories

```python
from ai_blame.extractor import convert_to_file_histories, apply_filters

# Apply post-extraction filters
config = FilterConfig(initial_and_recent_only=True)
filtered_edits = apply_filters(edits_by_file, config)

# Convert to FileHistory objects
histories = convert_to_file_histories(filtered_edits)

for path, history in histories.items():
    print(f"\n{path}:")
    for event in history.events:
        print(f"  {event.timestamp}: {event.action} by {event.model}")
```

## Working with Configuration

### Loading Config Files

```python
from ai_blame.config import find_config, load_config, get_default_config

# Auto-find .ai-blame.yaml
config_path = find_config()
if config_path:
    output_config = load_config(config_path)
else:
    output_config = get_default_config()

# Get rule for a specific file
rule = output_config.get_rule_for_file("src/main.py")
print(f"Policy: {rule.policy}")  # e.g., OutputPolicy.SIDECAR
```

### Creating Config Programmatically

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
        FileRule(pattern="*.py", policy=OutputPolicy.COMMENT, comment_syntax="hash"),
    ],
)
```

## Updating Files

### Append to YAML

```python
from ai_blame.updater import append_yaml

success, result = append_yaml(
    file_path=Path("config.yaml"),
    history=history,
    dry_run=False,  # Set to True to preview
)
```

### Write Sidecar Files

```python
from ai_blame.updater import write_sidecar

success, result = write_sidecar(
    file_path=Path("src/main.py"),
    history=history,
    sidecar_pattern="{stem}.history.yaml",
    dry_run=False,
)
# Creates: src/main.history.yaml
```

### Embed as Comments

```python
from ai_blame.updater import write_comment
from ai_blame.models import CommentSyntax

success, result = write_comment(
    file_path=Path("script.py"),
    history=history,
    syntax=CommentSyntax.HASH,
    dry_run=False,
)
```

### Apply Rules Automatically

```python
from ai_blame.updater import apply_rule

# Get the rule for this file type
rule = output_config.get_rule_for_file("data.json")

# Apply it
success, msg = apply_rule(
    file_path=Path("data.json"),
    history=history,
    rule=rule,
    dry_run=False,
)
```

## Complete Example

```python
"""Extract and apply curation history to all YAML files in a project."""

from pathlib import Path
from ai_blame.extractor import (
    extract_edit_history,
    apply_filters,
    convert_to_file_histories,
)
from ai_blame.config import get_default_config
from ai_blame.models import FilterConfig
from ai_blame.updater import apply_rule


def main():
    # Configuration
    trace_dir = Path.home() / ".claude" / "projects" / "-Users-me-my-project"

    # Extract edits
    filter_config = FilterConfig(
        file_pattern=".yaml",
        initial_and_recent_only=True,
    )

    edits = extract_edit_history(trace_dir, filter_config)
    edits = apply_filters(edits, filter_config)
    histories = convert_to_file_histories(edits)

    # Get output config
    output_config = get_default_config()

    # Apply to each file
    for rel_path, history in histories.items():
        file_path = Path.cwd() / rel_path
        if not file_path.exists():
            print(f"Skipping (not found): {rel_path}")
            continue

        rule = output_config.get_rule_for_file(rel_path)
        if rule is None:
            continue

        success, msg = apply_rule(file_path, history, rule, dry_run=False)
        print(msg)


if __name__ == "__main__":
    main()
```

## Data Models Reference

See the [Data Models Reference](../reference/models.md) for complete documentation of all classes.
