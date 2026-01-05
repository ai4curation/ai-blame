# Custom Column Layouts and Aliases

The `ai-blame blame` command provides fine-grained control over output formatting with customizable column layouts and abbreviation aliases.

## Column Layout DSL

Use the `--columns` flag with a mini-DSL to specify which columns to display and their order:

```bash
ai-blame blame <file> --columns AMTLC
```

### Available Column Specifiers

| Letter | Column | Description |
|--------|--------|-------------|
| `A` | Agent | Agent tool and version (e.g., `claude-code@1.0.0`) |
| `M` | Model | AI model name (e.g., `claude-3-opus`) |
| `T` | Timestamp | When the change was made |
| `L` | Line | Line number |
| `C` | Code | Source code content |

### Examples

```bash
# Show only model, timestamp, line, and code (default when --show-agent is false)
ai-blame blame src/main.rs --columns MTLC

# Show agent, model, and code columns only
ai-blame blame src/main.rs --columns AMC

# Minimal output: just line numbers and code
ai-blame blame src/main.rs --columns LC

# Full output with custom order
ai-blame blame src/main.rs --columns CATML
```

## Abbreviation Aliases

Long agent and model names can make output cluttered. Use alias flags to define shorter abbreviations:

### Agent Aliases

```bash
ai-blame blame <file> --agent-alias claude-code=CC --agent-alias cursor=CR
```

### Model Aliases  

```bash
ai-blame blame <file> --model-alias claude-3-opus=opus --model-alias gpt-4o=gpt4o
```

### Combined Example

```bash
ai-blame blame src/main.rs \
  --columns AMTLC \
  --agent-alias claude-code=CC \
  --agent-alias cursor=CR \
  --model-alias claude-3-opus=opus-4.5 \
  --model-alias gpt-4o=gpt4o
```

This produces compact output like:
```
Agent        | Model     | Timestamp        | Line | Code
-------------|-----------|------------------|------|------------------
CC@1.0.0     | opus-4.5  | 2025-12-01 09:00 |   1  | fn main() {
CR@2.1.0     | gpt4o     | 2025-12-01 10:15 |   2  |     println!("Hello");
```

## Relationship to --show-agent

When using the legacy `--show-agent` flag:

- **Without `--columns`**: `--show-agent` controls whether the Agent column is included in the default layout
- **With `--columns`**: `--show-agent` is ignored; explicitly include `A` in your column spec to show agent information

```bash
# These are equivalent:
ai-blame blame file.rs --show-agent
ai-blame blame file.rs --columns AMTLC

# These are equivalent:
ai-blame blame file.rs 
ai-blame blame file.rs --columns MTLC
```

## Default Behavior

- **Default columns** (no `--columns` flag): `MTLC` (model, timestamp, line, code)
- **With `--show-agent`**: `AMTLC` (agent, model, timestamp, line, code)  
- **Case insensitive**: `amtlc` works the same as `AMTLC`
- **Whitespace ignored**: `A M T L C` works the same as `AMTLC`

## Error Handling

Invalid column specifiers produce helpful error messages:

```bash
$ ai-blame blame file.rs --columns AMXLC
Error: invalid column specifier 'X' in "AMXLC". Valid specifiers: A, M, T, L, C
```

## Performance Notes

- Column selection doesn't affect parsing performance—all data is extracted regardless
- Fewer columns produce cleaner, more readable output for large files
- Use aliases to keep output compact while preserving readability
---

## Related Topics

- **[Trace Exploration](../reference/exploration.md)** - `transcript list` command with column layouts
- **[Line-Level Analysis](../reference/blame-analysis.md)** - `blame` command with column options
- **[CLI Reference](../reference/cli.md)** - Complete syntax reference
- **[Command Index](../reference/index.md)** - Visual guide to all commands
