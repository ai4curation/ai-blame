# Performance & Optimization

Speed up `ai-blame` command execution with caching and filtering strategies.

---

## Conceptual Model

Performance optimization works on two levels:

1. **Caching** — Avoid re-parsing traces. `ai-blame` uses DuckDB caching per trace directory.
2. **Filtering** — Process only what you need. Use `--pattern` and other filters to reduce scope.

Most commands are already fast. Use these techniques when:

- Processing large projects with many traces
- Running `ai-blame` repeatedly (scripts, CI/CD)
- Exploring different filter combinations
- Debugging cache issues

---

## Understanding Caching

### How Caching Works

`ai-blame` caches parsed trace data in DuckDB format:

**Cache location:** `.ai-blame.ddb` in each trace directory

```
~/.claude/projects/-Users-alice-myproject/
├── 483c7d95.jsonl              (trace file)
├── 7f2d4e91.jsonl              (trace file)
└── .ai-blame.ddb               (cache database)
```

**Cache contents:**
- Parsed messages, tool uses, file operations
- Metadata (timestamps, models, session IDs)
- File edit boundaries (what changed when)

**Cache benefits:**
- First run: Parse full traces (seconds)
- Subsequent runs: Use cache (milliseconds)
- Automatic invalidation: When traces change, cache rebuilds

### Cache Invalidation Strategy

Different trace types use different invalidation approaches:

**Claude Code traces:**
- All-or-nothing invalidation
- If ANY trace file in directory changes, entire cache rebuilds
- Reason: Subagent traces reference each other

**Codex/Copilot traces:**
- Per-file invalidation
- Only changed files are re-parsed
- Unchanged files reuse cached data
- Reason: Traces are independent

---

## Cache Control Options

### Option 1: Default (Use Cache Automatically)

```bash
ai-blame stats
ai-blame report
ai-blame blame src/main.rs
```

Cache is checked automatically:
- If cache exists and is valid: Use it (fast)
- If cache missing or invalid: Rebuild (slower, one-time)

**When to use:** Always, unless troubleshooting.

### Option 2: Rebuild Cache Explicitly

```bash
ai-blame report --rebuild-cache
```

Forces rebuild of `.ai-blame.ddb` even if it exists:

- Slow: Must re-parse all traces
- Useful: Cache corruption, trace updates not detected

### Option 3: Skip Cache

```bash
ai-blame report --no-cache
```

Parses traces without using or saving cache:

- Slowest: Re-parses every time
- Useful: Troubleshooting, testing, one-off commands

**Tradeoff:** Accuracy vs speed for debugging.

---

## Caching in Action

### Scenario 1: First Run (No Cache)

```bash
$ time ai-blame stats

Trace directory: ~/.claude/projects/-Users-alice-myproject/
Trace files: 15
...

real    2.3s        # Slower: building cache
```

### Scenario 2: Subsequent Run (With Cache)

```bash
$ time ai-blame stats

Trace directory: ~/.claude/projects/-Users-alice-myproject/
Trace files: 15
...

real    0.1s        # Fast: using cache
```

### Scenario 3: After Adding New Trace

```bash
# Add new session (new .jsonl file)

$ time ai-blame stats

Trace directory: ~/.claude/projects/-Users-alice-myproject/
Trace files: 16
...

real    2.8s        # Slower: rebuilding cache (all-or-nothing)
```

---

## Filtering Strategies

Filters reduce the scope of processing. Use when:

- Exploring specific file types
- Interested in particular directory
- Want to skip trivial edits
- Need focused analysis

### Strategy 1: Filter by File Pattern

Show only files matching pattern:

```bash
# YAML files only
ai-blame report --pattern ".yaml"

# JSON files
ai-blame report --pattern ".json"

# Python files
ai-blame blame --pattern ".py"

# Specific directory
ai-blame timeline --pattern "docs/"

# Multiple patterns (glob)
ai-blame stats --pattern "src/**/*.rs"
```

**Speed improvement:** Skips non-matching files entirely.

### Strategy 2: Filter by Edit Size

Skip small edits (typo fixes, whitespace):

```bash
# Only edits ≥ 100 characters
ai-blame report --min-change-size 100

# Combine with pattern
ai-blame report --pattern ".yaml" --min-change-size 100
```

**Benefit:** Cleaner output, focus on substantial changes.

### Strategy 3: Filter by Time (First & Last)

Keep only first and last edit per file:

```bash
ai-blame report --initial-and-recent
ai-blame annotate --initial-and-recent
```

**Use case:**
- High-level history
- Reduce noise from intermediate edits
- Focus on creation and final version

### Strategy 4: Combine Multiple Filters

```bash
# YAML files only, min 100 char changes, first/last only
ai-blame report \
  --pattern ".yaml" \
  --min-change-size 100 \
  --initial-and-recent
```

**Result:** Most focused, minimal output.

---

## Common Performance Scenarios

### Scenario 1: Large Project Exploration

Quick overview of a large project:

```bash
# Step 1: Fast stats
time ai-blame stats --pattern "docs/"
# Output: fast, limited to docs/

# Step 2: Focused timeline
time ai-blame timeline --pattern ".yaml"
# Output: only YAML changes

# Total: seconds instead of minutes
```

### Scenario 2: CI/CD Integration

Repeated runs in automation:

```bash
# First run (builds cache)
ai-blame report --pattern "docs/" > report.txt
# Slow (seconds)

# Subsequent runs (uses cache)
ai-blame report --pattern "docs/" > report.txt
# Fast (milliseconds)

# When adding new traces, cache rebuilds automatically
```

