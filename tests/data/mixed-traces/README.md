# Mixed Traces Test Fixtures

This directory contains test fixtures combining both Claude Code and Codex CLI traces for the same project.

## Directory Structure

- `claude-traces/` - Claude Code editor traces (JSONL files from ~/.claude/projects/...)
- `codex-sessions/` - Codex CLI session traces (JSONL files from ~/.codex/sessions/...)
- `repo/` - Working files from the test project

## Known Issues & Observations

### foo.md Attribution Bug

The file `foo.md` was initially created by Claude Code. Later, Codex CLI sessions were run in the same directory, but the last two lines of the file:

```
A lantern hums beside the gate.
Footsteps stitch the thaw to dawn.
```

Are shown as being attributed to Claude (2025-12-28), when they appear to have been added by Codex (2025-12-29).

**Root Cause**: These lines exist in the working directory but haven't been committed to any Codex CLI "ghost commit" yet. The Codex CLI creates ghost commits when snapshots are taken during tool execution, but uncommitted working directory changes are not captured in the traces.

**Expected Behavior**: Without a corresponding edit record in the traces, the blame algorithm can only attribute these lines based on the file's edit history in the traces, which shows them as part of the original Claude creation.

**To Properly Attribute**: Would require:
1. Codex CLI to commit these changes to a ghost commit, or
2. A separate mechanism to track uncommitted working directory changes alongside trace sessions

## Test Coverage

- `test_mixed_claude_and_codex_traces`: Verifies extraction from both Claude and Codex traces
- `test_codex_cli_file_modifications_detected`: Verifies Codex file modification detection across ghosts
