# Provenance Annotation

Extract provenance from AI traces and embed it in your project files.

---

## Conceptual Model

**Provenance annotation** answers the question: "Which AI model/agent made this change, and when?"

The workflow has two phases:

1. **Preview** (`report`) — See what would be added without modifying files
2. **Apply** (`annotate`) — Actually write provenance to files (with `--dry-run` option to preview first)

The two-step pattern lets you:
- Review changes before committing
- Understand filtering and output options
- Dry-run with `--dry-run` flag before final application

---

## The `report` Command (Preview Only)

### What It Does

Shows a stdout report summarizing what edits would be written to each file. **Does not modify files.**

Perfect for exploring what provenance exists without making changes.

### Command Syntax

```bash
ai-blame report [OPTIONS] [TARGET]
```

### Basic Usage

**Show all edits for all files:**

```bash
ai-blame report
```

**Focus on specific file:**

```bash
ai-blame report config.yaml
```

Substring match on filename. Example: `report Asthma` finds `disease_definitions/Asthma.yaml`.

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--config <CONFIG>` | `-c` | Auto | Config file path (auto-find `.ai-blame.yaml`) |
| `--initial-and-recent` | | False | Only keep first and last edit per file |
| `--min-change-size <N>` | `-m` | 0 | Skip edits smaller than N characters |
| `--show-all` | | False | Show all YAML previews (not just first 5) |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

### Example: Report Output

```
$ ai-blame report --initial-and-recent

Processing: disease_definitions.yaml

Edits for disease_definitions.yaml:
  [CREATED] 2025-12-01 08:03:42 UTC
    Model: claude-opus-4-5-20251101
    Agent: claude-code (v2.0.75)
    Session ID: 483c7d95-6b9c-46db-afd4-3ecb6257781a
    Files touched: 5

  [EDITED] 2025-12-15 20:34:29 UTC
    Model: claude-opus-4-5-20251101
    Agent: claude-code (v2.1.0)
    Session ID: 483c7d95-6b9c-46db-afd4-3ecb6257781a
    Files touched: 3

Total edits: 2 (filtered from 8)
```

---

## Common Filtering Strategies

### Strategy 1: Keep All Edits

```bash
# Default: show everything
ai-blame report

# Good for: Complete audit trail
```

### Strategy 2: Keep Only First and Last

```bash
ai-blame report --initial-and-recent

# Good for: High-level history, reduces clutter
```

### Strategy 3: Skip Small Changes

```bash
ai-blame report --min-change-size 100

# Good for: Ignoring typo fixes, focusing on substantial changes
```

### Strategy 4: Combine Filters

```bash
ai-blame report --initial-and-recent --min-change-size 100

# Good for: Minimal history with only substantial changes
```

### Strategy 5: Focus on File Type

```bash
# Only YAML files
ai-blame report --pattern ".yaml"

# Only documentation
ai-blame report --pattern "docs/"
```

---

## The `annotate` Command (Apply)

### What It Does

Applies provenance to your project files according to your `.ai-blame.yaml` config:

- **Sidecar mode:** Creates `.edit_history.*` files alongside originals
- **In-place mode:** Appends `edit_history` section to YAML/JSON files

**Note:** Always use `--dry-run` first to preview changes.

### Command Syntax

```bash
ai-blame annotate [OPTIONS] [TARGET]
```

### Basic Usage (with Preview)

**Recommended pattern:**

```bash
# Step 1: Preview
ai-blame annotate --dry-run

# Step 2: Review output
# (verify it looks correct)

# Step 3: Apply
ai-blame annotate
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--config <CONFIG>` | `-c` | Auto | Config file path |
| `--dry-run` | | False | Don't write; show what would happen |
| `--initial-and-recent` | | False | Only keep first and last edit |
| `--min-change-size <N>` | `-m` | 0 | Skip edits smaller than N characters |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by pattern |

### Example: Dry-Run Output

```bash
$ ai-blame annotate --dry-run

