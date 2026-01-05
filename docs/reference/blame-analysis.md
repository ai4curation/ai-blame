# Line-Level Analysis

Understand which AI model contributed each line of a file using line-by-line blame.

---

## Conceptual Model

The `blame` command answers: "Who wrote this line? Which model/agent?"

Like `git blame`, but for AI-assisted edits:

```
     1  claude-opus-4-5    def process_data(input: str):
     2  claude-opus-4-5        items = parse_input(input)
     3  claude-opus-4-5
     4  claude-3-5-sonnet      return items
```

Each line shows which AI model created or last modified it.

---

## The `blame` Command

### What It Does

Shows git-blame-style line-by-line attribution for a file. Each line displays:

- **Line number**
- **Model** that created/last-edited this line
- **Line content**

### Command Syntax

```bash
ai-blame blame [OPTIONS] <FILE>
```

### Basic Usage

**Blame an entire file:**

```bash
ai-blame blame src/main.rs
```

**Focus on specific lines:**

```bash
ai-blame blame src/main.rs --lines 10-20
```

**Group by model (blocks):**

```bash
ai-blame blame src/main.rs --blocks
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILE` | File to show blame for (path relative to cwd, or absolute) |

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--trace-dir <TRACE_DIR>` | `-t` | Auto | Claude trace directory |
| `--dir <DIR>` | `-d` | cwd | Target project directory |
| `--home <HOME>` | | `~` | Home directory for trace lookup |
| `--lines <N-M>` | | | Restrict output to a line range like `"10-20"` |
| `--blocks` | | False | Show block boundaries (consecutive same-model lines) |

---

## Understanding Blame Output

### Default Output (Line-by-Line)

```bash
$ ai-blame blame src/main.rs

     1  claude-opus-4-5    fn process_data(input: &str) -> Result<Vec<Item>> {
     2  claude-opus-4-5        let items = parse_input(input)?;
     3  claude-opus-4-5
     4  claude-opus-4-5        items.iter()
     5  claude-opus-4-5            .filter(|item| item.is_valid())
     6  claude-3-5-sonnet       .map(|item| transform(item))
     7  claude-3-5-sonnet       .collect()
     8  claude-opus-4-5    }
```

**Interpretation:**
- Lines 1-5: Created by `claude-opus-4-5`
- Lines 6-7: Last modified by `claude-3-5-sonnet`
- Line 8: `claude-opus-4-5`

**Why line 6 is different:** Maybe `claude-3-5-sonnet` optimized the `.map()` in a later session.

### Block Mode Output

Group consecutive lines from the same model:

```bash
$ ai-blame blame src/main.rs --blocks

Block 1: claude-opus-4-5 (lines 1-5)
     1  fn process_data(input: &str) -> Result<Vec<Item>> {
     2      let items = parse_input(input)?;
     3
     4      items.iter()
     5          .filter(|item| item.is_valid())

Block 2: claude-3-5-sonnet (lines 6-7)
     6          .map(|item| transform(item))
     7          .collect()

Block 3: claude-opus-4-5 (line 8)
     8  }
```

**Better for:** Seeing at a glance which models worked on different sections.

---

## Common Patterns & Interpretation

### Pattern 1: Single Model Throughout

```
     1  claude-opus-4-5    def function():
     2  claude-opus-4-5        result = calculate()
     3  claude-opus-4-5        return result
```

**Interpretation:** One session, one model created entire function.

### Pattern 2: Mixed Models (Edits)

```
     1  claude-opus-4-5    def function():
     2  claude-3-5-sonnet      result = calculate()
     3  claude-opus-4-5        return result
```

**Interpretation:** Opus created it, Sonnet optimized line 2 later, Opus added return.

**Why?** Different sessions, different models. Maybe one session was exploratory (Opus), another optimized (Sonnet).

### Pattern 3: Blank Lines

```
     5  claude-opus-4-5
     6  claude-opus-4-5
     7  claude-3-5-sonnet
```

Blank lines are attributed to whoever created them.

### Pattern 4: Comments vs Code

```
    20  claude-opus-4-5    # This function processes data
    21  claude-opus-4-5    def process(data):
    22  claude-3-5-sonnet      return data * 2
