# Related Tools

Several tools address the challenge of tracking AI contributions to codebases. This document compares different approaches and explains where `ai-blame` fits in the landscape.

## The Provenance Problem

As AI agents become routine collaborators in software development and knowledge curation, we face a new challenge: **attribution**. Traditional version control tells us *who committed* changes, but in an AI-assisted workflow, the human commits changes that an AI actually wrote.

Different tools take different approaches to solving this.

## git-ai

**Website**: [usegitai.com](https://usegitai.com/docs/how-git-ai-works)

[git-ai](https://usegitai.com) takes a git-native approach to AI attribution. Rather than modifying files, it extends git's metadata system.

### How It Works

1. **Checkpoints**: As you code with AI tools (Cursor, Claude Code, Copilot), git-ai creates temporary records in `.git/ai` marking which changes are AI-generated vs human-authored.

2. **Authorship Logs**: At commit time, checkpoints become optimized logs mapping line ranges to their AI origin, stored as [git notes](https://git-scm.com/docs/git-notes).

3. **Blame Integration**: Overlays AI authorship onto standard `git blame`, showing not just who last modified a line, but whether it originated from an AI.

### Key Strengths

- **Line-level granularity**: Tracks individual lines, not just files
- **Survives git operations**: Rebases, merges, and cherry-picks preserve attribution
- **Vendor neutral**: Supports multiple AI tools
- **Non-invasive**: Metadata stays in `.git/`, files remain unchanged

### Comparison with ai-blame

| Aspect | git-ai | ai-blame |
|--------|--------|----------|
| **Granularity** | Line-level | File-level |
| **Storage** | Git notes (`.git/`) | Embedded in files |
| **Timing** | Real-time during coding | Post-hoc extraction |
| **Portability** | Via git clone | Files carry their history |
| **Use case** | Development workflows | Knowledge bases, structured data |

## When to Use Which

### Use git-ai when:

- You need line-level attribution in code
- You want attribution to survive complex git workflows
- You prefer keeping files unchanged
- You're working in a typical software development context

### Use ai-blame when:

- You're curating knowledge bases or structured data (YAML, JSON)
- You want provenance embedded directly in files
- Files are distributed outside git (exports, APIs, publications)
- You need to extract provenance retroactively from existing traces
- You want history visible without special tooling

## Complementary Usage

The tools can complement each other:

- Use **git-ai** for your source code during development
- Use **ai-blame** for knowledge base files that get published or distributed

For example, in a biomedical ontology project:

```
project/
├── src/                    # git-ai tracks code
│   └── validators.py
└── kb/                     # ai-blame embeds history
    └── diseases/
        └── Asthma.yaml     # Contains edit_history
```

## Other Approaches

### Manual Attribution

Some teams add comments or commit message conventions:

```python
# AI-generated: Claude, 2025-01-15
def calculate_statistics(data):
    ...
```

**Pros**: Simple, no tooling required
**Cons**: Manual, inconsistent, doesn't scale

### AI Watermarking

Some AI providers embed invisible watermarks in generated text.

**Pros**: Automatic, detectable
**Cons**: Can be stripped, not structured, privacy concerns

### Trace Logging

Tools like [LangSmith](https://smith.langchain.com/) log AI interactions for debugging and monitoring.

**Pros**: Rich context, debugging value
**Cons**: Separate from files, requires infrastructure

## The Future

As AI assistance becomes ubiquitous, we expect:

- **Standardization**: Common formats for AI provenance metadata
- **Integration**: IDEs and git hosts surfacing AI attribution natively
- **Regulation**: Requirements for AI disclosure in certain domains

Both git-ai and ai-blame represent early attempts to build this infrastructure. The right tool depends on your workflow and what you're building.
