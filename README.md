# ai-blame

Extract provenance/audit trails from AI agent execution traces.

Like `git blame`, but for AI-assisted edits.

## Problem Statement

As AI agents increasingly assist with knowledge base curation, we need to track provenance — which agent/model made what changes, when, and why.

See: https://github.com/ai4curation/aidocs/issues/62

## Supported Trace Sources

- **Claude Code** traces (`~/.claude/projects/<encoded-cwd>/`)

## Installation

```bash
uv pip install ai-blame
```

Or from source:
```bash
git clone https://github.com/ai4curation/ai-blame
cd ai-blame
uv sync
```

## Usage

```bash
# Show stats about available traces
ai-blame stats

# Dry run - preview what curation_history would be added
ai-blame mine --initial-and-recent

# Actually apply changes to files
ai-blame mine --apply --initial-and-recent

# Filter to specific file
ai-blame mine Asthma.yaml --initial-and-recent
```

## Output Format

Appends a `curation_history` section to YAML files:

```yaml
# ... existing content ...

curation_history:
  - timestamp: "2025-12-01T08:03:42Z"
    model: claude-opus-4-5-20251101
    action: CREATED
  - timestamp: "2025-12-15T20:34:29Z"
    model: claude-opus-4-5-20251101
    action: EDITED
```

## How It Works

1. Scans Claude Code trace files (JSONL format)
2. Identifies successful `Edit` and `Write` tool operations
3. Extracts metadata: timestamp, model, file path
4. Groups by file and filters (first+last, size thresholds)
5. Appends `curation_history` to affected files

## Trace Directory Detection

If `--trace-dir` is not specified, the tool looks for traces in:
```
~/.claude/projects/<encoded-cwd>/
```

Where `<encoded-cwd>` is your current working directory with `/` replaced by `-`.

For example, `/Users/cjm/repos/dismech` becomes:
```
~/.claude/projects/-Users-cjm-repos-dismech/
```

## Developer Tools

There are several pre-defined command-recipes available.
They are written for the command runner [just](https://github.com/casey/just/). To list all pre-defined commands, run `just` or `just --list`.

## Credits

This project uses the template [monarch-project-copier](https://github.com/monarch-initiative/monarch-project-copier)

## License

BSD-3-Clause
