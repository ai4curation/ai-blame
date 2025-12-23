# Work with Test Data

This guide shows how to use `ai-blame` with trace data from different locations, useful for testing or working with multiple projects.

## The Problem

By default, `ai-blame` looks for traces based on your current working directory:

```
~/.claude/projects/-Users-you-current-project/
```

But sometimes you need to:

- Test with sample trace data
- Process traces from a different machine
- Work with traces stored in a non-standard location

## Using `--dir` and `--home`

The `--dir` and `--home` options let you specify where to look for traces:

```bash
# Look for traces as if you were in a different directory
ai-blame stats --dir /path/to/project

# Look for traces in a different home directory
ai-blame stats --dir /path/to/project --home /other/home
```

### How It Works

The trace directory is computed as:

```
$home/.claude/projects/<encoded-dir>/
```

Where `<encoded-dir>` is the directory path with `/` replaced by `-`.

**Example:**

```bash
ai-blame stats --dir /Users/alice/myproject --home /Users/alice
```

Looks for traces in:

```
/Users/alice/.claude/projects/-Users-alice-myproject/
```

## Testing with Sample Data

### Project Structure

Set up test data like this:

```
tests/data/
├── .claude/
│   └── projects/
│       └── -Users-me-tests-data/
│           ├── session1.jsonl
│           └── session2.jsonl
├── testdir1/
│   ├── foo.yaml
│   └── bar.txt
└── testdir2/
    └── config.yaml
```

!!! note
    The encoded folder name must match the absolute path of your test data directory.

### Running Against Test Data

```bash
# From the project root
ai-blame stats --dir tests/data --home tests/data

# Or with absolute paths
ai-blame stats \
  --dir /Users/me/project/tests/data \
  --home /Users/me/project/tests/data
```

### In Tests

```python
import pytest
from pathlib import Path
from typer.testing import CliRunner
from ai_blame.cli import app

runner = CliRunner()

TEST_DATA = Path(__file__).parent / "data"

def test_stats_with_test_data():
    result = runner.invoke(
        app,
        [
            "stats",
            "--dir", str(TEST_DATA.resolve()),
            "--home", str(TEST_DATA),
        ],
    )
    assert result.exit_code == 0
    assert "Trace files:" in result.output
```

## Using `--trace-dir` Directly

If you know the exact trace directory, use `--trace-dir`:

```bash
ai-blame stats --trace-dir ~/.claude/projects/-Users-alice-other-project/
```

This overrides `--dir` and `--home`.

## Priority Order

1. `--trace-dir` — Use this exact path
2. `--dir` + `--home` — Compute `$home/.claude/projects/<encoded-dir>/`
3. Default — Use `~/.claude/projects/<encoded-cwd>/`

## Rewriting Trace Paths

When copying traces between machines, file paths in the traces won't match. For testing, you may need to rewrite paths in the JSONL files:

```python
def rewrite_trace_file(src: Path, dest: Path, old_path: str, new_path: str):
    """Rewrite a JSONL trace file, replacing path references."""
    with open(src) as f:
        content = f.read()
    content = content.replace(old_path, new_path)
    with open(dest, "w") as f:
        f.write(content)
```

This is what the test suite does to make traces portable across different machines.

## Example: CI/CD Testing

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Test ai-blame
        run: |
          # Run against bundled test data
          ai-blame stats --dir tests/data --home tests/data
```
