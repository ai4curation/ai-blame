<p align="center">
  <img src="docs/assets/ai-blame-logo.png" alt="ai-blame logo" width="200">
</p>

# ai-blame

**Extract provenance from AI agent execution traces.**

Like `git blame`, but for AI-assisted edits.

[![PyPI](https://img.shields.io/pypi/v/ai-blame)](https://pypi.org/project/ai-blame/)
[![Documentation](https://img.shields.io/badge/docs-ai4curation.github.io-blue)](https://ai4curation.github.io/ai-blame)
[![License](https://img.shields.io/badge/license-BSD--3--Clause-green)](LICENSE)

## Why?

AI coding assistants modify your files, but `git blame` only shows who *committed* the changes—not which AI model actually wrote them. `ai-blame` fills this gap by extracting provenance from execution traces and embedding it in your files.

## Features

- **Automatic trace discovery** — Finds Claude Code traces based on your project directory
- **Multiple output modes** — Append to files, create sidecars, or embed as comments
- **Configurable per file type** — Different policies for YAML, JSON, Python, etc.
- **Dry-run by default** — Preview changes before applying
- **Flexible filtering** — By file pattern, change size, or time range

## Installation

```bash
pip install ai-blame

# Or with uv
uv pip install ai-blame
```

## Quick Start

```bash
# Check what traces are available
ai-blame stats

# Preview what would be added (dry run)
ai-blame mine --initial-and-recent

# Apply changes
ai-blame mine --apply --initial-and-recent

# Filter to specific files
ai-blame mine --pattern ".py" --apply
```

## Output Examples

### YAML/JSON files — Append directly

```yaml
# config.yaml
name: my-project
version: 1.0

edit_history:
  - timestamp: "2025-12-01T08:03:42+00:00"
    model: claude-opus-4-5-20251101
    agent_tool: claude-code
    action: CREATED
```

### Code files — Sidecar or comments

```python
# main.py (with comment policy)

def hello():
    print("Hello, world!")

# --- edit_history ---
# - timestamp: '2025-12-01T08:03:42+00:00'
#   model: claude-opus-4-5-20251101
#   action: CREATED
# --- end edit_history ---
```

Or use sidecar files: `main.py` → `main.history.yaml`

## Configuration

Create `.ai-blame.yaml` in your project root:

```yaml
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.json"
    policy: append
    format: json
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash
  - pattern: "tests/**"
    policy: skip
```

## Supported Agents

| Agent | Status |
|-------|--------|
| Claude Code | ✅ Supported |
| OpenAI Codex | 🔜 Planned |
| Others | [PRs welcome!](CONTRIBUTING.md) |

## Documentation

Full documentation: **[ai4curation.github.io/ai-blame](https://ai4curation.github.io/ai-blame)**

- [Getting Started](https://ai4curation.github.io/ai-blame/tutorials/getting-started/)
- [Configuration Guide](https://ai4curation.github.io/ai-blame/how-to/configuration/)
- [CLI Reference](https://ai4curation.github.io/ai-blame/reference/cli/)

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md).

PRs especially welcome for additional agent support (Cursor, Aider, Copilot, etc.).

## License

BSD-3-Clause
