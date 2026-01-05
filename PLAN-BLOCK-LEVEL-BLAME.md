# Plan: Block/Line-Level Blame

## Overview

Implement line-level and block-level blame functionality similar to `git blame`, but for AI-assisted edits. This will allow users to see which AI model modified specific lines or blocks of code, with timestamp and context information.

## Current State

ai-blame.rs currently:
- Extracts file-level provenance from AI agent execution traces
- Supports multiple output modes (append, sidecar, comment)
- Tracks creation and modification events at the file level
- Stores edit history with timestamp, model, and action metadata

## Goals

1. **Line-level tracking**: Associate each line with the AI model that last modified it
2. **Block-level tracking**: Group related changes into logical blocks
3. **Interactive display**: Provide a `git blame`-like interface showing provenance alongside code
4. **Historical view**: Show the evolution of specific lines/blocks over time

## Refined MVP (Trace-driven)

The current Claude Code trace payload already contains enough information for a useful MVP:

- `toolUseResult.oldString` / `toolUseResult.newString` (exact replacement strings)
- `toolUseResult.type == "create"` with `toolUseResult.content` (full file content on create)
- `toolUseResult.structuredPatch` (sometimes present; may include unified diff-style hunks)

**Key implication**: we can implement blame without adding heavy dependencies (no SQLite, rope, or tree-sitter) by using a *reverse-apply* strategy over `oldString/newString`.

### MVP algorithm: reverse-apply blame (git-blame style)

Given current file content (from disk) and a time-ordered list of edits from traces:

1. Start with current file lines; mark all lines as unassigned.
2. Iterate edits **from newest → oldest**.
3. For each edit:
   - If it is a `create`, assign any still-unassigned lines to that event (then stop).
   - If it is a replace edit, locate the edit’s `newString` in the current working string.
     - If found, assign the corresponding line span to that event **only where still unassigned**.
     - Reverse-apply by replacing that occurrence of `newString` with `oldString`.
4. The result is a per-line “last AI event that introduced this exact text”.

This yields a practical `git blame`-like attribution for the *current working tree*.

### MVP block-level grouping

Block-level blame can be derived from line-level blame via a simple, effective rule:

- **Block** = maximal consecutive run of lines attributed to the same event (same timestamp/model/session).

This gives stable, explainable block boundaries without needing syntax parsing.

### Limitations (explicitly accepted for MVP)

- If `newString` occurs multiple times, we choose the “best” match (prefer matches near any hunk line number in `structuredPatch` when available; otherwise pick the first).
- If we can’t locate `newString`, we skip that reverse-apply step; attribution may be `unknown` for some lines.
- Deletions (where `newString` is empty) don’t affect blame for existing lines; we may skip reversing those edits.

## Architecture (MVP-first)

### Data Model Extensions

#### Current Edit History Structure
```yaml
edit_history:
  - timestamp: "2025-12-01T08:03:42+00:00"
    model: claude-opus-4-5-20251101
    agent_tool: claude-code
    action: CREATED
```

#### Proposed Line/Block Structures (sidecar-oriented)

For MVP, prefer storing line/blame data in sidecars (to avoid modifying code files) and keep file-level `edit_history` as-is.

```yaml
line_provenance:
  - line_range: [1, 5]
    timestamp: "2025-12-01T08:03:42+00:00"
    model: claude-opus-4-5-20251101
    agent_tool: claude-code
    action: CREATED
  - line_range: [10, 15]
    timestamp: "2025-12-02T14:22:10+00:00"
    model: claude-sonnet-4-20250514
    agent_tool: claude-code
    action: MODIFIED
    # Optional debug fields for matching/uncertainty
    match_quality: exact | ambiguous | missing
    structured_patch: "@@ -12,2 +12,3 @@ ..."
```

### New Data Structures (MVP)

```rust
// src/models.rs additions

/// Represents a line or range of lines in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

/// Line-level provenance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineProvenance {
    pub line_range: LineRange,
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub agent_tool: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_patch: Option<String>,
}

/// Block-level grouping of related changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockProvenance {
    pub block_id: String,
    pub line_ranges: Vec<LineRange>,
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub agent_tool: String,
    pub description: Option<String>,
    pub related_files: Vec<String>,
}
```

## Implementation Phases (Revised)

### Phase 1: Extract richer edit payload (MVP)

**Goal**: Capture enough data from traces to compute blame reliably.

**Tasks**:
1. Extend trace parsing to capture:
   - `oldString`, `newString`, `structuredPatch`
   - `content` for creates
2. Preserve backwards compatibility for existing `annotate` behavior.

**Testing**:
- Unit tests proving we still parse timestamps/models and now also capture replacement payload.

### Phase 2: Blame computation + display (MVP)

**Goal**: Provide `git blame`-like output for current working tree.

