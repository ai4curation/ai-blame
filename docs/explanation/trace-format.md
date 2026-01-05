# Trace Format

This document explains the structure of trace files that `ai-blame` parses.

## Claude Code Traces

### File Location

Claude Code stores traces in:

```
~/.claude/projects/<encoded-cwd>/
```

`<encoded-cwd>` is a filesystem-safe encoding of your working directory path (path separators become `-`, and punctuation like `.` may also be normalized to `-`).

Each session creates a JSONL file named with a UUID:

```
~/.claude/projects/-Users-alice-myproject/
├── a1b2c3d4-5678-90ab-cdef-123456789abc.jsonl
├── b2c3d4e5-6789-01cd-ef23-456789abcdef.jsonl
└── agent-c3d4e5f6-7890-12de-f345-6789abcdef01.jsonl
```

Files prefixed with `agent-` are from subagent invocations.

### JSONL Structure

Each line is a JSON object representing a message in the conversation.

### Key Fields for Extraction

`ai-blame` extracts these fields when present:

| Field | Location | Description |
|-------|----------|-------------|
| `file_path` | `toolUseResult.filePath` | Absolute path to the file |
| `timestamp` | root `timestamp` | When the operation occurred |
| `model` | parent assistant `message.model` | Model identifier |
| `session_id` | root `sessionId` | Session identifier |
| `is_create` | `toolUseResult.type == "create"` | Whether file was created |
| `agent_version` | root `version` | Claude Code version |

## Codex/GitHub Copilot Traces

### File Location

Codex traces may be stored in various locations depending on the client/implementation:

```
~/.codex/                              # Codex Direct (may have history.jsonl or sessions/)
~/.codex/sessions/                     # Codex sessions directory
~/.copilot/traces/                     # GitHub Copilot
~/.config/github-copilot/traces/       # GitHub Copilot (config variant)
~/.vscode/copilot/traces/              # VS Code Copilot extension
~/.cursor/traces/                      # Cursor IDE
~/.openai/traces/                      # OpenAI API
```

`ai-blame` scans these locations automatically and uses any that exist. Files are typically `.jsonl` format (one JSON object per line).

### JSONL Structure

Each line is a JSON object representing a completion or edit event.

### Key Fields for Extraction

`ai-blame` extracts these fields when present:

| Field | Location | Description |
|-------|----------|-------------|
| `file` or `file_path` or `filePath` | root | Path to the file |
| `timestamp` | root `timestamp` | When the operation occurred (RFC3339) |
| `model` | root `model` | Model identifier (e.g., "gpt-4", "codex-davinci-002") |
| `session_id` or `sessionId` | root | Session identifier |
| `event` or `action` | root | Event type ("create", "edit", "completion") |
| `content` | root `content` | File content for create operations |
| `old_content` | root `old_content` | Original content for edits |
| `new_content` | root `new_content` | New content for edits |
| `diff` or `patch` | root | Structured diff/patch information |

### Example Codex Trace Records

**Create event:**
```json
{
  "event": "create",
  "file": "/path/to/file.py",
  "model": "gpt-4",
  "timestamp": "2025-12-01T08:00:00Z",
  "session_id": "abc123",
  "content": "def hello():\n    print('Hello')\n"
}
```

**Edit event:**
```json
{
  "event": "edit",
  "file_path": "/path/to/file.py",
  "model": "gpt-4",
  "timestamp": "2025-12-01T08:05:00Z",
  "session_id": "abc123",
  "old_content": "def hello():\n    pass\n",
  "new_content": "def hello():\n    print('Hello')\n"
}
```

## Mixed Traces

`ai-blame` can parse trace files containing both Claude Code and Codex format records in the same file, automatically detecting the format of each record.
