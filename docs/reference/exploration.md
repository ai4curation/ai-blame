# Trace Exploration

Discover, understand, and explore AI agent execution traces in your project.

---

## Conceptual Model

**Trace exploration** answers: "What traces exist? What changed? What did the AI do?"

Four commands work together in a discovery workflow:

1. **`stats`** — Quick overview: How many traces? How many edits?
2. **`timeline`** — Chronological history: What changed when?
3. **`transcript list`** — See all AI sessions: Who worked on this?
4. **`transcript view`** — Deep dive: What did a specific session do?

Each answers progressively more detailed questions.

---

## The `stats` Command

### What It Does

Shows statistics about available traces and edits. Answers: "What do I have to work with?"

### Command Syntax

```bash
ai-blame stats [OPTIONS]
```

### Basic Usage

**Show all traces:**

```bash
ai-blame stats
```

**For a specific project:**

```bash
ai-blame stats --dir /path/to/project
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

### Example Output

```
$ ai-blame stats

Trace directory: /Users/alice/.claude/projects/-Users-alice-myproject/
Trace files: 5
  Session traces: 3
  Agent traces: 2

Files with edits (all files): 23
Total successful edits: 47

Top edited files:
  1. disease_definitions.yaml (8 edits)
  2. phenotypes.yaml (6 edits)
  3. config.yaml (5 edits)
```

### Interpreting Stats

- **Trace files:** Number of recorded sessions (.jsonl files)
- **Session traces:** Parent Claude Code/Codex sessions
- **Agent traces:** Subagent sessions (spawned by parent)
- **Files with edits:** How many unique files were modified
- **Total edits:** Sum of all CREATE/EDIT/DELETE operations

---

## The `timeline` Command

### What It Does

Shows a chronological timeline of all file modifications from traces. Answers: "What changed when?"

### Command Syntax

```bash
ai-blame timeline [OPTIONS]
```

### Basic Usage

**Show complete timeline:**

```bash
ai-blame timeline
```

**Filter by file type:**

```bash
ai-blame timeline --pattern ".yaml"
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--pattern <PATTERN>` | `-p` | `""` | Filter files by path pattern |

### Example Output

```
$ ai-blame timeline

=== Timeline of Actions ===

2025-12-01 08:03:42 UTC  [CREATED]  disease_definitions.yaml (claude-opus-4-5)
2025-12-01 08:05:18 UTC  [EDITED]   disease_definitions.yaml (claude-opus-4-5)
2025-12-02 14:12:15 UTC  [CREATED]  phenotypes.yaml (gpt-4)
2025-12-02 14:15:30 UTC  [EDITED]   config.yaml (claude-opus-4-5)
2025-12-05 10:22:05 UTC  [EDITED]   disease_definitions.yaml (claude-3-5-sonnet)
```

### Understanding Timeline

- **Timestamp** (UTC) — When the edit occurred
- **Action** — CREATED, EDITED, or DELETED
- **File** — Which file was affected
- **Model** — Which AI model made the change

Use timeline to:
- See project evolution chronologically
- Identify when each file was first created
- Spot patterns (which files change together?)
- Check if models changed over time

---

## The `transcript list` Command

### What It Does

Lists all AI sessions (Claude Code, Codex, subagents). Answers: "Which sessions touched this project?"

### Command Syntax

```bash
ai-blame transcript list [OPTIONS]
```

### Basic Usage

**List 20 most recent sessions (default):**

```bash
ai-blame transcript list
```

**List ALL sessions:**

```bash
ai-blame transcript list -n 0
```

**Show models used in each session:**

```bash
ai-blame transcript list --columns SATMO
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--limit <N>` | `-n` | 20 | Show N most recent (0 for all) |
| `--format <table\|json>` | | `table` | Output format |
| `--columns <SPEC>` | | SATM | Column specification |

### Example: Default Output (SATM)

```
$ ai-blame transcript list

Session ID                                  Agent          Start Time              Msgs
────────────────────────────────────────────────────────────────────────────────────────
483c7d95-6b9c-46db-afd4-3ecb6257781a       claude-code    2025-12-01 08:03        53
7f2d4e91-8a5c-4e1b-9c6e-2f8d4a3b9c5e       claude-code    2025-11-30 14:22        28
b1e5f9c3-6d2a-4f8e-9a1c-3e7b8f2d5a9c       codex-cli      2025-11-28 10:15        12
```

### Example: With Models (SATMO)

```
$ ai-blame transcript list --columns SATMO

