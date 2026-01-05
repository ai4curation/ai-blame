# Frequently Asked Questions

## Ghost Commits and Codex Attribution

### What is a "ghost commit"?

A **ghost commit** is a special git commit object created by Codex CLI to capture file snapshots during execution. These commits:

- Are stored in your `.git/objects/` directory (the git object database)
- Have the author `Codex Snapshot <snapshot@codex.local>`
- Are **not** attached to any branch or tag (they're not in your commit history)
- Can be inspected with `git show <commit-hash>` or `git cat-file`

**Example**: When Codex CLI starts or finishes an operation, it creates a ghost commit capturing the current state of all tracked files.

### How do I make ghost commits "real"?

Ghost commits are ephemeral reference objects used only for tracking file snapshots. To permanently record Codex changes in your git history:

#### Option 1: Commit the changes to a real branch

```bash
# After Codex CLI finishes, commit the changes normally
git add .
git commit -m "Changes from Codex CLI session"
git push origin main
```

This is the standard approach - just treat Codex output like you would any other generated code.

#### Option 2: Create a branch from a ghost commit

If you want to preserve a specific ghost commit snapshot as a real branch:

```bash
# Find the ghost commit hash from logs or git object database
GHOST_COMMIT="b843e9becf13584978668f53f7a787c093494d06"

# Create a real branch pointing to it
git branch codex-snapshot $GHOST_COMMIT

# Or create a tag
git tag codex-session-001 $GHOST_COMMIT
```

#### Option 3: Just use ai-blame (automatic)

If you've committed changes to git **after** running Codex CLI, `ai-blame` will detect:
- Claude Code edits (stored in traces)
- Codex CLI changes (detected via ghost commits + git diffs)

No special steps needed - just commit normally.

### Why aren't my Codex CLI edits showing up in blame?

There are several reasons:

#### 1. **Changes made with bash commands (not captured in traces)**

If you use Codex CLI to run bash commands that modify files (like `>> foo.md`), these changes are **not recorded** in the trace format:

```bash
# This gets logged in ~/.codex/log/*, but NOT in the traces
codex exec "printf 'some text' >> foo.md"
```

**Root cause**: Bash commands run through Codex don't produce structured edit records - only shell command logs.

**Solutions**:
- Use Codex's structured file editing features (not raw bash `>>`)
- Commit the changes after running bash, then use git to understand what changed
- Use `ai-blame report` with git integration to see uncommitted changes

#### 2. **Changes in working directory (not yet in a ghost commit)**

If you edit files in your working directory **after** Codex's last snapshot:

```bash
# Codex creates ghost commit with version A of foo.md
# Then you edit foo.md manually
# Your edits are in working directory, NOT in the ghost commit
```

**Solution**: Commit the changes to git, then rerun `ai-blame`.

#### 3. **No ghost commit was created**

Ghost commits are created at specific points (usually session start/end). If a session doesn't create a ghost commit, file changes won't be detected.

**Solution**: Run a complete Codex CLI session that includes snapshots, or manually commit changes to git.

### How do I know if a file was edited by Codex or Claude?

Use `ai-blame blame <file>`:

```bash
ai-blame blame foo.md
```

This shows:
- **claude-haiku-4-5-...**: Edit made by Claude Code
- **gpt-5.2-codex**: Edit made by Codex CLI
- **No attribution**: Untracked changes or uncommitted edits

The timestamp and line numbers show exactly what changed when.

### Why do some lines show Claude attribution when Codex edited them?

This happens when:

1. **Claude initially created the file** (stored in Claude Code traces)
2. **Codex later added lines** (but via bash `>>` or uncommitted changes)
3. **ai-blame can only see the Claude creation** (because Codex changes aren't in parseable traces)

**Example** from your test:
- Lines 1-21: Created by Claude (2025-12-28 15:21)
- Lines 22-23: Added by Codex bash script (visible in logs but not in traces)
- Result: All lines show Claude attribution

**Why this is expected**: The blame algorithm works backwards through trace records. Without a structured edit record, it can't attribute those lines to Codex.

**Workaround**: Commit the Codex changes to git before running `ai-blame`:

```bash
# After Codex finishes and modifies files:
git add foo.md
git commit -m "Codex updates"
ai-blame blame foo.md  # Now it will detect the edit
```

## Performance

### Why is `ai-blame stats` slow?

The tool needs to parse hundreds of trace files to extract edit records. Initial runs typically take 30-60 seconds depending on trace history size.

### How can I speed up repeated runs?

Caching is enabled by default! No special configuration needed:

```bash
# First run: ~55 seconds (builds cache)
ai-blame stats

# Second run: ~3-5 seconds (uses cache)
ai-blame stats
```

You can disable it with `--no-cache` or `export AI_BLAME_NO_CACHE=1`.

See [Performance and Caching](./how-to/performance-and-caching.md) for full details.

### How much faster is caching?

**Expected speedup**: ~90% for unchanged traces

| Scenario | Time |
|----------|------|
| First run (no cache) | 53 seconds |
| First run (with cache) | ~55 seconds |
| Second run (cache hit) | ~3-5 seconds |
| 10% files changed | ~10-15 seconds |

Speedup scales with trace count - larger trace histories see bigger wins.

### Where is the cache stored?

The cache is stored in `.ai-blame.ddb` next to your trace files:

```
~/.claude/projects/myproject/
├── session-1.jsonl
├── session-2.jsonl
└── .ai-blame.ddb  ← Cache file
```

### How do I clear the cache?

```bash
# Delete the cache file
rm .ai-blame.ddb

# Or force rebuild via CLI
ai-blame stats --rebuild-cache
```

## General Questions

### Which AI tools does ai-blame support?

- **Claude Code**: Full support (stores structured edit records in traces)
- **GitHub Copilot**: Full support (similar trace structure)
- **Codex CLI**: Full support (via ghost commits + snapshot snapshots)
- **Other tools**: Limited support (depends on trace format)

See [Trace Format](./explanation/trace-format.md) for details.

### Can I use ai-blame without traces?

No - `ai-blame` requires trace data to know who made which changes. Without traces, it can only show:
- Git history (via `git blame`)
- File creation/modification times (generic, not AI-specific)

For uncommitted changes without trace data, use:
```bash
git diff  # See what changed
git log   # See git history with attribution
```

### How do I trust the attribution?

The attribution is as trustworthy as your traces:

1. **Traces are generated by the AI tool** (Claude, Copilot, Codex)
2. **Timestamps come from the tool's system clock**
3. **No modifications are made** to traces during extraction

However:
- Traces don't verify that the AI actually made the changes (user could edit after)
- Uncommitted working directory changes are invisible to `ai-blame`
- Ghost commits can be cleaned up by aggressive `git gc`

**Best practice**: Use `ai-blame` in conjunction with `git log` for full provenance understanding.

### Can I exclude certain files from blame?

Yes - use the configuration file:

```yaml
# .ai-blame.yaml
filters:
  file_pattern: "*.py"  # Only show Python files
  min_change_size: 100  # Ignore changes smaller than 100 bytes
```

See [Configuration](./how-to/configuration.md) for details.

### What if ai-blame reports no edits found?

Possible causes:

1. **No traces in the expected directory**
   - Check `~/.claude/projects/` and `~/.codex/sessions/`
   - Ensure the project was actually edited with Claude Code or Codex CLI

2. **File pattern doesn't match**
   - Use `ai-blame report` without filters to see all files
   - Check file paths are absolute or relative correctly

3. **Traces are for a different project**
   - Claude Code and Codex CLI create separate traces per project
   - Use `--trace-dir` to specify the correct trace directory

4. **Time filters excluded all edits**
   - Check `--since` and `--until` date ranges

**Debugging**:
```bash
ai-blame stats --trace-dir ~/.claude/projects
ai-blame report . --show-all  # See all files and edits
```

## See Also

- [Codex CLI Traces](./explanation/codex-traces.md) - Technical details about ghost commits
- [Claude Code Traces](./explanation/claude-traces.md) - How Claude traces work
- [How It Works](./explanation/how-it-works.md) - The blame algorithm explained
- [CLI Reference](./reference/cli.md) - Command-line options
