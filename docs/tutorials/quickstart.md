# Quickstart Guide

Get started with `ai-blame` in 5 minutes. This guide covers the most common tasks.

## Installation

```bash
git clone https://github.com/ai4curation/ai-blame
cd ai-blame
cargo build --release
cargo install --path .
```

## Quick Commands

### 1️⃣ Check Your Traces

```bash
cd /path/to/your/project
ai-blame stats
```

Shows how many traces and edits are available.

### 2️⃣ See What Changed

```bash
# Timeline of all changes
ai-blame timeline

# Line-by-line blame for a file
ai-blame blame src/main.rs

# Group lines by model
ai-blame blame src/main.rs --blocks
```

### 3️⃣ Explore Sessions

```bash
# List all sessions
ai-blame transcript list

# Show models used in each session
ai-blame transcript list --columns SATMO

# View a specific session
ai-blame transcript view <session-id> --full
```

### 4️⃣ Add Provenance to Files

```bash
# Initialize config
ai-blame init

# Preview what will be added
ai-blame report --initial-and-recent

# Apply changes to files
ai-blame annotate --initial-and-recent
```

## Common Patterns

### Filter by File Type

```bash
# Only YAML files
ai-blame report --pattern ".yaml"

# Only files in docs/
ai-blame report --pattern "docs/"
```

### Export for Documentation

```bash
# Export a session as markdown
ai-blame transcript view <session-id> --format markdown > session.md

# See the full conversation
ai-blame transcript view <session-id> --full --show-thinking
```

### View Specific File Changes

```bash
# Blame a specific file
ai-blame blame config.yaml

# Report for a specific file
ai-blame report config.yaml
```

## Understanding Session IDs

You may see the same Session ID appear multiple times in `transcript list`. This is normal—it means Claude Code spawned subagents (Explore, Task, Plan agents) that share the same parent session. Each row is a different trace file.

Use `--columns SATMO` to see which models were used in each trace file. This helps distinguish parent sessions from subagent sessions.

## Next Steps

- **Full tutorial:** [Getting Started](getting-started.md)
- **All commands:** [CLI Reference](../reference/cli.md)
- **Understand sessions:** [Transcripts & Subagents](../explanation/transcripts-and-subagents.md)
- **Configure output:** [Configuration](../how-to/configuration.md)
- **Questions?** [FAQs](../faqs.md)

## Tips & Tricks

| Task | Command |
|------|---------|
| Dry run (preview only) | `ai-blame report` |
| Keep only first/last edit | `--initial-and-recent` |
| Show file blame details | `ai-blame blame <file>` |
| Group blame by model | `ai-blame blame <file> --blocks` |
| List all sessions | `ai-blame transcript list -n 0` |
| Export session | `ai-blame transcript view <id> --format markdown` |
| See blame at line X | `ai-blame blame <file> --lines X-X` |
