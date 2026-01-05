# CLI Syntax Reference

Complete command syntax and options reference for `ai-blame`.

**Note:** This is a syntax reference. For conceptual guides and workflows, see:

- **[Command Index](index.md)** — Visual overview of all commands
- **[Setup & Configuration](setup.md)** — `init` command deep-dive
- **[Provenance Annotation](annotation.md)** — `report` and `annotate` workflow
- **[Trace Exploration](exploration.md)** — `stats`, `timeline`, and `transcript` commands
- **[Line-Level Analysis](blame-analysis.md)** — `blame` command details
- **[Performance](performance.md)** — Caching and optimization strategies

## Global Options

These options are available for all commands:

| Option | Description |
|--------|-------------|
| `--help` | Show help message and exit |
| `--version` | Show version and exit |

## Commands

### `ai-blame init`

Create a starter `.ai-blame.yaml` config file.

**→ [Full Guide](setup.md)** — Configuration options, flavors, setup workflows

```bash
ai-blame init [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--dir <DIR>` | `-d` | cwd | Directory to write `.ai-blame.yaml` into |
| `--flavor <sidecar\|in-place>` | | `sidecar` | Config template flavor |
| `--force` | | False | Overwrite an existing `.ai-blame.yaml` |

#### Examples

```bash
# Create a starter config (sidecar mode: minimizes edits to existing files)
ai-blame init

# Prefer in-place annotations when possible
ai-blame init --flavor in-place

# Overwrite an existing config
ai-blame init --force
```

!!! warning "File writes"
    `ai-blame init` writes **only** `.ai-blame.yaml`.  
    `ai-blame annotate` will write to project files unless you pass `--dry-run`.

---

### `ai-blame report`

Write a stdout report summarizing curation history (no filesystem changes).

