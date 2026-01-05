# Transcripts and Subagents

The `ai-blame transcript` commands let you explore and analyze the complete history of Claude Code and Codex sessions. Understanding how transcripts work—especially with **subagents**—helps you navigate multi-step tasks and complex interactions.

## What is a Transcript?

A **transcript** is a complete record of one interaction session between you and an AI agent. It includes:

- **Messages**: All back-and-forth exchanges (user prompts and assistant responses)
- **Metadata**: Session ID, agent name, start/end times, working directory
- **Content Blocks**: Not just text—also tool uses, file operations, thinking blocks, commands, and their outputs
- **Trace File**: The source `.jsonl` file where the session was originally recorded

### Example Session

```
User:    "Help me refactor this function"
Claude:  "I'll analyze the code... [reads file] Here's a refactored version..."
User:    "That looks good, apply it"
Claude:  "Applying changes... [creates/edits file]"
```

Each of these interactions appears as a message in the transcript.

---

## What are Subagents?

Claude Code can autonomously spawn **subagents** to handle complex, multi-step tasks. These are specialized AI instances that can:

- **Research**: The `Explore` agent searches and analyzes codebases
- **Plan**: The `Plan` agent designs implementation strategies
- **Task**: The `Task` agent handles complex multi-step operations
- **And more**: Other specialized agents for specific domains

### When are Subagents Created?

Claude Code launches subagents when:

1. You ask a complex question (e.g., "Find all places where errors are handled")
2. You request implementation with a complex design (e.g., "Add authentication to my app")
3. You need research or exploration of an unfamiliar codebase
4. A task is too complex for a single agent to handle efficiently

### Key Point: Shared Session IDs

When Claude Code spawns a subagent, the **subagent shares the same Session ID as the parent session**. However, each subagent interaction is recorded in a **separate trace file**.

This means you may see the same Session ID appearing multiple times in `transcript list` output—each row represents a different trace file (parent session or subagent session), not a different user interaction.

---

## Understanding Session ID Duplication

Here's a real example from your earlier output:

```
Session ID                               Agent           Start Time             Msgs  Files
-----------------------------------------------------------------------------------------------
483c7d95-6b9c-46db-afd4-3ecb6257781a     claude-code     2025-12-31 03:56         53      0
483c7d95-6b9c-46db-afd4-3ecb6257781a     claude-code     2025-12-31 03:55          2      0
483c7d95-6b9c-46db-afd4-3ecb6257781a     claude-code     2025-12-31 03:55          2      0
```

All three rows have the same Session ID (`483c7d95...`). Here's what this means:

1. **Row 1 (53 msgs)**: Main Claude Code session—the parent interaction you initiated
2. **Row 2 (2 msgs)**: A subagent (e.g., Explore agent) that Claude spawned to research something
3. **Row 3 (2 msgs)**: Another subagent (e.g., Task agent) that Claude spawned for a specific operation

All three are part of the same **user-initiated conversation**, but they're recorded separately because each involves different agent tools or different trace files.

---

## Why Separate Trace Files?

Separate trace files for subagents provide several benefits:

1. **Isolation**: Subagent work is isolated and can be reviewed independently
2. **Clear hierarchy**: You can see the parent session and its child sessions
3. **Efficient parsing**: Subagent traces can be cached and invalidated independently (per-file caching in Codex)
4. **Deduplication**: Using UUIDs (Claude) or sequential IDs (Codex), the system prevents duplicate messages across related traces

---

## Viewing Transcripts with Subagents

### Listing with Model Information

To see which models were used in each trace file (helpful for identifying agent types), use:

```bash
ai-blame transcript list --columns SATMO
```

Output might look like:

```
Session ID                        Agent           Start Time           Msgs  Models
───────────────────────────────────────────────────────────────────────────────────────
483c7d95-6b9c-46db-afd4-3ec...   claude-code     2025-12-31 03:56      53    claude-opus-4.5
483c7d95-6b9c-46db-afd4-3ec...   claude-code     2025-12-31 03:55      2     claude-haiku
483c7d95-6b9c-46db-afd4-3ec...   claude-code     2025-12-31 03:55      2     claude-haiku
```

- The main session used **Opus 4.5** (more capable model)
- The subagents used **Haiku** (faster, lighter-weight model)

### Viewing a Specific Subagent Transcript

Since each trace file is independent, you can view any subagent transcript by session ID:

```bash
# View the main session
ai-blame transcript view 483c7d95

# View a specific subagent session
ai-blame transcript view 483c7d95 --full --show-thinking
```

Because all three rows have the same Session ID, Claude will find and display the first matching transcript. If you need to view a specific file, you can pass the full path:

```bash
ai-blame transcript view ~/.claude/projects/-Users-alice-project/483c7d95-agent-explore.jsonl
```

---

## Understanding Agent-Touched Files with Subagents

When Claude spawns a subagent to explore or research:

1. **Subagent reads files** without modifying them (files_touched may show 0)
2. **Main session writes files** based on subagent recommendations
3. **Cache is intelligently invalidated** when traces change

This is why you might see:

```
483c7d95...   claude-code     2025-12-31 03:56      53    Files: 5   (main session modified files)
483c7d95...   claude-code     2025-12-31 03:55      2     Files: 0   (subagent only read files)
```

---

## Data Model Summary

```
┌─ User-Initiated Session (Session ID: 483c7d95...)
│
├─ Trace File: session.jsonl
│  ├─ Message 1: User prompt
│  ├─ Message 2-53: Claude responses, tool uses, file edits
│  └─ Metadata: started at 03:56, modified 5 files
│
├─ Trace File: session-agent-explore.jsonl (spawned subagent)
│  ├─ Message 1: Explore task
│  ├─ Message 2: Exploration results
│  └─ Metadata: started at 03:55, read 0 files (no writes)
│
└─ Trace File: session-agent-task.jsonl (spawned subagent)
   ├─ Message 1: Task request
   ├─ Message 2: Task completion
   └─ Metadata: started at 03:55, read 0 files
```

All share the same Session ID but have different trace files and potentially different models.

---

## CLI Tips for Subagent Workflows

### See Everything (including subagents)

```bash
# Show all transcripts with models—helps distinguish subagents
ai-blame transcript list -n 0 --columns SATMO
```

### Export a Complex Session for Documentation

```bash
# Export the main session and subagent work as markdown
ai-blame transcript view 483c7d95 --format markdown --full > main-session.md
```

### Analyze Subagent Performance

Use the `--show-thinking` and `--show-tools` flags to understand what subagents are doing:

```bash
ai-blame transcript view 483c7d95 --show-thinking --show-tools
```

---

## Related Documentation

- [CLI Reference: transcript list](../reference/cli.md#ai-blame-transcript-list) — Column specifications and examples
- [CLI Reference: transcript view](../reference/cli.md#ai-blame-transcript-view) — View options and output formats
- [How It Works: Trace Format](./trace-format.md) — Low-level detail on how traces are structured
- [Claude Traces](./claude-traces.md) — Claude Code trace format specifics
- [Codex Traces](./codex-traces.md) — Codex CLI trace format specifics
