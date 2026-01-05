# Claude Code Traces

This document explains how Claude Code records file edits and how `ai-blame` extracts blame attribution from these traces.

## Overview

Claude Code (formerly Claude Editor) is Anthropic's command-line tool for AI-assisted development. It tracks all file edits in structured trace files that include timestamps, model identifiers, and file change information.

## Trace Storage

Claude Code stores traces in the `.claude` directory in your home directory:

```
~/.claude/projects/<encoded-path>/
```

where `<encoded-path>` is the current working directory path with `/` replaced by `-`. For example:

```
~/.claude/projects/-Users-alice-myproject/
├── a1b2c3d4-5678-90ab-cdef-123456789abc.jsonl
├── b2c3d4e5-6789-01cd-ef23-456789abcdef.jsonl
└── agent-c3d4e5f6-7890-12de-f345-6789abcdef01.jsonl
```

Each `.jsonl` file represents a single Claude Code session with one JSON object per line.

### Session Types

- **Regular sessions**: Standard Claude Code sessions, named with a UUID
- **Agent sessions**: Claude Code subagent invocations, prefixed with `agent-`

## Trace Structure

### Message Flow

Claude Code traces record the conversation between the user and Claude, including:

1. **User messages**: Commands and questions from the user
2. **Assistant responses**: Claude's reasoning and decisions
3. **Tool use messages**: Claude using file editing tools
4. **Tool results**: Outcome of file operations

### Key Objects for Blame Attribution

`ai-blame` extracts edit information from two main record types:

#### 1. Parent Assistant Messages

These contain the model identifier used for the edit:

```json
{
  "uuid": "assistant-uuid-123",
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-5-20251101",
    "content": [
      {
        "type": "tool_use",
        "id": "toolu_123abc",
        "name": "EditFile",
        "input": { ... }
      }
    ]
  }
}
```

**Key fields:**
- `message.model`: The Claude model used (e.g., `claude-opus-4-5-20251101`, `claude-haiku-4-5-20251001`)
- `message.content[].id`: Tool use ID for matching with tool results

#### 2. Tool Result Messages (User Messages with File Operations)

These contain the actual file edit operations:

```json
{
  "uuid": "user-uuid-456",
  "type": "user",
  "timestamp": "2025-12-01T08:00:00Z",
  "sessionId": "session-123",
  "parentUuid": "assistant-uuid-123",
  "toolUseResult": {
    "type": "create",
    "filePath": "/Users/alice/myproject/main.py",
    "content": "def hello():\n    print('Hello')\n"
  }
}
```

or for edits:

```json
{
  "uuid": "user-uuid-456",
  "type": "user",
  "timestamp": "2025-12-01T08:05:00Z",
  "sessionId": "session-123",
  "parentUuid": "assistant-uuid-123",
  "toolUseResult": {
    "type": "edit",
    "filePath": "/Users/alice/myproject/main.py",
    "oldString": "def hello():\n    pass\n",
    "newString": "def hello():\n    print('Hello')\n",
    "structuredPatch": "@@ -1,2 +1,2 @@"
  }
}
```

**Key fields:**
- `timestamp`: RFC3339-formatted timestamp of when the edit occurred
- `sessionId`: Identifies which Claude Code session performed the edit
- `parentUuid`: References the assistant message (for model lookup)
- `toolUseResult.filePath`: Absolute path to the edited file
- `toolUseResult.type`: Either "create" or "edit"
- `toolUseResult.oldString`: Previous content (for edits)
- `toolUseResult.newString`: New content (for creates or edits)
- `toolUseResult.content`: Full file content (for creates)
- `toolUseResult.structuredPatch`: Unified diff format patch

## Blame Extraction Algorithm

`ai-blame` reconstructs file blame by:

1. **Parsing all tool result messages** in chronological order
2. **Resolving the model** by looking up the `parentUuid` in parent assistant messages
3. **Reverse-applying edits** from newest to oldest to map current lines to their origins
4. **Matching new content** to the original file using the patch hints from `structuredPatch`
5. **Attributing lines** to the edit that introduced them

### Example: Three-Edit Sequence

Given a file `math.py` that went through three edits:

**Edit 1** (2025-12-01 08:00:00, claude-opus):
```python
def add(a, b):
    return a + b
```

**Edit 2** (2025-12-01 08:05:00, claude-haiku):
```python
def add(a, b):
    """Add two numbers."""
    return a + b
```

**Edit 3** (2025-12-01 08:10:00, claude-opus):
```python
def add(a, b):
    """Add two numbers."""
    result = a + b
    return result
```

`ai-blame` would attribute:
- Lines 1-2: claude-opus (from Edit 1)
- Line 3: claude-haiku (from Edit 2 - docstring addition)
- Lines 4-5: claude-opus (from Edit 3 - variable introduction)

## Known Limitations

1. **Agent subagents**: Edits within agent sessions are attributed to "claude-code-agent" rather than a specific model version
2. **Deleted content**: Once lines are deleted, they cannot be blamed (there's no record of deletion)
3. **Complex refactors**: Very large changes might have matching issues if context is ambiguous
4. **Timestamps**: Only edits recorded in traces have timestamps; uncommitted work has no trace

## See Also

- [Codex CLI Traces](./codex-traces.md) - Codex CLI trace format and ghost commits
- [Trace Format](./trace-format.md) - Overview of all supported trace formats
- [How It Works](./how-it-works.md) - General architecture and blame algorithm
