# Command Index

Quick visual guide to all `ai-blame` commands. Choose your starting point below.

---

## Command Tree

```
ai-blame
├── Setup Commands
│   ├── init          — Create starter configuration
│   └── completions   — Generate shell tab-completion scripts
│
├── Discovery Commands
│   ├── stats         — Quick statistics on available traces
│   ├── timeline      — Chronological history of all edits
│   └── transcript    — Explore AI agent sessions
│       ├── list      — List all transcripts
│       └── view      — Display detailed transcript
│
├── Analysis Commands
│   └── blame         — Line-by-line attribution
│
└── Provenance Commands
    ├── report        — Preview what would be added (dry-run)
    └── annotate      — Apply provenance to files
```

---

## Task-to-Command Mapping

Find the right command for what you need to do:

| What You Want to Do | Command | Documentation |
|---------------------|---------|----------------|
| **Set up** a new project | `init` | [Setup & Configuration](setup.md) |
| **Enable tab completion** | `completions` | [Shell Completions](../how-to/shell-completions.md) |
| **Know what traces exist** | `stats` | [Trace Exploration](exploration.md#stats) |
| **See timeline of changes** | `timeline` | [Trace Exploration](exploration.md#timeline) |
| **List AI sessions** | `transcript list` | [Trace Exploration](exploration.md#transcript) |
| **Review AI conversation** | `transcript view` | [Trace Exploration](exploration.md#transcript-view) |
| **Find who edited each line** | `blame` | [Line-Level Analysis](blame-analysis.md) |
| **Preview provenance additions** | `report` | [Provenance Annotation](annotation.md#report) |
| **Add provenance to files** | `annotate` | [Provenance Annotation](annotation.md#annotate) |
| **Speed up processing** | `--no-cache` or `--rebuild-cache` | [Performance](performance.md) |
| **Filter by file type** | `--pattern` | [Trace Exploration](exploration.md) |

---

## Common Command Workflows

### Workflow 1: Quick Project Overview (5 minutes)

```bash
# Step 1: See what traces exist
ai-blame stats

# Step 2: Understand project history
ai-blame timeline

# Step 3: Spot-check a specific file
ai-blame blame src/main.rs --lines 1-20
```

**When to use:** New project, initial discovery, understanding scope
**Documentation:** [Trace Exploration](exploration.md)

### Workflow 2: Explore a Session (10 minutes)

```bash
# Step 1: List all transcripts with models
ai-blame transcript list --columns SATMO

# Step 2: View a specific session
ai-blame transcript view <session-id> --full --show-thinking
```

**When to use:** Understanding why specific decisions were made, reviewing AI work
**Documentation:** [Understanding Subagents](../explanation/transcripts-and-subagents.md)

### Workflow 3: Annotate Your Project (15 minutes)

```bash
# Step 1: Initialize config (first time only)
ai-blame init

# Step 2: Preview what will be added
ai-blame report --initial-and-recent

# Step 3: Dry-run the annotation
ai-blame annotate --initial-and-recent --dry-run

# Step 4: Apply to files
ai-blame annotate --initial-and-recent
```

**When to use:** Ready to embed provenance history in your codebase
**Documentation:** [Provenance Annotation](annotation.md)

### Workflow 4: Analyze Specific File (5 minutes)

```bash
# Step 1: See who edited what
ai-blame blame docs/config.yaml

# Step 2: Group by model
ai-blame blame docs/config.yaml --blocks

# Step 3: Focus on line range
ai-blame blame docs/config.yaml --lines 10-40
```

**When to use:** Debugging, code review, understanding specific changes
**Documentation:** [Line-Level Analysis](blame-analysis.md)

---

## Global Options

These work with all commands:

| Option | Description |
|--------|-------------|
| `--help` | Show help for any command |
| `--version` | Show version information |
| `-d, --dir <DIR>` | Target project directory (default: current directory) |
| `-t, --trace-dir <TRACE_DIR>` | Claude trace directory (auto-detected by default) |
| `--home <HOME>` | Home directory for trace lookup (default: `~`) |

---

## Reference Documentation

Each command has detailed documentation organized by functionality:

| Functionality | Commands | When Used |
|---------------|----------|-----------|
| **[Setup & Configuration](setup.md)** | `init` | First-time project setup |
| **[Trace Exploration](exploration.md)** | `stats`, `timeline`, `transcript` | Discovering and understanding traces |
| **[Provenance Annotation](annotation.md)** | `report`, `annotate` | Embedding provenance in files |
| **[Line-Level Analysis](blame-analysis.md)** | `blame` | Understanding line-by-line attribution |
| **[Performance](performance.md)** | Cache & filter flags | Optimizing command execution |
| **[CLI Syntax Reference](cli.md)** | All commands | Complete syntax and option details |

---

## Learning Paths

### Path 1: "I'm brand new"
1. Read: [What is ai-blame?](../index.md#what-is-ai-blame)
2. Do: [Quickstart](../tutorials/quickstart.md)
3. Learn: [Trace Exploration](exploration.md)

### Path 2: "I want to explore traces"
1. Start: [Trace Exploration](exploration.md)
2. Deep dive: [Understanding Subagents](../explanation/transcripts-and-subagents.md)
3. Advanced: [Trace Format](../explanation/trace-format.md)

### Path 3: "I want to annotate my project"
1. Start: [Provenance Annotation](annotation.md)
2. Configure: [Configuration Guide](../how-to/configuration.md)
3. Advanced: [Performance](performance.md)

### Path 4: "I want to analyze changes"
1. Start: [Line-Level Analysis](blame-analysis.md)
2. Learn: [How It Works](../explanation/how-it-works.md)
3. Reference: [CLI Syntax](cli.md)

---

## Need Help?

- **Questions?** Check [FAQs](../faqs.md)
- **Lost?** Start with the [Quickstart](../tutorials/quickstart.md)
- **Technical details?** See [How It Works](../explanation/how-it-works.md)
- **Want to contribute?** See [Contributing](https://github.com/ai4curation/ai-blame/blob/main/CONTRIBUTING.md)
