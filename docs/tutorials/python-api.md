# Python API

!!! warning
    The Rust version of ai-blame does **not** provide a Python API. It is a CLI-only tool distributed via PyPI for easy installation.

## Installation

The Rust version can be installed using Python package managers:

```bash
# Using pip
pip install ai-blame

# Using uv
uv add --dev ai-blame

# Using pipx (global install)
pipx install ai-blame
```

This installs the `ai-blame` CLI binary. No Rust toolchain required!

## Using the CLI from Python

If you need to use ai-blame from Python scripts, you can call the CLI using subprocess:

```python
import subprocess
import json
from pathlib import Path

# Run ai-blame report and capture JSON output
result = subprocess.run(
    ["ai-blame", "report", "--format", "json"],
    cwd="/path/to/project",
    capture_output=True,
    text=True,
    check=True
)

# Parse the output
report = json.loads(result.stdout)
for file_info in report:
    print(f"{file_info['path']}: {len(file_info['edits'])} edits")
```

### Running Other Commands

```python
import subprocess

# Run annotate with error handling
try:
    subprocess.run(
        ["ai-blame", "annotate", "--dry-run", "--pattern", "*.py"],
        cwd="/path/to/project",
        check=True
    )
except subprocess.CalledProcessError as e:
    print(f"ai-blame command failed with exit code {e.returncode}")
    # Handle the error appropriately

# Run blame on a specific file
try:
    result = subprocess.run(
        ["ai-blame", "blame", "myfile.py", "--blocks"],
        cwd="/path/to/project",
        capture_output=True,
        text=True,
        check=True
    )
    print(result.stdout)
except subprocess.CalledProcessError as e:
    print(f"Command failed: {e.stderr}")
except FileNotFoundError:
    print("ai-blame not found. Make sure it's installed: pip install ai-blame")
```

## Need a Python API?

If you have a use case that requires a proper Python API (beyond calling the CLI), please [open an issue](https://github.com/ai4curation/ai-blame/issues) describing your needs. We can consider adding PyO3 bindings if there's sufficient demand.

## Python Version (Deprecated)

The original Python implementation with a Python API is at [`ai4curation/ai-blame`](https://github.com/ai4curation/ai-blame). See the [migration guide](../../MIGRATION.md) for more information.

### Old Python API (for reference)

This API is from the deprecated Python version and is **not available** in the Rust version:

```python
# This ONLY works with the old Python version
from pathlib import Path
from ai_blame.extractor import extract_edit_history, convert_to_file_histories
from ai_blame.models import FilterConfig

# Define the trace directory (or use default)
trace_dir = Path.home() / ".claude" / "projects" / "-Users-you-my-project"

# Extract all edits
config = FilterConfig()
edits_by_file = extract_edit_history(trace_dir, config)

# See what files were edited
for file_path, edits in edits_by_file.items():
    print(f"{file_path}: {len(edits)} edits")
```


