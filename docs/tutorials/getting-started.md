# Getting Started

This tutorial walks you through using `ai-blame` to extract curation history from Claude Code traces and add it to your files.

## Prerequisites

- Python 3.10+
- [uv](https://docs.astral.sh/uv/) package manager (recommended)
- Claude Code installed and used on a project

## Installation

```bash
# Install from PyPI
uv pip install ai-blame

# Or install from source
git clone https://github.com/ai4curation/ai-blame
cd ai-blame
uv sync
```

## Step 1: Check Your Traces

First, navigate to a project where you've used Claude Code:

```bash
cd /path/to/your/project
```

Then check what traces are available:

```bash
ai-blame stats
```

You should see output like:

```
Trace directory: /Users/you/.claude/projects/-Users-you-path-to-your-project
Trace files: 12
  Session traces: 8
  Agent traces: 4

Files with edits (all files): 23
Total successful edits: 47
```

!!! info "Where are traces stored?"
    Claude Code stores execution traces in `~/.claude/projects/`. Each project has its own directory, named by encoding the project path (replacing `/` with `-`).

## Step 2: Preview Changes (Dry Run)

By default, `ai-blame` runs in dry-run mode, showing what would happen without modifying files:

```bash
ai-blame mine
```

This displays:

1. A summary table of all files with edit history
2. YAML previews of the `edit_history` that would be added

```
=== Summary ===
File                                               | Edits | First Edit           | Last Edit
--------------------------------------------------|-------|----------------------|--------------------
config.yaml                                        |     3 | 2025-12-01 08:03     | 2025-12-15 20:34
data/entities.json                                 |     5 | 2025-12-02 14:22     | 2025-12-14 11:15

=== YAML Preview: config.yaml ===
edit_history:
- timestamp: '2025-12-01T08:03:42+00:00'
  model: claude-opus-4-5-20251101
  agent_tool: claude-code
  agent_version: '2.0.75'
  action: CREATED
...

[DRY RUN] No files modified. Use --apply to actually update files.
```

## Step 3: Filter the Results

Often you'll want to reduce the verbosity. The `--initial-and-recent` flag keeps only the first and last edit per file:

```bash
ai-blame mine --initial-and-recent
```

You can also filter by file pattern:

```bash
# Only YAML files
ai-blame mine --pattern ".yaml"

# Only files in a specific directory
ai-blame mine --pattern "kb/"

# A specific file
ai-blame mine config.yaml
```

## Step 4: Apply Changes

Once you're happy with the preview, apply the changes:

```bash
ai-blame mine --apply --initial-and-recent
```

Output:

```
Scanning traces in: /Users/you/.claude/projects/-Users-you-path-to-project
Using config: .ai-blame.yaml

=== Summary ===
...

Applying changes...
  Updated: config.yaml
  Updated: data/entities.json
```

## Step 5: Verify the Results

Open one of the modified files. You should see a new `edit_history` section at the end:

```yaml
# config.yaml
name: my-project
version: 1.0.0

edit_history:
  - timestamp: "2025-12-01T08:03:42+00:00"
    model: claude-opus-4-5-20251101
    agent_tool: claude-code
    agent_version: "2.0.75"
    action: CREATED
  - timestamp: "2025-12-15T20:34:29+00:00"
    model: claude-opus-4-5-20251101
    agent_tool: claude-code
    agent_version: "2.1.0"
    action: EDITED
```

## Next Steps

- Learn how to [configure output policies](../how-to/configuration.md) for different file types
- Understand the [CLI reference](../reference/cli.md) for all available options
- Read about [how the extraction works](../explanation/how-it-works.md)
