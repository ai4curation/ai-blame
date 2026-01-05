# Migration Guide: Python ai-blame → Rust ai-blame

This guide helps users migrate from the [Python ai-blame](https://github.com/ai4curation/ai-blame) to the Rust version.

## TL;DR

The Rust version is a **drop-in replacement** with the same CLI interface:

```bash
# Old: Python version
pip install ai-blame

# New: Rust version (same command!)
pip install ai-blame
```

All CLI commands work identically. The main difference is that the Python API is not available in the Rust version.

## Why Migrate?

The Rust version offers several advantages:

- **10-100x faster** - Significantly faster trace parsing and file processing
- **Lower memory usage** - More efficient memory handling
- **Single binary** - No Python runtime or dependencies needed
- **Cross-platform** - Better support for Windows, macOS (including M1/M2), and Linux
- **Easier installation** - Pre-built wheels for all platforms
- **Better reliability** - Rust's type system catches bugs at compile time

## Installation

### Python Package Managers (Recommended)

The Rust version can be installed using the same Python package managers:

```bash
# Using pip
pip install ai-blame

# Using uv (faster alternative)
uv add --dev ai-blame

# Using pipx (global installation)
pipx install ai-blame
```

**Important**: When you install the Rust version, it will replace the Python version. Both use the same package name `ai-blame`.

### Other Installation Methods

```bash
# Using Cargo (Rust package manager)
cargo install ai-blame

# From source
git clone https://github.com/ai4curation/ai-blame
cd ai-blame
cargo install --path .
```

## CLI Compatibility

The Rust version maintains full CLI compatibility. All commands work the same:

### Commands

| Command | Status | Notes |
|---------|--------|-------|
| `ai-blame report` | ✅ Identical | Same output format |
| `ai-blame annotate` | ✅ Identical | Same behavior |
| `ai-blame stats` | ✅ Identical | Same statistics |
| `ai-blame blame` | ✅ Identical | Same line attribution |
| `ai-blame init` | ✅ Identical | Generates same config |

### Configuration Files

The `.ai-blame.yaml` configuration format is **100% compatible**. You don't need to change anything:

```yaml
# This config works with both versions
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.py"
    policy: comment
```

### Command-Line Options

All command-line flags work identically:

```bash
# These commands work the same in both versions
ai-blame report --initial-and-recent
ai-blame annotate --dry-run --pattern "*.py"
ai-blame stats --trace-dir ~/.claude/projects/my-project
ai-blame blame file.py --lines 10-20 --blocks
```

## Python API Migration

### Important: No Python API

The Rust version does **not** provide a Python API. It is a CLI-only tool.

If you were using the Python API programmatically:

```python
# This worked in the Python version
from ai_blame.extractor import extract_edit_history
edits = extract_edit_history(trace_dir, config)
```

You have three options:

1. **Use the CLI instead** (recommended for most cases):
   ```bash
   ai-blame report --format json > edits.json
   ```
   Then parse the JSON output in your Python script.

2. **Use subprocess to call the CLI**:
   ```python
   import subprocess
   import json
   
   result = subprocess.run(
       ["ai-blame", "report", "--format", "json"],
       capture_output=True,
       text=True
   )
   edits = json.loads(result.stdout)
   ```

3. **Request Python API support**:
   If you have a legitimate need for a Python API, please [open an issue](https://github.com/ai4curation/ai-blame/issues) describing your use case. We can consider adding Python bindings using PyO3 if there's sufficient demand.

## Feature Comparison

| Feature | Python Version | Rust Version |
|---------|---------------|--------------|
| CLI commands | ✅ | ✅ |
| Configuration files | ✅ | ✅ |
| Trace parsing | ✅ | ✅ (10-100x faster) |
| File annotation | ✅ | ✅ |
| Sidecar files | ✅ | ✅ |
| Block-level blame | ✅ | ✅ |
| Python API | ✅ | ❌ (CLI only) |
| Installation via pip/uv | ✅ | ✅ |
| Pre-built binaries | ❌ | ✅ |
| Cross-platform support | ⚠️ | ✅ (better) |

## Testing the Migration

### 1. Install the Rust version

```bash
# In a fresh virtual environment
python -m venv test-env
source test-env/bin/activate
pip install ai-blame
```

### 2. Verify it's the Rust version

```bash
ai-blame --version
# Should show: ai-blame 0.1.0 (or later)
```

### 3. Test with your existing config

```bash
cd your-project
ai-blame report --dry-run
```

### 4. Compare output

Run the same command with both versions and compare:

```bash
# Python version output
python-ai-blame report > python-output.txt

# Rust version output
ai-blame report > rust-output.txt

# Should be identical or very similar
diff python-output.txt rust-output.txt
```

## Troubleshooting

### "No module named 'ai_blame'"

If you're trying to import `ai_blame` in Python:

```python
from ai_blame import something  # This won't work
```

The Rust version doesn't provide a Python module. Use the CLI instead or subprocess to call it.

### "Command not found: ai-blame"

Make sure the installation directory is in your PATH:

```bash
# For pip installs
pip install --user ai-blame
export PATH="$HOME/.local/bin:$PATH"

# For pipx installs
pipx install ai-blame
# pipx handles PATH automatically
```

### Different output format

The Rust version aims for identical output, but there may be minor formatting differences. If you notice significant discrepancies, please [report an issue](https://github.com/ai4curation/ai-blame/issues).

### Performance seems the same

Make sure you're using the release build. The pre-built wheels from PyPI are already optimized. If you built from source:

```bash
cargo build --release
```

## Getting Help

If you encounter issues during migration:

1. Check the [README](https://github.com/ai4curation/ai-blame#readme) for updated documentation
2. Search [existing issues](https://github.com/ai4curation/ai-blame/issues)
3. Open a [new issue](https://github.com/ai4curation/ai-blame/issues/new) with:
   - What you were doing with the Python version
   - What's not working with the Rust version
   - Any error messages

## Timeline

The Python version at [ai4curation/ai-blame](https://github.com/ai4curation/ai-blame) will be maintained for a transition period but is expected to be archived once the Rust version is stable and widely adopted.

New features will be added to the Rust version only.

## Feedback

We want to make this migration as smooth as possible. If you have suggestions or run into problems, please [let us know](https://github.com/ai4curation/ai-blame/issues).