**Tasks**:
1. Implement reverse-apply blame computation.
2. Implement block grouping from consecutive line attributions.
3. Add CLI: `ai-blame show <file> [--lines A-B] [--blocks]`.

### Phase 3: Optional persistence and integration (post-MVP)

**Tasks**:
1. Extend `annotate` to optionally write `line_provenance`/`block_provenance` to sidecars (config-driven).
2. Add richer matching/uncertainty reporting.
3. Add history/diff style commands once we have a stable on-disk schema.

**Output Format**:
```
model           timestamp         line | code
-----------------------------------------------------------------------------
claude-opus-4   2025-12-01 08:03   1   | use ai_blame::cli;
claude-opus-4   2025-12-01 08:03   2   |
claude-sonnet-4 2025-12-02 14:22   3   | fn main() {
claude-sonnet-4 2025-12-02 14:22   4   |     if let Err(e) = cli::run() {
claude-opus-4   2025-12-01 08:03   5   |         eprintln!("Error: {}", e);
```

## Technical Challenges

### Challenge 1: Line Number Stability
**Problem**: Line numbers change as code is edited
**Solution**: 
- Maintain line offset mappings for each edit
- Use content hashing to identify moved/renamed lines
- Store historical line mappings for accurate blame

### Challenge 2: Merge Conflicts
**Problem**: Multiple AI edits to same lines
**Solution**:
- Track conflict detection and resolution
- Show all contributors when lines conflict
- Maintain provenance through conflict resolution

### Challenge 3: Performance
**Problem**: Large files and many traces slow down analysis
**Solution**:
- Incremental processing of new traces only
- Cache parsed blame data
- Use efficient data structures (interval trees for line ranges)
- Lazy loading of detailed provenance

### Challenge 4: Accuracy
**Problem**: AI traces may not capture all changes accurately
**Solution**:
- Combine with actual file diffs for verification
- Detect and mark uncertain attributions
- Allow manual corrections/annotations

## Dependencies

### MVP dependencies

No new crates required for the MVP; implement reverse-apply matching using `String` operations.

## Testing Strategy

### Unit Tests
- Diff parsing edge cases (empty lines, large changes, binary files)
- Line mapping with insertions, deletions, and moves
- Blame query API with various filters
- Block detection algorithm accuracy

### Integration Tests
- Process real Claude Code traces
- Generate blame output for test projects
- Verify blame accuracy against known edits
- Test backward compatibility with file-level system

### Performance Tests
- Benchmark blame generation for large files (10k+ lines)
- Test database query performance with many traces
- Memory usage for large codebases

## Documentation

### User Documentation
- Update README.md with line-level blame examples
- Create tutorial: "Understanding Your AI Code History"
- Add cookbook: Common blame queries and analysis

### Developer Documentation
- Architecture overview of blame system
- Data format specifications
- API reference for blame queries
- Guide for extending to new agent types

## Success Metrics

1. **Accuracy**: >95% of lines correctly attributed to AI model
2. **Performance**: Blame generation <1s for files up to 1000 lines
3. **Usability**: Users can quickly understand which AI modified what
4. **Coverage**: Support for all file types ai-blame already handles

## Future Enhancements

### Post-MVP Features
1. **Web-based visualization**: Interactive blame viewer in browser
2. **IDE integration**: Show blame inline in VS Code, IntelliJ, etc.
3. **Blame statistics**: Dashboard showing AI contribution metrics
4. **Blame export**: Generate reports in various formats (HTML, PDF, CSV)
5. **Cross-file blame**: Track changes that span multiple files
6. **Semantic blame**: Attribute logical changes (feature additions, bug fixes) rather than just line changes

### Advanced Features
1. **Machine learning**: Improve attribution accuracy using ML models
2. **Natural language queries**: "Show me what Claude changed last week"
3. **Blame prediction**: Predict which AI model would make specific changes
4. **Collaboration tracking**: Track AI-human collaboration patterns

## Timeline Summary

This plan is intentionally MVP-first and should be implementable quickly:

- **Phase 1**: enrich trace extraction
- **Phase 2**: implement `show` (line + block display) using reverse-apply
- **Phase 3**: optional persistence/integration with `annotate`

## Open Questions

1. Should we store complete file history or just line-level diffs?
2. How to handle binary files or files with no line structure?
3. What level of detail to include in sidecar vs. inline comments?
4. How to represent uncertainty in attribution?
5. Should block detection be syntax-aware (using tree-sitter) or heuristic-based?

## References

- [git blame documentation](https://git-scm.com/docs/git-blame)
- [similar crate](https://docs.rs/similar/) - Diff algorithms
- [tree-sitter](https://tree-sitter.github.io/) - Syntax-aware parsing
- Current ai-blame.rs codebase structure
