# Configure Output Policies

This guide shows how to configure `ai-blame` to handle different file types with different output strategies.

## The Configuration File

Create a `.ai-blame.yaml` file in your project root:

```yaml
# .ai-blame.yaml

defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  - pattern: "*.yaml"
    policy: append

  - pattern: "*.yml"
    policy: append

  - pattern: "*.json"
    policy: append
    format: json

  - pattern: "*.py"
    policy: comment
    comment_syntax: hash

  - pattern: "*.md"
    policy: skip
```

## Output Policies

### `append` — Add to the File

Best for structured data files. Adds a `edit_history` key directly:

```yaml
rules:
  - pattern: "*.yaml"
    policy: append
```

For JSON files, specify the format:

```yaml
rules:
  - pattern: "*.json"
    policy: append
    format: json
```

### `sidecar` — Write to Companion File

Creates a separate history file alongside the original:

```yaml
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"
```

Given `src/main.py`, this creates `src/main.history.yaml`.

**Sidecar pattern variables:**

| Variable | Description | Example |
|----------|-------------|---------|
| `{name}` | Full filename | `main.py` |
| `{stem}` | Filename without extension | `main` |
| `{ext}` | Extension with dot | `.py` |
| `{dir}` | Parent directory | `src` |

**Examples:**

```yaml
# foo.py → foo.history.yaml
sidecar_pattern: "{stem}.history.yaml"

# foo.py → foo.py.history.yaml
sidecar_pattern: "{name}.history.yaml"

# foo.py → .history/foo.py.yaml
sidecar_pattern: ".history/{name}.yaml"
```

### `comment` — Embed as Comments

Adds history as a comment block at the end of the file:

```yaml
rules:
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash

  - pattern: "*.js"
    policy: comment
    comment_syntax: slash

  - pattern: "*.html"
    policy: comment
    comment_syntax: html
```

**Comment syntaxes:**

| Syntax | Format |
|--------|--------|
| `hash` | `# comment` |
| `slash` | `// comment` |
| `html` | `<!-- comment -->` |

Result in a Python file:

```python
# ... your code ...

# --- edit_history ---
# - timestamp: '2025-12-01T08:03:42+00:00'
#   model: claude-opus-4-5-20251101
#   action: CREATED
# --- end edit_history ---
```

### `skip` — Ignore Files

Don't process matching files:

```yaml
rules:
  - pattern: "*.md"
    policy: skip

  - pattern: "tests/**"
    policy: skip
```

## Rule Matching

Rules are evaluated in order — **first match wins**.

```yaml
rules:
  # Specific rule first
  - pattern: "config/*.yaml"
    policy: append

  # Then general rule
  - pattern: "*.yaml"
    policy: sidecar
```

### Pattern Syntax

- Simple glob: `*.yaml`, `*.py`
- Path patterns: `src/**/*.py`, `kb/*.yaml`
- Filename patterns: `config.yaml`, `README.md`

For patterns without `/`, only the filename is matched. For patterns with `/` or `**`, the full path is matched.

## Specifying Config File

```bash
# Auto-find (walks up from cwd)
ai-blame mine

# Explicit path
ai-blame mine --config /path/to/.ai-blame.yaml
```

## Example Configurations

### Knowledge Base Project

```yaml
# Append history to knowledge files, skip everything else
defaults:
  policy: skip

rules:
  - pattern: "kb/**/*.yaml"
    policy: append
```

### Python Library

```yaml
# Sidecar for code, append for configs
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "pyproject.toml"
    policy: skip
  - pattern: "tests/**"
    policy: skip
```

### Mixed Project

```yaml
defaults:
  policy: sidecar
  sidecar_pattern: ".history/{name}.yaml"

rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.json"
    policy: append
    format: json
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash
  - pattern: "*.js"
    policy: comment
    comment_syntax: slash
  - pattern: "docs/**"
    policy: skip
```
