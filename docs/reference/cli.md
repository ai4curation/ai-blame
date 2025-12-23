# CLI Reference

Complete reference for all `ai-blame` commands and options.

## Global Options

These options are available for all commands:

| Option | Description |
|--------|-------------|
| `--help` | Show help message and exit |

## Commands

### `ai-blame mine`

Mine Claude Code traces and add edit history to files.

```bash
ai-blame mine [OPTIONS] [TARGET]
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `TARGET` | Optional. Filter results to files matching this name (e.g., `Asthma.yaml`) |

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir` | `-t` | Auto | Claude trace directory (overrides `--dir` and `--home`) |
| `--dir` | `-d` | cwd | Target project directory. Traces looked up in `$home/.claude/projects/<encoded-dir>/` |
| `--home` | | `~` | Home directory where `.claude/` lives |
| `--config` | `-c` | Auto | Config file path (default: auto-find `.ai-blame.yaml`) |
| `--apply` | `-a` | False | Actually apply changes (default is dry-run) |
| `--initial-and-recent` | `-ir` | False | Only keep first and last edit per file |
| `--min-change-size` | `-m` | 0 | Skip edits smaller than N characters |
| `--pattern` | `-p` | | Filter files by path pattern |
| `--show-all` | | False | Show all YAML previews (not just first 5) |

#### Examples

```bash
# Dry run for current directory
ai-blame mine

# Preview with initial+recent filter
ai-blame mine --initial-and-recent

# Apply changes
ai-blame mine --apply --initial-and-recent

# Filter to YAML files only
ai-blame mine --pattern ".yaml"

# Process specific file
ai-blame mine config.yaml

# Use explicit trace directory
ai-blame mine -t ~/.claude/projects/-Users-me-other-project/

# Use different project directory
ai-blame mine --dir /path/to/project

# Use test data
ai-blame mine --dir tests/data --home tests/data

# Use custom config
ai-blame mine --config /path/to/.ai-blame.yaml
```

---

### `ai-blame stats`

Show statistics about available traces.

```bash
ai-blame stats [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir` | `-t` | Auto | Claude trace directory |
| `--dir` | `-d` | cwd | Target project directory |
| `--home` | | `~` | Home directory where `.claude/` lives |
| `--pattern` | `-p` | | Filter files by path pattern |

#### Examples

```bash
# Stats for current directory
ai-blame stats

# Stats for specific project
ai-blame stats --dir /path/to/project

# Stats for YAML files only
ai-blame stats --pattern ".yaml"

# Stats from test data
ai-blame stats --dir tests/data --home tests/data
```

#### Output

```
Trace directory: /Users/you/.claude/projects/-Users-you-my-project
Trace files: 12
  Session traces: 8
  Agent traces: 4

Files with edits (all files): 23
Total successful edits: 47
```

---

## Trace Directory Resolution

The trace directory is determined by (in order of priority):

1. **`--trace-dir`**: Use this exact path
2. **`--dir` + `--home`**: Compute as `$home/.claude/projects/<encoded-dir>/`
3. **Default**: `~/.claude/projects/<encoded-cwd>/`

The `<encoded-dir>` is the directory path with `/` replaced by `-`.

**Example:**

| Directory | Encoded |
|-----------|---------|
| `/Users/alice/project` | `-Users-alice-project` |
| `/home/bob/work/repo` | `-home-bob-work-repo` |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (e.g., trace directory not found, file not found) |

---

## Environment

`ai-blame` uses:

- Current working directory for default trace lookup
- User's home directory (`~`) for default `.claude/` location
- `.ai-blame.yaml` configuration if present (searched upward from cwd)
