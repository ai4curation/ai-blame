# Contributing to ai-blame

:+1: Thank you for taking the time to contribute!

## Table Of Contents

* [Code of Conduct](#code-of-conduct)
* [How to Contribute](#how-to-contribute)
  * [Reporting Issues](#reporting-issues)
  * [Adding New Agent Support](#adding-new-agent-support)
  * [Pull Requests](#pull-requests)
* [Development Setup](#development-setup)

## Code of Conduct

The ai-blame team strives to create a welcoming environment for all contributors.
Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Issues

Use the [Issue Tracker](https://github.com/ai4curation/ai-blame/issues) for:

- Bug reports
- Feature requests
- Questions about usage

### Adding New Agent Support

We welcome PRs to add support for additional AI coding agents! Currently planned:

- **OpenAI Codex** — Planned by maintainers

PRs welcome for:

- Cursor
- Aider
- GitHub Copilot
- Windsurf
- Other AI coding assistants

To add support for a new agent:

1. Study the trace format of the agent (where are traces stored? what format?)
2. Add a new parser in `src/ai_blame/extractor.py` or create a new module
3. Add test data in `tests/data/` with sample traces
4. Write tests that verify extraction works correctly
5. Update documentation

### Pull Requests

- PRs should be atomic and address a single issue
- Reference issues using standard conventions (e.g., "fixes #123")
- Ensure all tests pass: `just test`
- Follow the existing code style (enforced by `ruff`)

## Development Setup

```bash
# Clone the repository
git clone https://github.com/ai4curation/ai-blame
cd ai-blame

# Install dependencies
uv sync

# Run tests
just test

# Run specific test file
uv run pytest tests/test_cli.py -v

# Build docs locally
just docs
```

### Project Structure

```
src/ai_blame/
├── cli.py          # Typer CLI commands
├── config.py       # Configuration loading (.ai-blame.yaml)
├── extractor.py    # Trace parsing and edit extraction
├── models.py       # Pydantic data models
└── updater.py      # File update logic (append, sidecar, comment)

tests/
├── data/           # Test trace data
├── test_cli.py     # CLI integration tests
├── test_extractor.py
└── test_updater.py
```

### Testing with Real Traces

The test suite includes real Claude Code trace data in `tests/data/`. To test with your own traces:

```bash
ai-blame stats --dir /path/to/project --home /path/to/home
```

### Code Style

- Use type hints
- Write docstrings with doctests where appropriate
- Follow existing patterns in the codebase
- Run `just format` before committing

[issues]: https://github.com/ai4curation/ai-blame/issues/
[pulls]: https://github.com/ai4curation/ai-blame/pulls/
