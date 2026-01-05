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
  - pattern: "package-lock.json"
    policy: skip
```

---

## Related Topics

- **[Setup & Configuration](../reference/setup.md)** — How to initialize and configure `.ai-blame.yaml`
- **[Configuration Guide](configuration.md)** — Detailed configuration options
- **[Provenance Annotation](../reference/annotation.md)** — How different output policies work
- **[CLI Reference](../reference/cli.md)** — Command syntax for all commands


