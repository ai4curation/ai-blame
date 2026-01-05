# Setup & Configuration

Getting `ai-blame` ready to track provenance in your project.

---

## Conceptual Model

**Setup** is a one-time operation to initialize `ai-blame` for your project. The `init` command creates a configuration file (`.ai-blame.yaml`) that defines:

- Which output policy to use (sidecar, in-place, or comments)
- Which file types to process
- How to handle different formats (YAML, JSON, Python, etc.)

Once configured, you can run other commands (`stats`, `blame`, `report`, `annotate`) without additional setup.

---

## The `init` Command

### What It Does

Creates a starter `.ai-blame.yaml` configuration file in your project directory. This file controls how `ai-blame` processes and annotates your files.

### Basic Usage

**Simplest form (current directory):**

```bash
ai-blame init
```

Creates `.ai-blame.yaml` in your current working directory with default settings (sidecar mode).

### Command Syntax

```bash
ai-blame init [OPTIONS]
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--dir <DIR>` | `-d` | cwd | Directory to write `.ai-blame.yaml` into |
| `--flavor <sidecar\|in-place>` | | `sidecar` | Config template flavor |
| `--force` | | False | Overwrite an existing `.ai-blame.yaml` |

---

## Configuration Flavors

### Sidecar Mode (Default)

Best for: Projects where you want to minimize edits to existing files.

```bash
ai-blame init --flavor sidecar
```

**What it does:** Writes provenance to separate files named `<filename>.edit_history.yaml` for YAML files, `<filename>.edit_history.json` for JSON files, etc.

**Example:**
```
Before: disease_definitions.yaml
After:  disease_definitions.yaml (unchanged)
        disease_definitions.edit_history.yaml (NEW - sidecar)
```

**Pros:**
- Non-invasive (original files untouched)
- Works with any file type
- Easy to review before committing

**Cons:**
- Creates additional files
- Requires separate viewer to see history

### In-Place Mode

Best for: Projects where you prefer history embedded directly in files.

```bash
ai-blame init --flavor in-place
```

**What it does:** Appends `edit_history` section directly to YAML/JSON files.

**Example YAML:**
```yaml
# Original content
name: Disease Definitions
definitions:
  - name: Asthma
    code: J45

# Added by ai-blame
edit_history:
  - timestamp: 2025-12-01 08:03:42 UTC
    model: claude-opus-4-5-20251101
    agent: claude-code (v2.0.75)
    action: CREATED
```

**Pros:**
- All information in one file
- Self-documenting
- No extra files to manage

**Cons:**
- Modifies existing files
- Only works for YAML/JSON
- History embedded in data

---

## Configuration File Structure

Once created, `.ai-blame.yaml` looks like this:

```yaml
# Output policy: how to write provenance
output:
  # Sidecar mode: write to separate files
  mode: sidecar

  # File patterns to process
  patterns:
    - "*.yaml"
    - "*.json"
    - "*.md"

  # Format-specific settings
  formats:
    yaml:
      sidecar_suffix: .edit_history.yaml
    json:
      sidecar_suffix: .edit_history.json
    python:
      sidecar_suffix: .edit_history.txt

# Filtering
filters:
  # Only include edits larger than N characters
  min_change_size: 0

  # Skip intermediate edits (keep only first and last)
  # initial_and_recent: true
```

### Understanding Configuration Options

**`output.mode`:**
- `sidecar` — Write to separate `.edit_history.*` files
- `in-place` — Append to YAML/JSON files directly

**`output.patterns`:**
- List of glob patterns for files to process
- Supports wildcards: `*.yaml`, `docs/**/*.md`

**`output.formats[format].sidecar_suffix`:**
- File extension for sidecar files
- Example: `.edit_history.yaml` means `config.yaml` → `config.edit_history.yaml`

**`filters.min_change_size`:**
- Minimum number of characters changed to include an edit
- Use to skip trivial edits (whitespace, single chars)

---

## Common Setup Scenarios

### Scenario 1: YAML-Only Project

You're tracking a YAML knowledge base (like LinkML schemas):

```bash
# Initialize in sidecar mode (good for YAML)
ai-blame init

# Edit .ai-blame.yaml to remove unwanted patterns
# Keep only: *.yaml
```