### Scenario 3: Code Review Preparation

Before reviewing specific files:

```bash
# Fast: focus on file(s) being reviewed
time ai-blame blame src/config.rs --blocks
# Output: who edited what in config.rs
```

### Scenario 4: Troubleshooting Cache Issues

```bash
# Suspect cache is stale?
ai-blame stats --no-cache
# Slow but fresh

# Compare with cached version
ai-blame stats
# Should match

# If different: rebuild
ai-blame stats --rebuild-cache
```

---

## Optimization Tips

### Tip 1: Use Filters First

```bash
# Bad: Process all 1000 files
ai-blame report

# Good: Process only YAML
ai-blame report --pattern ".yaml"

# Better: YAML in docs/ only
ai-blame report --pattern "docs/**/*.yaml"
```

### Tip 2: Leverage Cache in Scripts

```bash
#!/bin/bash
# Script that runs multiple commands

# First run caches results
ai-blame stats
ai-blame timeline
ai-blame transcript list

# All subsequent commands are fast (cached)
ai-blame blame src/main.rs
ai-blame report
```

### Tip 3: Combine Filters for Focused Analysis

```bash
# Speed and clarity
ai-blame report \
  --pattern ".yaml" \
  --min-change-size 50 \
  --initial-and-recent
```

### Tip 4: Use --dry-run Before Committing

```bash
# Preview is fast (no writes, uses cache)
ai-blame annotate --dry-run --pattern "docs/"

# If satisfied, apply
ai-blame annotate --pattern "docs/"
```

---

## Batch Processing

### Process Multiple Directories

```bash
# Script to process several projects
for project in ~/projects/*; do
  echo "Processing $project..."
  time ai-blame stats --dir "$project"
done

# Cache is per-directory, so each builds once
```

### Export Results

```bash
# Save timeline for later analysis
ai-blame timeline > timeline.txt

# Export transcripts as markdown (one-time)
ai-blame transcript view session-1 --format markdown > s1.md
ai-blame transcript view session-2 --format markdown > s2.md

# Archives are static (no re-parsing)
```

---

## When to Optimize

### When Performance Matters

- **Scripts/CI:** Running repeatedly
- **Large projects:** 100+ traces, 1000+ files
- **Interactive:** Exploring in real-time
- **Slow systems:** Older hardware, network-mounted dirs

### When You Can Ignore Performance

- **One-off commands:** Not performance-sensitive
- **Small projects:** < 10 traces, < 100 files
- **Offline processing:** Run once, use results

---

## Cache Troubleshooting

### Problem: Cache Seems Stale

```bash
# Check if new traces were added
ls ~/.claude/projects/.../

# Verify cache rebuilds
ai-blame stats --rebuild-cache

# Check output matches
ai-blame stats
```

### Problem: Command Slower Than Expected

```bash
# Check if cache exists
ls -la ~/.claude/projects/.../.ai-blame.ddb

# Try with explicit skip to measure
time ai-blame stats --no-cache   # Slow
time ai-blame stats               # Fast (should use cache)

# If no improvement, check filters
ai-blame stats --pattern "docs/"  # Should be faster
```

### Problem: Corrupted Cache

```bash
# Rebuild from scratch
rm ~/.claude/projects/.../.ai-blame.ddb

# Rerun command (will rebuild cache)
ai-blame stats
```

---

## Advanced: Understanding Cache Behavior

### Cache Metadata

Cache stores:

```
.ai-blame.ddb
├── traces (parsed trace data)
├── messages (user/assistant exchanges)
├── tool_uses (file operations)
├── metadata (models, timestamps, session IDs)
└── file_edits (edit boundaries, attribution)
```

### Cache Invalidation Details

**Claude Code (all-or-nothing):**
- Subagent traces reference parent session
- Caching tracks relationships
- Any change → full rebuild

**Codex (per-file):**
- Traces are independent
- Caching tracks per-file timestamps
- Changed file → rebuild only that file

### Manual Cache Management

```bash
# List cache info
ls -lh ~/.claude/projects/.../.ai-blame.ddb

# Check cache age
stat ~/.claude/projects/.../.ai-blame.ddb

# Force clean rebuild
rm ~/.claude/projects/.../.ai-blame.ddb
ai-blame stats  # Rebuilds
```

---

## Related Topics

- **[Trace Exploration](exploration.md)** — Commands that benefit from caching
- **[Provenance Annotation](annotation.md)** — Filtering strategies for annotation
- **[How It Works](../explanation/how-it-works.md)** — Cache architecture details

---

## Summary: Performance Checklist

| Goal | Technique | Speed |
|------|-----------|-------|
| **First run** | Use cache (auto) | Seconds |
| **Subsequent runs** | Cache hit | Milliseconds |
| **Multiple projects** | Per-directory cache | Seconds each |
| **Large projects** | Use `--pattern` | Fast |
| **Focused analysis** | Filter files + size | Fast |
| **Automation** | Cache + scripts | Milliseconds |
| **Troubleshooting** | `--rebuild-cache` | Seconds |
| **Testing** | `--no-cache` | Slow (but fresh) |

---

## Next Steps

After optimizing:

1. **Explore:** Use filters with [Trace Exploration](exploration.md)
2. **Annotate:** Apply filters to [Provenance Annotation](annotation.md)
3. **Script:** Use caching in [automation workflows](../how-to/performance-and-caching.md)
