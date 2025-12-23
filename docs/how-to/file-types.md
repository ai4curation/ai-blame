# Handle Different File Types

This guide shows recommended configurations for various file types.

## Structured Data Files

### YAML Files

YAML files work best with the `append` policy:

```yaml
rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.yml"
    policy: append
```

**Result:**

```yaml
# your-file.yaml
name: Example
version: 1.0

edit_history:
  - timestamp: "2025-12-01T08:03:42+00:00"
    model: claude-opus-4-5-20251101
    action: CREATED
```

### JSON Files

JSON files can also use `append` with the JSON format:

```yaml
rules:
  - pattern: "*.json"
    policy: append
    format: json
```

**Result:**

```json
{
  "name": "Example",
  "version": "1.0",
  "edit_history": [
    {
      "timestamp": "2025-12-01T08:03:42+00:00",
      "model": "claude-opus-4-5-20251101",
      "action": "CREATED"
    }
  ]
}
```

## Code Files

For code files, you have two options: sidecar files or embedded comments.

### Python

=== "Sidecar (Recommended)"

    ```yaml
    rules:
      - pattern: "*.py"
        policy: sidecar
        sidecar_pattern: "{stem}.history.yaml"
    ```

    Creates `main.history.yaml` alongside `main.py`.

=== "Comments"

    ```yaml
    rules:
      - pattern: "*.py"
        policy: comment
        comment_syntax: hash
    ```

    Embeds at end of file:
    ```python
    # --- edit_history ---
    # - timestamp: '2025-12-01T08:03:42+00:00'
    #   model: claude-opus-4-5
    #   action: CREATED
    # --- end edit_history ---
    ```

### JavaScript / TypeScript

=== "Sidecar"

    ```yaml
    rules:
      - pattern: "*.js"
        policy: sidecar
      - pattern: "*.ts"
        policy: sidecar
      - pattern: "*.tsx"
        policy: sidecar
    ```

=== "Comments"

    ```yaml
    rules:
      - pattern: "*.js"
        policy: comment
        comment_syntax: slash
      - pattern: "*.ts"
        policy: comment
        comment_syntax: slash
    ```

### HTML / XML

```yaml
rules:
  - pattern: "*.html"
    policy: comment
    comment_syntax: html
```

**Result:**

```html
<!-- edit_history
- timestamp: '2025-12-01T08:03:42+00:00'
  model: claude-opus-4-5
  action: CREATED
-->
```

## Documentation Files

### Markdown

Markdown files are often regenerated or are documentation that shouldn't include audit trails:

```yaml
rules:
  - pattern: "*.md"
    policy: skip
  - pattern: "docs/**"
    policy: skip
```

Or use sidecar if you want to track them:

```yaml
rules:
  - pattern: "*.md"
    policy: sidecar
```

## Configuration Files

### Skip Generated/Lock Files

```yaml
rules:
  - pattern: "*.lock"
    policy: skip
  - pattern: "uv.lock"
    policy: skip
  - pattern: "package-lock.json"
    policy: skip
```

### Track Project Configs

```yaml
rules:
  - pattern: "pyproject.toml"
    policy: sidecar
  - pattern: "package.json"
    policy: append
    format: json
```

## Complete Example

Here's a comprehensive configuration for a typical Python project:

```yaml
# .ai-blame.yaml

defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  # Structured data - append directly
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.yml"
    policy: append
  - pattern: "*.json"
    policy: append
    format: json

  # Skip documentation and tests
  - pattern: "*.md"
    policy: skip
  - pattern: "docs/**"
    policy: skip
  - pattern: "tests/**"
    policy: skip

  # Skip generated files
  - pattern: "*.lock"
    policy: skip
  - pattern: ".gitignore"
    policy: skip

  # Python code - use sidecar (falls through to default)
  # - pattern: "*.py"
  #   policy: sidecar  # Uses default
```

## Organizing Sidecar Files

### Same Directory (Default)

```yaml
sidecar_pattern: "{stem}.history.yaml"
```

```
src/
├── main.py
├── main.history.yaml
├── utils.py
└── utils.history.yaml
```

### Hidden Files

```yaml
sidecar_pattern: ".{stem}.history.yaml"
```

```
src/
├── main.py
├── .main.history.yaml
├── utils.py
└── .utils.history.yaml
```

### Separate Directory

```yaml
sidecar_pattern: ".history/{name}.yaml"
```

```
src/
├── main.py
├── utils.py
└── .history/
    ├── main.py.yaml
    └── utils.py.yaml
```

!!! tip
    Using a separate `.history/` directory keeps your source tree clean and makes it easy to `.gitignore` or include history files as needed.
