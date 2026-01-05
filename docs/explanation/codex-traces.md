# Codex CLI Traces and Ghost Commits

This document explains how Codex CLI records file edits using "ghost commits" and how `ai-blame` extracts blame attribution from these traces.

## What is Codex?

[Codex](https://developers.openai.com/docs/guides/codex) is OpenAI's AI coding assistant available through:

- **Codex CLI** (`codex` command-line tool): Direct CLI access to code generation
- **GitHub Copilot**: IDE integration in VS Code and other editors
- **OpenAI API**: Custom integrations

This document focuses on **Codex CLI traces** and the unique "ghost commit" mechanism it uses to track file changes.

## Trace Storage

Codex CLI stores session traces in:

```
~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<session-id>.jsonl
```

For example:

```
~/.codex/sessions/
├── 2025/
│   └── 12/
│       └── 29/
│           ├── rollout-2025-12-29T16-54-27-019b6b08-58f3-7580.jsonl
│           └── rollout-2025-12-29T17-33-00-019b6b2b-85de-7a40.jsonl
├── history.jsonl
└── config.toml
```

Each file represents a single Codex CLI session with one JSON object per line.

## Ghost Commits: The Unique Codex Innovation

Unlike Claude Code which stores file content directly in traces, **Codex CLI stores file changes as git commits in your repository's git object database**. These are called "ghost commits":

```
Ghost Commit Structure:
  commit <hash>
  tree <tree-hash>
  author Codex Snapshot <snapshot@codex.local>

  codex snapshot

  <file contents>
```

### Why Ghost Commits?

Ghost commits serve several purposes:

1. **Efficient storage**: Don't need to store full file contents in traces
2. **Git integration**: Leverage git's existing object storage and diff capabilities
3. **History preservation**: Can track the evolution of files through snapshots
4. **Content integrity**: Use git's SHA hashing for content verification

### How It Works

When Codex CLI runs in your repository:

1. At various points (often start and end of operations), Codex creates a "snapshot"
2. Each snapshot captures the state of all tracked files
3. A git commit object is created in your `.git/objects/` directory
4. The commit hash is recorded in the trace as `ghost_commit.id`
5. Later, content can be retrieved with: `git show <commit-hash>:<filepath>`

## Trace Structure

### Ghost Snapshot Records

The key records for blame attribution are `response_item` events with `ghost_snapshot` payloads:

```json
{
  "timestamp": "2025-12-29T16:55:51.498Z",
  "type": "response_item",
  "payload": {
    "type": "ghost_snapshot",
    "ghost_commit": {
      "id": "bf1fe931bc70f605aacd81822dd1d8dea2ca0822",
      "parent": null,
      "preexisting_untracked_files": [
        ".gitignore",
        ".python-version",
        "README.md",
        "config.yaml",
        "created-with-codex.md",
        "data.json",
        "example.py",
        "foo.md",
        "main.py",
        "notes.md",
        "pyproject.toml"
      ],
      "preexisting_untracked_dirs": []
    }
  }
}
```

**Key fields:**
- `timestamp`: RFC3339-formatted timestamp of the snapshot
- `ghost_commit.id`: SHA1 hash of the git commit object
- `preexisting_untracked_files`: List of files that existed in the repo at this point

### Turn Context Records

Model information is in `turn_context` events:

```json
{
  "timestamp": "2025-12-29T16:54:43.779Z",
  "type": "turn_context",
  "payload": {
    "cwd": "/Users/alice/myproject",
    "approval_policy": "never",
    "sandbox_policy": { "type": "danger-full-access" },
    "model": "gpt-5.2-codex",
    "summary": "auto"
  }
}
```

**Key fields:**
- `model`: The model used (e.g., `gpt-5.2-codex`)
- `cwd`: Current working directory during the session

## Blame Extraction Algorithm

`ai-blame` reconstructs file blame from Codex traces by:

1. **Parsing ghost snapshots** in chronological order
2. **Comparing file lists** between successive snapshots to find changes
3. **For added files**: Using `git show <commit>:<file>` to retrieve content
4. **For modified files**: Comparing content between commits to detect changes
5. **Creating edit records** with the snapshot timestamp and model

### Example: File Modifications Across Snapshots

Given three snapshots:

**Snapshot 1** (2025-12-29 16:54:43, commit: 808b54e):
```
Files: config.yaml, data.json, example.py, foo.md, ...
```

**Snapshot 2** (2025-12-29 16:55:24, commit: 18538bd):
```
Files: config.yaml, data.json, example.py, foo.md, ...  (no changes)
```

**Snapshot 3** (2025-12-29 16:55:51, commit: bf1fe93):
```
Files: config.yaml, data.json, example.py, foo.md, created-with-codex.md, ...
```

`ai-blame` would attribute:
- `created-with-codex.md`: Added in Snapshot 3 → created by gpt-5.2-codex at 16:55:51
- Other files: No changes detected in Snapshot 2→3 → no new edits
- If `foo.md` content differed between snapshots, it would be marked as modified

## Accessing Ghost Commits

You can manually inspect ghost commits:

```bash
# View a file from a ghost commit
git show <commit-hash>:<filepath>

# View the commit object
git cat-file -p <commit-hash>

# List git objects in your repo
ls -la .git/objects/
```

## Known Limitations and Observations

### 1. Uncommitted Working Directory Changes

**Limitation**: If you edit files after Codex runs but before the next snapshot, those changes won't be in any ghost commit.

**Example**: You run `codex exec "create file A"`, Codex creates a snapshot showing file A. Then you manually edit file A. Those manual edits are in your working directory but **not** in any ghost commit, so they can't be blamed.

**Workaround**: Codex will capture changes in the next snapshot when it runs, or you can commit the changes to git manually.

### 2. Ghost Commits Are Ephemeral

**Observation**: Ghost commits are stored in `.git/objects/` but are not part of any ref (branch, tag, etc.). If you run `git gc`, they might be cleaned up.

**Best Practice**: Don't run aggressive git garbage collection on repositories with active Codex CLI usage.

### 3. Multiple Sessions in Same Repo

**Observation**: Multiple Codex CLI sessions in the same repo share the same git object database, but each session creates independent ghost commits.

**Attribution**: Each session records files as they existed during that session, so `ai-blame` correctly attributes edits to the right session and model.

## API and Documentation

For more information about Codex CLI, see the official documentation:

- **[Codex CLI Documentation](https://developers.openai.com/docs/guides/codex/cli)** - Official guide
- **[Codex CLI Features](https://developers.openai.com/docs/guides/codex/cli/features)** - Feature reference
- **[Codex CLI Reference](https://developers.openai.com/docs/guides/codex/cli/reference)** - Command-line options
- **[Codex CLI Exec Guide](https://github.com/openai/codex/blob/main/docs/exec.md)** - Exec command documentation
- **[OpenAI Codex Overview](https://developers.openai.com/docs/guides/codex)** - General Codex information

## Codex CLI vs GitHub Copilot Traces

This document focuses on **Codex CLI** traces. GitHub Copilot (IDE integration) stores traces differently:

| Aspect | Codex CLI | GitHub Copilot |
|--------|-----------|-----------------|
| **Storage** | `~/.codex/sessions/` | IDE/Copilot config directories |
| **Format** | JSON with ghost commits | JSON with structured events |
| **File Changes** | Ghost commits in `.git/objects/` | Direct event records |
| **Attribution** | Snapshot timestamps | Event timestamps |

`ai-blame` supports both formats automatically.

## See Also

- [Claude Traces](./claude-traces.md) - Claude Code trace format
- [Trace Format](./trace-format.md) - Overview of all supported formats
- [How It Works](./how-it-works.md) - General blame algorithm
