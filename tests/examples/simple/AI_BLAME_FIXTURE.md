# ai-blame developer fixture: `tests/examples/simple`

This directory is a small example project used for manual testing of `ai-blame`
against a repo-local, reproducible set of Claude traces.

## Quickstart

From the repo root:

```bash
cd tests/examples/simple

# Use the repo-local fixture home (does not use your real ~/.claude)
ai-blame stats --home ../simple-home

# Show blame for an example file
ai-blame blame main.py --home ../simple-home

# Include agent/tool info alongside model (recommended)
ai-blame blame main.py --home ../simple-home --show-agent
```

## What’s in the fixture

- **Project files**: this directory (`tests/examples/simple/`)
- **Fixture home**: `tests/examples/simple-home/`
  - Contains `.claude/projects/<encoded-path>/...jsonl`
  - The JSONL `toolUseResult.filePath` entries are rewritten to point at the files
    under `tests/examples/simple/`