Session ID                                  Agent          Start Time              Msgs  Models
──────────────────────────────────────────────────────────────────────────────────────────────
483c7d95-6b9c-46db-afd4-3ecb6257781a       claude-code    2025-12-01 08:03        53    claude-opus-4-5
7f2d4e91-8a5c-4e1b-9c6e-2f8d4a3b9c5e       claude-code    2025-11-30 14:22        28    claude-haiku
```

### Column Layout Specification

Use `--columns` to customize which columns appear:

| Letter | Column | Description |
|--------|--------|-------------|
| `S` | Session ID | Unique identifier (40 chars) |
| `A` | Agent | Tool used (claude-code, codex-cli, etc.) |
| `T` | Start Time | Session start timestamp |
| `M` | Message Count | Number of messages in session |
| `F` | Files Touched | Number of files modified |
| `O` | Models Used | Models used in this trace |
| `L` | Last Message | Preview of last assistant message (50 chars) |

**Example layouts:**

- `SATM` (default) — Session, Agent, Time, Messages
- `SATMO` — Add Models column (helps identify subagents)
- `ATOM` — Skip Session ID, focus on Agent/Time/Messages/Models
- `SATMOL` — Everything

---

## Understanding Session ID Duplication

You may see the same Session ID multiple times in `transcript list`. **This is normal** and happens when:

1. **Multi-agent sessions:** Claude Code spawns subagents (Explore, Task, Plan) that share parent Session ID
2. **Session spanning files:** Long sessions split across multiple trace files
3. **Agent differentiation:** Using `--columns SATMO` shows which models were used

**Example:**

```
Session ID                                  Agent          Models
────────────────────────────────────────────────────────────────────
483c7d95-6b9c-46db-afd4-3ecb6257781a       claude-code    claude-opus-4-5      (main session)
483c7d95-6b9c-46db-afd4-3ecb6257781a       claude-code    claude-haiku         (subagent 1)
483c7d95-6b9c-46db-afd4-3ecb6257781a       claude-code    claude-haiku         (subagent 2)
```

All three share the same Session ID but used different models. The main session typically uses more powerful model (Opus), subagents use faster model (Haiku).

**→ Learn more:** [Transcripts & Subagents](../explanation/transcripts-and-subagents.md)

---

## The `transcript view` Command

### What It Does

Displays a detailed transcript of a specific Claude Code or Codex session. Answers: "What exactly did this session do?"

### Command Syntax

```bash
ai-blame transcript view [OPTIONS] <SESSION>
```

### Basic Usage

**View by Session ID (substring match):**

```bash
ai-blame transcript view 483c7d95
```

**View with full details:**

```bash
ai-blame transcript view 483c7d95 --full
```

**Show AI reasoning (thinking blocks):**

```bash
ai-blame transcript view 483c7d95 --show-thinking
```

**Show tool usage details:**

```bash
ai-blame transcript view 483c7d95 --show-tools
```

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--format <text\|markdown\|json>` | | `text` | Output format |
| `--full` | | False | Show full content (don't truncate) |
| `--show-thinking` | | False | Display thinking/chain-of-thought |
| `--show-tools` | | False | Display tool use and results |

### Example Output

**Default (text, truncated):**

```
Session: 483c7d95-6b9c-46db-afd4-3ecb6257781a
Agent: claude-code
Start: 2025-12-01 08:03:42 UTC
Messages: 53
Files Touched: 5
Models: claude-opus-4-5-20251101
Working Directory: /Users/alice/myproject
Trace File: ~/.claude/projects/-Users-alice-myproject/483c7d95.jsonl

─────────────────────────────────────────────────────────────────

User Message 1:
  Help me create a disease definitions schema

Claude Response 1:
  I'll create a LinkML schema for disease definitions. [creates file]...

User Message 2:
  Add phenotypes section

Claude Response 2:
  I'll extend the schema with phenotype associations... [truncated]
```

### Output Formats

**Text (default):**

Clean, terminal-friendly display with section breaks and controlled truncation. Good for interactive exploration.

**Markdown:**

GitHub-flavored markdown suitable for documentation or GitHub issues:

```bash
ai-blame transcript view 483c7d95 --format markdown > session.md
```

**JSON:**

Complete machine-readable transcript with all fields:

```bash
ai-blame transcript view 483c7d95 --format json | jq .
```

Good for programmatic processing or archiving.

---

## Common Exploration Workflows

### Workflow 1: Project Overview (5 minutes)

Understanding what you're working with:

```bash
# Step 1: See overall statistics
ai-blame stats

# Step 2: View timeline of changes
ai-blame timeline

# Output shows: how many traces, what changed, what changed when
```

### Workflow 2: Session Discovery (10 minutes)

Finding specific sessions:

```bash
# Step 1: List all sessions
ai-blame transcript list -n 0 --columns SATMO

# Step 2: View a specific session
ai-blame transcript view 483c7d95

# Step 3: For more detail
ai-blame transcript view 483c7d95 --full --show-thinking
```

### Workflow 3: Understanding Subagents (5 minutes)

When Claude spawned helper agents:

```bash
# Step 1: Look for repeated Session IDs with different models
ai-blame transcript list --columns SATMO

# Step 2: View the main session
ai-blame transcript view 483c7d95 --full

# Step 3: Learn about subagent architecture
# (See: Understanding Subagents documentation)
```

### Workflow 4: File History (3 minutes)

What happened to a specific file:

```bash
# See when it was created/modified
ai-blame timeline --pattern "config.yaml"

# See all sessions that touched it
ai-blame stats --pattern "config.yaml"

# Line-by-line attribution
ai-blame blame config.yaml
```

---

## Filtering and Searching

### Filter by File Pattern

Show only changes to YAML files:

```bash
ai-blame timeline --pattern ".yaml"
ai-blame stats --pattern ".yaml"
```

Filter by directory:

```bash
ai-blame timeline --pattern "docs/"
```

### Find Sessions with Specific Models

Use `--columns SATMO` to show models:

```bash
ai-blame transcript list --columns SATMO

# Then identify sessions that used specific model (e.g., claude-opus-4-5)
```

### Show Latest N Sessions

```bash
# Show last 5
ai-blame transcript list -n 5

# Show last 50
ai-blame transcript list -n 50

# Show all
ai-blame transcript list -n 0
```

---

## Advanced Patterns

### Export Session as Documentation

Create a markdown record of what was done:

```bash
ai-blame transcript view 483c7d95 --format markdown --full > session-doc.md

# Share with team or archive
```

### Programmatic Access

Extract data as JSON for tooling:

```bash
ai-blame transcript list --format json | jq '.[0].session_id'

ai-blame transcript view 483c7d95 --format json | jq '.messages[0]'
```

### Compare Sessions

```bash
# List all with models
ai-blame transcript list -n 0 --columns SATMO

# View specific sessions to compare
ai-blame transcript view <session-1> --format markdown > s1.md
ai-blame transcript view <session-2> --format markdown > s2.md

# Diff
diff s1.md s2.md
```

---

## Interpreting Session Information

### Session ID
Unique identifier for the user-initiated session. Multiple trace files can share the same Session ID (subagents).

### Agent
The tool that ran: `claude-code`, `codex-cli`, etc.

### Start Time
When the session started (UTC).

### Messages
Total messages exchanged (user prompts + Claude responses).

### Files Touched
Number of files modified. Note: Subagent sessions may show 0 if they only read files.

### Models
AI models used. Helps distinguish:
- **claude-opus-4-5** — Main session (powerful model)
- **claude-haiku** — Subagent (faster, lighter)
- **gpt-4** — Copilot/Codex sessions

---

## Tips & Best Practices

### Tip 1: Start with Stats

Always begin with `ai-blame stats` to understand scope:

```bash
ai-blame stats
# Good: "Ah, 47 edits across 23 files"

ai-blame timeline
# Now: "Timeline shows creation on Dec 1, major edit Dec 15"

ai-blame transcript list
# Find: "3 sessions, mostly claude-code"
```

### Tip 2: Use Models to Identify Subagents

Subagents typically use different (faster) models:

```bash
ai-blame transcript list --columns SATMO
# Look for: Same Session ID, different models
# Opus = main, Haiku = subagents
```

### Tip 3: Export Before Archiving

Keep historical records:

```bash
ai-blame transcript view 483c7d95 --format markdown > archive/session-483c7d95.md
```

### Tip 4: Combine Filters

Get exactly what you need:

```bash
# YAML files only
ai-blame timeline --pattern ".yaml"

# Recent sessions only
ai-blame transcript list -n 5 --columns SATMO

# Specific directory
ai-blame stats --pattern "docs/"
```

---

## Troubleshooting

### No traces found

```bash
# Verify trace directory exists
ls ~/.claude/projects/

# Use explicit path
ai-blame stats --trace-dir /path/to/traces
```

### Session ID not found

```bash
# Try listing first
ai-blame transcript list

# Use more characters of ID
ai-blame transcript view 483c7d95-6b9c-46db

# Use full path
ai-blame transcript view ~/.claude/projects/.../483c7d95.jsonl
```

### Output truncated

```bash
# Use --full to see everything
ai-blame transcript view 483c7d95 --full

# Or export as markdown
ai-blame transcript view 483c7d95 --format markdown > full.md
```

---

## Related Topics

- **[Line-Level Analysis](blame-analysis.md)** — Line-by-line attribution in files
- **[Understanding Subagents](../explanation/transcripts-and-subagents.md)** — Deep dive into multi-agent sessions
- **[Provenance Annotation](annotation.md)** — Embedding what you discover
- **[Performance](performance.md)** — Optimizing trace exploration
- **[CLI Reference: stats](cli.md#ai-blame-stats)** — Syntax reference
- **[CLI Reference: timeline](cli.md#ai-blame-timeline)** — Syntax reference
- **[CLI Reference: transcript](cli.md#ai-blame-transcript-list)** — Syntax reference

---

## Next Steps

After exploring traces, you can:

1. **Annotate:** Use what you learned to add provenance with [Provenance Annotation](annotation.md)
2. **Analyze:** Dive deeper with [Line-Level Analysis](blame-analysis.md)
3. **Optimize:** Speed up with [Performance](performance.md)
4. **Understand:** Learn the system with [How It Works](../explanation/how-it-works.md)

Start with [Provenance Annotation](annotation.md) to take action on what you discovered.