[DRY RUN] Would create: disease_definitions.edit_history.yaml
[DRY RUN] Would create: phenotypes.edit_history.yaml
[DRY RUN] Would append to: config.yaml

Total: 3 files modified (dry-run only)
```

### Example: Actual Output

```bash
$ ai-blame annotate

✓ Created: disease_definitions.edit_history.yaml
✓ Created: phenotypes.edit_history.yaml
✓ Appended to: config.yaml

Total: 3 files modified
```

---

## Common Workflows

### Workflow 1: First-Time Annotation

You just ran `ai-blame init` and want to annotate your project:

```bash
# Step 1: Verify config
cat .ai-blame.yaml

# Step 2: Preview all changes
ai-blame report

# Step 3: Dry-run annotation
ai-blame annotate --dry-run

# Step 4: Review output carefully

# Step 5: Apply
ai-blame annotate

# Step 6: Verify files were created/modified
ls -la
git status
```

### Workflow 2: Selective Annotation (YAML Only)

You only want to annotate YAML files:

```bash
# Preview YAML annotations only
ai-blame report --pattern ".yaml"

# Dry-run for YAML
ai-blame annotate --pattern ".yaml" --dry-run

# Apply to YAML
ai-blame annotate --pattern ".yaml"
```

### Workflow 3: Minimal History (First & Last Only)

You want only the creation and most recent edit:

```bash
# Preview minimal history
ai-blame report --initial-and-recent

# Dry-run
ai-blame annotate --initial-and-recent --dry-run

# Apply
ai-blame annotate --initial-and-recent
```

### Workflow 4: Incremental Updates

You've already annotated once and ran more sessions:

```bash
# See new/updated edits
ai-blame report

# Preview changes only
ai-blame annotate --dry-run

# Apply updates
ai-blame annotate
```

---

## Understanding Output Modes

### Sidecar Mode

Configuration:
```yaml
output:
  mode: sidecar
```

**What happens:**

For each file `document.yaml`:

- Original stays: `document.yaml` (unchanged)
- History added: `document.edit_history.yaml` (NEW)

**Content of `document.edit_history.yaml`:**

```yaml
# Provenance history for document.yaml
edits:
  - timestamp: 2025-12-01 08:03:42 UTC
    action: CREATED
    model: claude-opus-4-5-20251101
    agent: claude-code (v2.0.75)
    session_id: 483c7d95-6b9c-46db-afd4-3ecb6257781a

  - timestamp: 2025-12-15 20:34:29 UTC
    action: EDITED
    model: claude-opus-4-5-20251101
    agent: claude-code (v2.1.0)
    session_id: 483c7d95-6b9c-46db-afd4-3ecb6257781a
```

**Pros:**
- Original files untouched
- Works with any file type
- Easy to review separately

**Cons:**
- Creates extra files
- Requires custom viewer to see history alongside data

---

### In-Place Mode

Configuration:
```yaml
output:
  mode: in-place
```

**What happens:**

For YAML/JSON files, appends `edit_history` section directly.

**Before:**
```yaml
name: Disease Definitions
version: 1.0.0
definitions:
  - name: Asthma
    code: J45.9
```

**After:**
```yaml
name: Disease Definitions
version: 1.0.0
definitions:
  - name: Asthma
    code: J45.9

edit_history:
  - timestamp: 2025-12-01 08:03:42 UTC
    action: CREATED
    model: claude-opus-4-5-20251101
    agent: claude-code (v2.0.75)

  - timestamp: 2025-12-15 20:34:29 UTC
    action: EDITED
    model: claude-opus-4-5-20251101
    agent: claude-code (v2.1.0)
```

**Pros:**
- All info in one file
- Self-documenting
- No extra files

**Cons:**
- Modifies data files
- Only works for YAML/JSON
- History is part of data

---

## Edge Cases & Tips

### Case 1: File Not Found

If `ai-blame report file.yaml` finds nothing:

```bash
# Verify file exists
ls file.yaml

