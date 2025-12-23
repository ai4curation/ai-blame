<p align="center">
  <img src="assets/ai-blame-logo.png" alt="ai-blame logo" width="200">
</p>

# ai-blame

**Extract provenance from AI agent execution traces.**

Like `git blame`, but for AI-assisted edits.

---

## What is ai-blame?

As AI agents increasingly assist with knowledge base curation, documentation, and code generation, we need to track **provenance** — which agent/model made what changes, when, and why.

`ai-blame` mines execution traces from AI coding assistants (currently Claude Code) and extracts a structured audit trail of file modifications. This history can be:

- **Appended** directly to YAML/JSON files as a `edit_history` section
- **Written** to sidecar files (e.g., `foo.history.yaml`)
- **Embedded** as comments in code files

## Quick Example

```bash
# See what traces are available
ai-blame stats

# Preview what would be added (dry run)
ai-blame mine --initial-and-recent

# Actually apply changes
ai-blame mine --apply --initial-and-recent
```

This produces output like:

```yaml
# ... existing file content ...

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

## Key Features

- **Automatic trace discovery** — Finds Claude Code traces based on your current directory
- **Configurable output policies** — Append, sidecar, or comment-based history
- **Flexible filtering** — By file pattern, change size, or time range
- **Dry-run by default** — Preview changes before applying
- **Multiple file type support** — YAML, JSON, Python, and more

## Supported Trace Sources

| Agent | Status |
|-------|--------|
| Claude Code | ✅ Supported |
| OpenAI Codex | 🔜 Planned |
| Others (Cursor, Aider, etc.) | PRs welcome! |

## Installation

```bash
# Using uv (recommended)
uv pip install ai-blame

# Or from source
git clone https://github.com/ai4curation/ai-blame
cd ai-blame
uv sync
```

## Next Steps

| | |
|---|---|
| :material-rocket-launch: **[Getting Started](tutorials/getting-started.md)** | New to ai-blame? Start here for a hands-on tutorial. |
| :material-cog: **[Configuration](how-to/configuration.md)** | Learn how to customize output policies for different file types. |
| :material-console: **[CLI Reference](reference/cli.md)** | Complete reference for all commands and options. |
| :material-lightbulb: **[Philosophy](explanation/philosophy.md)** | Understand why provenance tracking matters. |