```

**Interpretation:** Opus wrote comment + function signature, Sonnet optimized implementation.

---

## Blame Workflows

### Workflow 1: Understand a File (2 minutes)

Quick overview of who did what:

```bash
# See who touched which parts
ai-blame blame config.yaml --blocks

# Output shows: blocks of contributions by model
# Tells you: "Opus created structure, Sonnet tweaked values"
```

### Workflow 2: Investigate Specific Lines (1 minute)

Focus on a problem area:

```bash
# Line 42 looks wrong. Who wrote it?
ai-blame blame src/main.rs --lines 40-45

# Output shows model that created line 42
# Follow up: `ai-blame transcript view <session-id>`
```

### Workflow 3: Code Review Preparation

Before reviewing AI-contributed code:

```bash
# Understand contribution distribution
ai-blame blame docs/config.yaml --blocks

# See which models contributed
# Plan review: "Sonnet handled structure, Opus handled values"
```

### Workflow 4: Debugging Mixed-Model Files

When multiple models edited the same file:

```bash
# Show all contributions
ai-blame blame src/script.py

# Identify sections to review
# Think: "Why did Sonnet change lines 15-20 after Opus created 1-14?"

# For details, explore transcript
ai-blame transcript list --columns SATMO
ai-blame transcript view <session-that-modified-15-20>
```

---

## Advanced Usage

### Focus on Line Range

Show only lines 10-40:

```bash
ai-blame blame src/main.rs --lines 10-40
```

Useful for:
- Large files (focus on problem area)
- Code review (check specific function)
- Debugging (investigate specific lines)

### Range Syntax

```bash
# Single line
ai-blame blame file.py --lines 5-5

# Range
ai-blame blame file.py --lines 1-50

# From start to line
ai-blame blame file.py --lines 1-100

# Invalid (shows error)
ai-blame blame file.py --lines 20-10  # Error: end < start
```

### Interpret Model Names

Model names in blame output follow this pattern:

- **`claude-opus-4-5-20251101`** → claude-opus-4-5 (displayed short)
- **`claude-3-5-sonnet-20241022`** → claude-3-5-sonnet (short form)
- **`claude-haiku-3-20240307`** → claude-haiku (short form)
- **`gpt-4-turbo-20240409`** → gpt-4-turbo (Copilot/Codex)

Display shows the short form for readability.

---

## Combining Blame with Other Commands

### Find Who Edited a Line, Then Explore

```bash
# Step 1: See line 42 is by claude-3-5-sonnet
ai-blame blame src/main.rs --lines 42-42

# Step 2: List sessions that used that model
ai-blame transcript list --columns SATMO

# Step 3: View specific session for context
ai-blame transcript view <session-id> --full --show-thinking
```

### Timeline + Blame (Multi-Step Story)

```bash
# Step 1: See when file was created/edited
ai-blame timeline --pattern "config.yaml"
# Output: Created Dec 1 by claude-opus-4-5
#         Edited Dec 15 by claude-3-5-sonnet

# Step 2: Check blame to see which lines changed
ai-blame blame config.yaml --blocks
# Output: Shows lines edited in Dec 15 session

# Step 3: View Dec 15 session for context
ai-blame transcript list --columns SATMO
ai-blame transcript view <dec-15-session>
```

### Report + Blame (Full Provenance)

```bash
# Step 1: See edit summary
ai-blame report config.yaml
# Output: Created Dec 1, Edited Dec 15

# Step 2: See line-level details
ai-blame blame config.yaml
# Output: Which lines by which model

# Step 3: Annotate (embed provenance)
ai-blame annotate config.yaml
# Creates: config.edit_history.yaml
```

---

## Edge Cases

### Case 1: File Not in Traces

```bash
$ ai-blame blame unknown.py

Error: File 'unknown.py' not found in traces.
```

**Solution:** Verify file was actually edited by Claude:
```bash
ai-blame timeline --pattern "unknown"
ai-blame stats --pattern "unknown"
```

### Case 2: Entirely Unmodified File

If a file appears in project but no traces mention it:

```bash
# File exists but blame finds nothing
ai-blame blame docs/README.md