# Check if traces mention this file
ai-blame stats

# Try with full pattern
ai-blame report --pattern "file.yaml"
```

### Case 2: No Edits for File

If a file doesn't appear in report output:

- No traces mention this file, OR
- Traces reference file but didn't modify it (e.g., only read)

This is normal—files are only annotated if there are edits.

### Case 3: Dry-Run Shows Nothing but report Shows Edits

```bash
# This shouldn't happen, but if it does:
# Check that .ai-blame.yaml exists and is readable

cat .ai-blame.yaml

# Verify patterns include your files
ai-blame report --pattern ".yaml"

# Force rebuild cache
ai-blame report --rebuild-cache
```

### Case 4: Want to Re-Annotate

If you run `annotate` twice:

```bash
# First run
ai-blame annotate

# Second run (re-runs same annotations)
ai-blame annotate
```

Behavior depends on mode:

- **Sidecar mode:** Overwrites existing `.edit_history.*` files
- **In-place mode:** Appends again (may create duplicates)

To avoid duplicates in in-place mode, don't re-run on the same data.

---

## Common Mistakes

### ❌ Mistake 1: Running `annotate` without `--dry-run` first

**Problem:** Unexpected changes to files

**Solution:**
```bash
# Always preview first
ai-blame annotate --dry-run

# Review output

# Then apply
ai-blame annotate
```

### ❌ Mistake 2: Forgetting to configure `.ai-blame.yaml`

**Problem:** Wrong files annotated, or files skipped

**Solution:**
```bash
# After init, review and edit config
cat .ai-blame.yaml

# Verify patterns match your project
ai-blame report
```

### ❌ Mistake 3: Annotating before traces are discovered

**Problem:** Empty reports, nothing annotated

**Solution:**
```bash
# First verify traces exist
ai-blame stats

# Then annotate
ai-blame report
```

### ❌ Mistake 4: Using wrong trace directory

**Problem:** Reports show no edits or wrong edits

**Solution:**
```bash
# Specify trace directory explicitly
ai-blame report --trace-dir ~/.claude/projects/-Users-alice-project/

# Or verify default location
ls ~/.claude/projects/
```

---

## Integration with Git

### Commit Provenance Files

After annotation, commit the new files:

```bash
git add .ai-blame.yaml
git add *.edit_history.*  # sidecar mode
git add .                # or in-place mode (modifies files)
git commit -m "Add provenance annotation"
git push
```

### Review in PR

If adding provenance in a PR:

```bash
# Preview changes
ai-blame annotate --dry-run

# Commit
ai-blame annotate
git add .
git commit

# Push PR
```

Reviewers can see exactly what was added.

---

## Performance Notes

- `report` is fast—just reads metadata, doesn't process full traces
- `annotate --dry-run` is fast—no file writes
- `annotate` (actual application) is fast—trace parsing is cached

For large projects with many traces:

```bash
# Speed up by using --min-change-size to filter
ai-blame report --min-change-size 100

# Or focus on specific files
ai-blame report --pattern "docs/"
```

---

## Related Topics

- **[Setup & Configuration](setup.md)** — Creating and configuring `.ai-blame.yaml`
- **[Trace Exploration](exploration.md)** — Understanding what traces are available
- **[Configuration Guide](../how-to/configuration.md)** — Deep config option reference
- **[Performance](performance.md)** — Optimizing annotation runs
- **[CLI Reference: report](cli.md#ai-blame-report)** — Complete syntax
- **[CLI Reference: annotate](cli.md#ai-blame-annotate)** — Complete syntax

---

## Next Steps

After annotation, you can:

1. **Review what was added:** `git diff` or view `.edit_history.*` files
2. **Commit:** `git add` and `git commit`
3. **Explore in detail:** [Trace Exploration](exploration.md) for specific session details
4. **Analyze blame:** [Line-Level Analysis](blame-analysis.md) for per-line attribution

Start with [Trace Exploration](exploration.md) to understand what traces are available before annotating.