**→ [Full Guide](annotation.md#the-report-command-preview-only)** — Filtering strategies, interpreting output, workflows

```bash
ai-blame report [OPTIONS] [TARGET]
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `TARGET` | Optional. Filter results to files matching this name (e.g., `Asthma.yaml`) |

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--config <CONFIG>` | `-c` | Auto | Config file path (auto-find `.ai-blame.yaml` if present) |
| `--initial-and-recent` | | False | Only keep first and last edit per file |
| `--min-change-size <N>` | `-m` | 0 | Skip intermediate edits smaller than N chars |
| `--show-all` | | False | Show all YAML previews (not just first 5) |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

#### Examples

```bash
# Report for current directory
ai-blame report

# Report with initial+recent filter
ai-blame report --initial-and-recent

# Filter to YAML files only
ai-blame report --pattern ".yaml"

# Process specific file (substring match on path)
ai-blame report config.yaml

# Use explicit trace directory
ai-blame report -t ~/.claude/projects/-Users-me-other-project/

# Use different project directory
ai-blame report --dir /path/to/project

# Use custom config
ai-blame report --config /path/to/.ai-blame.yaml
```

---

### `ai-blame annotate`

Annotate files or write sidecars/comments using output rules (writes by default).

**→ [Full Guide](annotation.md#the-annotate-command-apply)** — Workflows, modes, best practices, common mistakes

```bash
ai-blame annotate [OPTIONS] [TARGET]
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `TARGET` | Optional. Filter results to files matching this name (e.g., `Asthma.yaml`) |

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--config <CONFIG>` | `-c` | Auto | Config file path (auto-find `.ai-blame.yaml` if present) |
| `--dry-run` | | False | Don't write anything; print what would happen |
| `--initial-and-recent` | | False | Only keep first and last edit per file |
| `--min-change-size <N>` | `-m` | 0 | Skip intermediate edits smaller than N chars |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

#### Examples

```bash
# Apply annotations for current directory
ai-blame annotate

# Dry run (no writes)
ai-blame annotate --dry-run --initial-and-recent

# Filter to YAML files only
ai-blame annotate --pattern ".yaml"

# Process specific file (substring match on path)
ai-blame annotate config.yaml

# Use custom config
ai-blame annotate --config /path/to/.ai-blame.yaml
```

---

### `ai-blame stats`

Show statistics about available traces.

**→ [Full Guide](exploration.md#the-stats-command)** — Interpreting statistics, common patterns

```bash
ai-blame stats [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

#### Examples

```bash
# Stats for current directory
ai-blame stats

# Stats for specific project
ai-blame stats --dir /path/to/project

# Stats for YAML files only
ai-blame stats --pattern ".yaml"
```

---

### `ai-blame timeline`

Show a chronological timeline of all file modifications from AI traces.

**→ [Full Guide](exploration.md#the-timeline-command)** — Understanding timeline, filtering strategies

```bash
ai-blame timeline [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

#### Examples

```bash
# Show timeline for current directory
ai-blame timeline

# Timeline for YAML files only
ai-blame timeline --pattern ".yaml"

# Timeline for specific project
ai-blame timeline --dir /path/to/project
```

#### Output

Timeline displays all file modifications in chronological order with:
- Timestamp (UTC)
- Action (CREATED, EDITED, DELETED)
- File path
- Model used
- Session/agent information

---

### `ai-blame blame`

Show git-blame-like line (and optional block) attribution for a file in the current working tree.

**→ [Full Guide](blame-analysis.md#the-blame-command)** — Interpreting output, combining with other commands, edge cases

```bash
ai-blame blame [OPTIONS] <FILE>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `FILE` | File to show blame for (path relative to cwd, or absolute) |

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--lines <N-M>` | | | Restrict output to a line range like `"10-20"` |
| `--blocks` | | False | Show block boundaries (consecutive lines attributed to the same event) |

#### Examples

```bash
ai-blame blame src/main.rs
ai-blame blame src/main.rs --lines 10-40
ai-blame blame src/main.rs --blocks
```

---

### `ai-blame transcript list`

List all transcripts from Claude Code and Codex sessions in the trace directory.

**→ [Full Guide](exploration.md#the-transcript-list-command)** — Column layouts, filtering, understanding session IDs

```bash
ai-blame transcript list [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--limit <N>` | `-n` | 20 | Show N most recent transcripts (0 for all) |
| `--format <table\|json>` | | `table` | Output format |
| `--columns <SPEC>` | | SATM | Column layout (see below) |

#### Column Layout Specification

Use `--columns` to customize which columns appear in the table. Each letter represents a column:

| Letter | Column | Width |
|--------|--------|-------|
| `S` | Session ID | 40 |
| `A` | Agent (claude-code, codex-cli, etc.) | 15 |
| `T` | Start Time | 20 |
| `M` | Message Count | 8 |
| `F` | Files Touched | 8 |
| `O` | Models Used | 30 |
| `L` | Last Message (preview) | 50 |

Example column specs:
- `SATM` (default): Session ID, Agent, Timestamp, Messages
- `SATMO`: Add Models column
- `SATMOL`: Add both Models and Last Message preview
- `ATOM`: Agent, Timestamp, Messages, Models (skip Session ID)

#### Examples

```bash
# List 20 most recent transcripts (default)
ai-blame transcript list

# Show all transcripts with models column
ai-blame transcript list -n 0 --columns SATMO

# Show last message preview for each transcript
ai-blame transcript list --columns SATMOL

# List as JSON (all fields included)
ai-blame transcript list --format json

# Show only agent, time, and model
ai-blame transcript list --columns ATO
```

#### Understanding Session ID Duplication

You may notice the same Session ID appearing multiple times in the list. This is normal and occurs in these scenarios:

1. **Multi-agent sessions**: Claude Code may spawn subagents (Task, Explore, Plan agents, etc.) that share the same parent session ID. Each subagent interaction gets its own trace file.
2. **Session spanning multiple files**: Long sessions may be split across multiple trace files, each with the same session ID.
3. **Session file organization**: The system captures relationships between files—each row represents a distinct trace file, not a distinct session.

The `--columns SATMO` layout makes this relationship clearer by showing which models were used in each trace file.

---

### `ai-blame transcript view`

Display a detailed transcript of a Claude Code or Codex session.

**→ [Full Guide](exploration.md#the-transcript-view-command)** — Output formats, workflows, advanced patterns

```bash
ai-blame transcript view [OPTIONS] <SESSION>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `SESSION` | Session ID (substring match), file path, or slug to view |

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory where `.claude/` lives |
| `--format <text\|markdown\|json>` | | `text` | Output format |
| `--full` | | False | Show full content (don't truncate long messages) |
| `--show-thinking` | | False | Display thinking/chain-of-thought blocks |
| `--show-tools` | | False | Display tool use and tool result details |

#### Examples

```bash
# View transcript by session ID (substring match)
ai-blame transcript view 483c7d95

# View transcript by file path
ai-blame transcript view ~/.claude/projects/-Users-alice-project/483c7d95.jsonl

# View with all details visible
ai-blame transcript view 483c7d95 --full --show-thinking --show-tools

# Export as markdown for documentation
ai-blame transcript view 483c7d95 --format markdown > session.md

# Export as JSON for programmatic access
ai-blame transcript view 483c7d95 --format json | jq .
```

#### Output Formats

**Text format** (default): Clean, terminal-friendly display with section separators and controlled truncation.

**Markdown format**: GitHub-flavored markdown suitable for documentation, blogs, or GitHub issues. Includes proper code fencing and formatting.

**JSON format**: Machine-readable output with full transcript structure (all fields, no truncation).

#### Header Information

The view header includes:
- **Session ID**: Unique identifier for the interaction
- **Slug** (if available): Feature flag or task identifier from Claude Code
- **Agent**: Tool used (claude-code, codex-cli, etc.)
- **Version**: Agent version that created the trace
- **Working Directory**: Directory where the agent was running
- **Trace File**: Path to the source trace file
- **Start/End**: Timestamps of the session
- **Messages**: Total message count
- **Files Touched**: Number of files modified

---

## Trace Directory Resolution

The trace directory is determined by (in order of priority):

1. **`--trace-dir`**: Use this exact path
2. **`--dir` + `--home`**: Compute as `$home/.claude/projects/<encoded-dir>/`
3. **Default**: `~/.claude/projects/<encoded-cwd>/`

The `<encoded-dir>` is a filesystem-safe encoding of the absolute path (at minimum, path separators like `/` become `-`, and punctuation like `.` is also normalized to `-` in practice).

**Example:**

| Directory | Encoded |
|-----------|---------|
| `/Users/alice/project` | `-Users-alice-project` |
| `/home/bob/work/repo` | `-home-bob-work-repo` |

---

### `ai-blame completions`

Generate shell completion scripts for tab completion support.

**→ [Full Guide](../how-to/shell-completions.md)** — Installation instructions for all shells

```bash
ai-blame completions <SHELL>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `SHELL` | Shell to generate completions for: `bash`, `zsh`, `fish`, `elvish`, `powershell` |

#### Examples

```bash
# Generate zsh completions (for Oh My Zsh)
ai-blame completions zsh > ~/.oh-my-zsh/completions/_ai-blame

# Generate bash completions
ai-blame completions bash > /etc/bash_completion.d/ai-blame

# Generate fish completions
ai-blame completions fish > ~/.config/fish/completions/ai-blame.fish
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (e.g., trace directory not found, file not found) |
| 2 | CLI usage error (e.g., invalid `--lines` range) |


