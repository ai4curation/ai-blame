# Configuration File Reference

Complete reference for the `.ai-blame.yaml` configuration file.

## File Location

The configuration file should be named `.ai-blame.yaml` and placed in your project root. `ai-blame` searches upward from the current directory to find it.

You can also specify a config file explicitly:

```bash
ai-blame mine --config /path/to/.ai-blame.yaml
```

## Schema

```yaml
# .ai-blame.yaml

defaults:
  policy: <policy>
  format: <format>
  sidecar_pattern: <pattern>
  comment_syntax: <syntax>

rules:
  - pattern: <glob>
    policy: <policy>
    format: <format>
    sidecar_pattern: <pattern>
    comment_syntax: <syntax>
```

## Top-Level Fields

### `defaults`

Default rule applied when no other rule matches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `policy` | string | `sidecar` | Output policy |
| `format` | string | `yaml` | Output format for `append` policy |
| `sidecar_pattern` | string | `{stem}.history.yaml` | Pattern for sidecar filenames |
| `comment_syntax` | string | | Comment syntax for `comment` policy |

### `rules`

List of rules evaluated in order. First match wins.

---

## Rule Fields

### `pattern`

**Required.** Glob pattern to match files.

| Pattern Type | Example | Matches |
|--------------|---------|---------|
| Extension | `*.yaml` | Any YAML file |
| Filename | `config.yaml` | Specific file |
| Directory | `kb/*.yaml` | YAML files in `kb/` |
| Recursive | `src/**/*.py` | Python files anywhere in `src/` |

For patterns without `/`, only the filename is matched. For patterns with `/` or `**`, the full relative path is matched.

### `policy`

**Required.** How to write curation history.

| Value | Description |
|-------|-------------|
| `append` | Add `edit_history` key directly to the file |
| `sidecar` | Write to a companion file |
| `comment` | Embed as comment block at end of file |
| `skip` | Don't process matching files |

### `format`

Format for `append` policy output.

| Value | Description |
|-------|-------------|
| `yaml` | YAML format (default) |
| `json` | JSON format |

### `sidecar_pattern`

Pattern for generating sidecar filenames. Only used with `sidecar` policy.

| Variable | Description | Example Input | Example Value |
|----------|-------------|---------------|---------------|
| `{name}` | Full filename | `main.py` | `main.py` |
| `{stem}` | Filename without extension | `main.py` | `main` |
| `{ext}` | Extension with dot | `main.py` | `.py` |
| `{dir}` | Parent directory | `src/main.py` | `src` |

**Examples:**

| Pattern | Input | Output |
|---------|-------|--------|
| `{stem}.history.yaml` | `main.py` | `main.history.yaml` |
| `{name}.history.yaml` | `main.py` | `main.py.history.yaml` |
| `.history/{name}.yaml` | `main.py` | `.history/main.py.yaml` |
| `.{stem}.history` | `main.py` | `.main.history` |

### `comment_syntax`

Comment syntax for `comment` policy.

| Value | Format | Languages |
|-------|--------|-----------|
| `hash` | `# comment` | Python, Ruby, Shell, YAML |
| `slash` | `// comment` | JavaScript, TypeScript, Go, Rust |
| `html` | `<!-- comment -->` | HTML, XML, Markdown |

---

## Complete Example

```yaml
# .ai-blame.yaml

# Default: use sidecar files
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  # Structured data files - append directly
  - pattern: "*.yaml"
    policy: append

  - pattern: "*.yml"
    policy: append

  - pattern: "*.json"
    policy: append
    format: json

  # Knowledge base files - always track
  - pattern: "kb/**/*.yaml"
    policy: append

  # Code files - embed as comments
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash

  - pattern: "*.js"
    policy: comment
    comment_syntax: slash

  - pattern: "*.ts"
    policy: comment
    comment_syntax: slash

  # HTML files
  - pattern: "*.html"
    policy: comment
    comment_syntax: html

  # Skip these files
  - pattern: "*.md"
    policy: skip

  - pattern: "*.lock"
    policy: skip

  - pattern: "tests/**"
    policy: skip

  - pattern: "docs/**"
    policy: skip

  - pattern: ".git/**"
    policy: skip
```

---

## Default Configuration

If no `.ai-blame.yaml` is found, the following defaults are used:

```yaml
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
```

This means:

- YAML files get history appended directly
- JSON files get history appended as JSON
- Everything else gets a sidecar file

---

## Validation

Invalid configurations will cause an error at runtime. Common issues:

- **Missing `pattern`** in a rule
- **Invalid `policy`** value (must be `append`, `sidecar`, `comment`, or `skip`)
- **Missing `comment_syntax`** when using `comment` policy
- **Invalid YAML syntax**
