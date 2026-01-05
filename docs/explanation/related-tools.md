# Related Tools

Several tools address the challenge of tracking AI contributions to codebases. This document compares different approaches and explains where `ai-blame` fits in the landscape.

## The Provenance Problem

As AI agents become routine collaborators in software development and knowledge curation, we face a new challenge: **attribution**. Traditional version control tells us *who committed* changes, but in an AI-assisted workflow, the human commits changes that an AI actually wrote.

Different tools take different approaches to solving this.

## git-ai

**Website**: [usegitai.com](https://usegitai.com/docs/how-git-ai-works)

[git-ai](https://usegitai.com) takes a git-native approach to AI attribution. Rather than modifying files, it extends git's metadata system.

### Comparison with ai-blame

| Aspect | git-ai | ai-blame |
|--------|--------|----------|
| **Granularity** | Line-level | File-level (plus `ai-blame blame` for a best-effort view) |
| **Storage** | Git notes (`.git/`) | Embedded in files |
| **Timing** | Real-time during coding | Post-hoc extraction |
| **Portability** | Via git clone | Files carry their history |
| **Use case** | Development workflows | Knowledge bases, structured data |


