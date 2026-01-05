# Performance and Caching

This guide explains how to use ai-blame's DuckDB caching system to speed up repeated analysis runs.

## The Problem: Slow Re-runs

The initial `ai-blame stats` run parses hundreds of trace files, which can take 30-60 seconds depending on your trace history size. However, if the traces haven't changed, subsequent runs do the exact same parsing work and take just as long.

**Example:**
```bash
# First run: 53 seconds
ai-blame stats

# Second run (no changes): still 53 seconds ❌
ai-blame stats
```

## The Solution: Trace Caching

ai-blame can cache parsed trace data in a DuckDB database (`.ai-blame.ddb`) stored alongside your traces. On subsequent runs, unchanged traces are loaded from the cache instead of re-parsed.

**With caching:**
```bash
# First run: ~55 seconds (builds cache)
AI_BLAME_ENABLE_CACHE=1 ai-blame stats

# Second run (no changes): ~3-5 seconds ✅
AI_BLAME_ENABLE_CACHE=1 ai-blame stats

# Cache hit: 90% speedup!
```

## Current Status

**✅ Enabled by Default**

Caching is now enabled by default with full support for cross-file UUID resolution in Claude traces. The fix for the duplicate edits bug is implemented in [#36](https://github.com/ai4curation/ai-blame/issues/36) and [#37](https://github.com/ai4curation/ai-blame/issues/37).

## Using the Cache

### Default Behavior (Cache Enabled)

Caching is enabled by default. No configuration needed:

```bash
ai-blame stats
ai-blame report
ai-blame annotate
```

The cache is automatically created and used on subsequent runs.

### Disable Cache for Specific Run

If caching is enabled globally but you want to skip it for one run:

```bash
ai-blame stats --no-cache
```

### Rebuild the Cache

To delete and rebuild the cache (forces re-parsing of all traces):

```bash
ai-blame stats --rebuild-cache
```

Or to rebuild while keeping cache enabled:

```bash
AI_BLAME_ENABLE_CACHE=1 ai-blame stats --rebuild-cache
```

## How Caching Works

### Cache Location

The cache is stored in `.ai-blame.ddb` in your trace directory (first trace directory if multiple are provided):

```
your-project/
├── .claude/
│   └── traces/
│       ├── session-1.jsonl
│       ├── session-2.jsonl
│       └── .ai-blame.ddb        ← Cache file (auto-created)
├── .ai-blame.yaml
└── src/
```

### Staleness Detection

The cache automatically detects when traces have changed:

| Property | Tracked | Used For |
|----------|---------|----------|
| Modification time | ✅ Nanosecond precision | Detect file changes |
| File size | ✅ Bytes | Quick change detection |
| Parser version | ✅ Version 1 schema | Detect schema changes |
| Record count | ✅ Stored count | Statistics |

### Provider-Specific Behavior

#### Claude Traces: All-or-Nothing Invalidation

Claude traces use cross-file UUID resolution to link edits across sessions. Due to this dependency:

- If **ANY** trace file changes, **ALL** files in that directory are re-parsed
- This ensures UUID references remain correct
- Cost: ~1-2 seconds per directory for small trace sets

**Example:**
```bash
# 5 trace files cached
# Edit 1 file → all 5 are re-parsed
# This is correct, but not optimal (see issue #38 for improvements)
```

#### Codex Traces: Per-File Invalidation

Codex/Copilot traces are independent (no cross-file dependencies):

- Each trace file is checked independently
- Only changed files are re-parsed
- Optimal cache behavior ✅

### Cache Schema

The cache stores:
- **trace_files table**: Metadata about each trace file (mtime, size, record count)
- **edit_records table**: Fully parsed edits with all fields (file path, timestamp, model, etc.)
- **Indexes**: On file path, timestamp, and session ID for fast queries

## Cleaning Up

To remove the cache manually:

```bash
# Delete single cache
rm .ai-blame.ddb

# Delete caches in all trace directories
find ~/.claude -name ".ai-blame.ddb" -delete
```

Cache files are automatically added to `.gitignore` and won't be committed to version control.

## Performance Impact

### First Run (Cache Enabled)
- **Expected**: ~1-2 seconds slower than without caching
- **Reason**: Writing to DuckDB has overhead
- **Trade-off**: Worth it for future runs

### Subsequent Runs (Unchanged Traces)
- **Expected**: ~90% speedup
- **Example**: 53 seconds → 3-5 seconds
- **Scaling**: Linear with trace count (faster with more traces)

### Incremental Updates (Some Traces Changed)
- **Expected**: Proportional to changed files
- **Example**: 10% of traces changed → ~10% of re-parsing cost + cache queries
- **Best case**: Single file changed, rest from cache → 5-10 seconds total

## Troubleshooting

### Cache Not Created

**Symptoms:** Run `AI_BLAME_ENABLE_CACHE=1 ai-blame stats` but no `.ai-blame.ddb` file appears.

**Solutions:**
1. Verify caching is enabled: `echo $AI_BLAME_ENABLE_CACHE` (should output `1`)
2. Check trace directory is writable: `touch /path/to/traces/.test`
3. Ensure traces contain edits (empty traces may not create cache entries)

### Cache Corruption

**Symptoms:** `Error: Failed to open cache: Constraint violation` or similar DuckDB errors.

**Solutions:**
```bash
# Rebuild the cache
ai-blame stats --rebuild-cache

# Or manually delete it
rm .ai-blame.ddb
```

## Future Improvements

- **[#38](https://github.com/ai4curation/ai-blame/issues/38)**: Incremental cache invalidation for Claude traces (smart UUID dependency tracking to reduce re-parsing on partial changes)

## See Also

- **[Performance & Optimization](../reference/performance.md)** — Caching and filtering strategies
- [CLI Reference: Cache Flags](../reference/cli.md#cache-options)
- [Configuration File](../reference/config-file.md)
- [FAQ: Performance](../faqs.md#performance)
