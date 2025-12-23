# CLAUDE.md for ai-blame

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Extract provenance from AI agent execution traces

The project uses `uv` for dependency management and `just` as the command runner.

## IMPORTANT INSTRUCTIONS

- we use test driven development, write tests first before implementing a feature
- do not try and 'cheat' by making mock tests (unless asked)
- if functionality does not work, keep trying, do not relax the test just to get poor code in
- always run tests
- use docstrings

We make heavy use of doctests, these serve as both docs and tests. `just test` will include these,
or do `just doctest` just to write doctests

In general AVOID try/except blocks, except when these are truly called for, for example
when interfacing with external systems. For wrapping deterministic code,  these are ALMOST
NEVER required, if you think you need them, it's likely a bad smell that your logic is wrong.

## Essential Commands


### Testing and Quality
- `just test` - Run all tests, type checking, and formatting checks
- `just pytest` - Run Python tests only
- `just mypy` - Run type checking
- `just format` - Run ruff linting/formatting checks
- `uv run pytest tests/test_simple.py::test_simple` - Run a specific test

### Running the CLI
- `uv run ai-blame --help` - Run the CLI tool with options

### Documentation
- `just _serve` - Run local documentation server with mkdocs

## Project Architecture

### Core Structure
- **src/ai_blame/** - Main package
  - `cli.py` - Typer-based CLI interface, entry point (`ai-blame` command)
  - `extractor.py` - Logic for extracting provenance from Claude Code trace files (JSONL)
  - `models.py` - Data models for curation history entries
  - `updater.py` - Logic for updating YAML files with curation history
- **tests/** - Test suite using pytest with parametrized tests
- **docs/** - MkDocs-managed documentation with Material theme

### What the Tool Does
1. Scans Claude Code trace files (`~/.claude/projects/<encoded-cwd>/`) in JSONL format
2. Identifies successful `Edit` and `Write` tool operations
3. Extracts metadata: timestamp, model, file path
4. Groups by file and filters (first+last, size thresholds)
5. Appends `edit_history` sections to affected YAML files

### Technology Stack
- **Python 3.10+** with `uv` for dependency management
- **Typer** for CLI interface
- **PyYAML** for YAML file manipulation
- **pytest** for testing
- **mypy** for type checking
- **ruff** for linting and formatting
- **MkDocs Material** for documentation
- **LinkML** (dev dependency) for data modeling

### Key Configuration Files
- `pyproject.toml` - Python project configuration, dependencies, and tool settings
- `justfile` - Command runner recipes for common development tasks
- `project.justfile` - Project-specific recipes (imported by main justfile)
- `mkdocs.yml` - Documentation configuration
- `uv.lock` - Locked dependency versions

## Development Workflow

1. Dependencies are managed via `uv` - use `uv add` for new dependencies
2. All commands are run through `just` or `uv run`
3. The project uses dynamic versioning from git tags (uv-dynamic-versioning)
4. GitHub repo: https://github.com/ai4curation/ai-blame