Then run:

```bash
ai-blame stats
ai-blame blame schema.yaml
ai-blame report
```

### Scenario 2: Code Project (Python)

You're tracking a Python project with comments:

```bash
# Use in-place mode for code comments
ai-blame init --flavor in-place

# Edit .ai-blame.yaml to include Python patterns
# Add: *.py to patterns
```

Then run:

```bash
ai-blame report
ai-blame annotate --dry-run
```

### Scenario 3: Mixed Documentation (Markdown + YAML)

You have docs in multiple formats:

```bash
# Initialize with defaults
ai-blame init

# Configuration already includes *.md, *.yaml, *.json
# Just verify patterns in .ai-blame.yaml

ai-blame stats
ai-blame timeline
```

### Scenario 4: Different Project, Different Home

```bash
# Initialize in a specific directory
ai-blame init --dir /path/to/other/project

# Use that config when running commands
ai-blame report --dir /path/to/other/project
```

---

## Workflow Setup to First Report

**Step 1: Initialize (2 minutes)**

```bash
cd /path/to/your/project
ai-blame init
```

This creates `.ai-blame.yaml`.

**Step 2: Review (1 minute)**

```bash
cat .ai-blame.yaml
```

Verify the patterns match your project structure. Edit if needed.

**Step 3: Verify Traces Exist (1 minute)**

```bash
ai-blame stats
```

Shows available traces and number of edits found.

**Step 4: Preview Report (2 minutes)**

```bash
ai-blame report
```

Shows what `ai-blame` would add to each file.

**Step 5: (Optional) Create Sidecar Files**

```bash
ai-blame annotate --dry-run
```

Preview the actual changes.

**Step 6: (Optional) Apply**

```bash
ai-blame annotate
```

Actually writes provenance to files.

**Total time: ~10 minutes**

---

## Advanced Topics

### Custom Config Location

By default, `ai-blame` looks for `.ai-blame.yaml` in your project root. To use a different location:

```bash
ai-blame report --config /path/to/config.yaml
ai-blame annotate --config /path/to/config.yaml
```

### Overwriting Existing Config

If `.ai-blame.yaml` already exists:

```bash
# Without --force: skipped (no-op)
ai-blame init

# With --force: overwrite
ai-blame init --force
```

Use `--force` to reset to defaults if your config got corrupted or outdated.

### Why Separate `.ai-blame.yaml`?

The configuration is kept separate from other project files because:

1. **Flexibility** — Change output mode without re-running trace parsing
2. **Version control** — Commit `.ai-blame.yaml` to track your preferences
3. **Sharing** — Other team members can use the same config
4. **Non-invasive** — Init doesn't modify your actual project files

---

## Troubleshooting

### "ai-blame init" did nothing

This is normal behavior—if `.ai-blame.yaml` already exists, `init` skips it to avoid overwrites. Use `--force` to recreate:

```bash
ai-blame init --force
```

### "Error: no traces found"

Setup is fine, but there are no Claude Code/Codex traces in the standard location. Check:

```bash
# Verify trace directory location
ls ~/.claude/projects/

# Or use explicit trace directory
ai-blame stats --trace-dir /explicit/path/to/traces
```

### Config doesn't seem to apply

Verify the path:

```bash
# Check if .ai-blame.yaml exists and is readable
cat .ai-blame.yaml

# Commands use this config automatically (if in project root)
ai-blame report

# Or specify explicitly
ai-blame report --config /path/to/.ai-blame.yaml
```

---

## Related Topics

- **[Provenance Annotation](annotation.md)** — Next step after setup: using `report` and `annotate`
- **[Configuration Guide](../how-to/configuration.md)** — Deep dive into all config options
- **[File Types](../how-to/file-types.md)** — How different file formats are handled
- **[CLI Reference: init](cli.md#ai-blame-init)** — Complete syntax reference

---

## Next Steps

Once `init` is complete, you're ready to:

1. **Explore traces:** `ai-blame stats`, `ai-blame timeline`, `ai-blame transcript list`
2. **Analyze blame:** `ai-blame blame <file>`
3. **Preview & apply:** `ai-blame report` → `ai-blame annotate`

Start with [Trace Exploration](exploration.md) to understand what traces are available.