Error: No edit history found for docs/README.md
```

**Why?** Traces don't mention this file (maybe only created manually, not via Claude).

### Case 3: Large File with Many Models

For files with complex history:

```bash
# First: see block structure
ai-blame blame app.py --blocks

# Then: focus on one section
ai-blame blame app.py --lines 100-150
```

### Case 4: Single-Line Range

```bash
# Check who created line 42
ai-blame blame config.yaml --lines 42-42

# Output: Shows only line 42 with model
```

---

## Interpreting Models in Blame

### Why Different Models?

Files can have multiple models when:

1. **Multiple sessions:** First session used Opus, second used Sonnet
2. **Edits across time:** Early versions Opus, later optimizations Sonnet
3. **Subagents:** Main session Opus, subagent optimization Haiku
4. **Copilot integration:** GitHub Copilot uses different models than Claude

### What to Look For

**Consistent single model:** Likely single session, iterative refinement.

**Multiple models:** Different sessions, different agents, or tools. Review sessions to understand context.

**Model downgrade (Opus→Haiku):** Likely subagent work (fast tasks) vs main session (complex planning).

**Model upgrade (Haiku→Opus):** Unlikely but indicates intervention for difficult section.

---

## Blame Limitations

### What Blame Can't Tell You

- **Why** a line was written (context is in transcript)
- **When exactly** (only which session, not timestamp)
- **What was changed in later edits** (shows current version)
- **Previous versions** (only shows current file state)

### For These, Use Other Commands

```bash
# For context/reasoning: transcript view
ai-blame transcript view <session-id> --show-thinking

# For timestamps: timeline
ai-blame timeline

# For edit history: report
ai-blame report <file>

# For previous versions: git log (if version-controlled)
git log <file>
```

---

## Tips & Best Practices

### Tip 1: Use Blocks for Overview

Always start with `--blocks` for large files:

```bash
ai-blame blame large_file.py --blocks
# Easier to see: "First half Opus, second half Sonnet"
```

### Tip 2: Combine with Transcript

When you see unexpected model:

```bash
ai-blame blame config.yaml
# Notice: line 15 is claude-3-5-sonnet (unexpected)

ai-blame transcript list --columns SATMO
# Find session that used Sonnet

ai-blame transcript view <session-id> --full
# Understand: why Sonnet? What was the task?
```

### Tip 3: Focus on Lines Before Review

Don't blame entire file if it's huge:

```bash
# Bad: huge output
ai-blame blame app.py

# Good: focused
ai-blame blame app.py --lines 100-150
ai-blame blame app.py --blocks
```

### Tip 4: Verify Models Match Expectations

Before trusting blame output:

```bash
# Check if expected models created traces
ai-blame transcript list --columns SATMO

# Verify models match your expectation
# (e.g., "Claude Code should use claude-opus-4-5")
```

---

## Troubleshooting

### "File not found in traces"

```bash
# Verify file exists locally
ls config.yaml

# Check if traces mention it
ai-blame timeline --pattern "config.yaml"

# Verify correct directory
ai-blame stats
```

### "No edit history found"

File exists locally but wasn't edited in traces:

```bash
# Was it created before traces started?
git log config.yaml  # Check git history

# Was it edited by Claude?
ai-blame timeline
ai-blame stats
```

### Output looks truncated

Shouldn't happen, but if it does:

```bash
# Try with explicit line range
ai-blame blame config.yaml --lines 1-100

# Force rebuild
ai-blame blame config.yaml --rebuild-cache
```

---

## Related Topics

- **[Trace Exploration](exploration.md)** — Understanding available traces and sessions
- **[Provenance Annotation](annotation.md)** — Embedding blame info in your files
- **[How It Works](../explanation/how-it-works.md)** — How blame is computed
- **[CLI Reference: blame](cli.md#ai-blame-blame)** — Complete syntax reference

---

## Next Steps

After analyzing blame:

1. **Annotate:** Embed findings in files with [Provenance Annotation](annotation.md)
2. **Explore:** Understand context with [Trace Exploration](exploration.md)
3. **Understand:** Learn the system with [How It Works](../explanation/how-it-works.md)

For specific session details, use `ai-blame transcript view <session-id>` to see reasoning and decisions.
