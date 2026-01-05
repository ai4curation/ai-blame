# Configuration File Reference

Complete reference for the `.ai-blame.yaml` configuration file.

## File Location

The configuration file should be named `.ai-blame.yaml` and placed in your project root. `ai-blame` searches upward from the current directory to find it.

You can also specify a config file explicitly:

```bash
ai-blame annotate --config /path/to/.ai-blame.yaml --dry-run
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
| `{ext}` | Extension (without dot) | `main.py` | `py` |
| `{dir}` | Parent directory | `src/main.py` | `src` |

### `comment_syntax`

Comment syntax for `comment` policy.

| Value | Format | Languages |
|-------|--------|-----------|
| `hash` | `# comment` | Python, Ruby, Shell, YAML |
| `slash` | `// comment` | JavaScript, TypeScript, Go, Rust |
| `html` | `<!-- comment -->` | HTML, XML, Markdown |


