# Python Distribution Strategy for ai-blame.rs

## Overview

This document outlines the strategy for distributing ai-blame.rs to Python users, following the successful model pioneered by Astral's tools (ruff, uv).

## Background

ai-blame.rs is a Rust port of the [Python ai-blame](https://github.com/ai4curation/ai-blame) tool. While the Rust implementation provides better performance, memory safety, and a single binary distribution, many Python developers want to install it using familiar Python package managers like `pip` or `uv`.

## The Astral Model

Astral has successfully demonstrated how to distribute Rust-based tools to Python users:

### Ruff Example
- Written entirely in Rust
- Primary interface is CLI (not a Python API)
- Distributed via PyPI as a Python package
- Uses maturin to build wheels containing the Rust binary
- Installable via `pip install ruff` or `uv add --dev ruff`
- No Rust toolchain required for installation

### uv Example
- Written entirely in Rust
- CLI tool, not a Python library
- Distributed via PyPI as a Python package
- Uses maturin to build wheels
- Seamless installation with `pip install uv`

### Key Benefits
1. **No Rust required**: Users don't need Rust toolchain installed
2. **Familiar workflow**: Uses standard Python package managers
3. **Pre-built binaries**: Wheels include compiled binaries for each platform
4. **Easy dev dependencies**: `uv add --dev ai-blame` just works
5. **Cross-platform**: Single distribution mechanism for all platforms

## Recommended Strategy

### Phase 1: Python Package Distribution (Recommended)

**Goal**: Enable `uv add --dev ai-blame` and `pip install ai-blame` without requiring Rust.

#### Implementation Steps

1. **Add maturin configuration**
   - Create `pyproject.toml` with maturin build-system
   - Configure package metadata
   - Set up binary distribution (no Python bindings needed)

2. **Set up CI/CD for wheel building**
   - Use maturin-action in GitHub Actions
   - Build wheels for Linux, macOS, Windows
   - Support multiple architectures (x86_64, aarch64)
   - Publish to PyPI on release

3. **Keep CLI-first approach**
   - No Python API initially
   - Binary is invoked as `ai-blame` command
   - Same CLI interface for all users

4. **Update documentation**
   - Add Python installation instructions
   - Show both Rust and Python installation methods
   - Clarify that it's the same binary regardless of installation method

#### Example pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "ai-blame"
version = "0.1.0"
description = "Extract provenance from AI agent execution traces"
readme = "README.md"
requires-python = ">=3.8"
license = {text = "BSD-3-Clause"}
authors = [
    {name = "Chris Mungall"}
]
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: BSD License",
    "Programming Language :: Rust",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.8",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
]

[project.urls]
Homepage = "https://github.com/ai4curation/ai-blame"
Repository = "https://github.com/ai4curation/ai-blame"

[tool.maturin]
# Build as a binary package (no Python API)
bindings = "bin"
# Strip debug symbols for smaller binaries
strip = true
```

#### Example GitHub Actions Workflow

```yaml
name: Build and Publish Python Package

on:
  release:
    types: [published]

jobs:
  build-wheels:
    name: Build wheels on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Build wheels
        uses: PyO3/maturin-action@v1
        with:
          command: build
          args: --release --out dist --find-interpreter
          manylinux: auto
      
      - name: Upload wheels
        uses: actions/upload-artifact@v4
        with:
          name: wheels-${{ matrix.os }}
          path: dist

  publish:
    name: Publish to PyPI
    runs-on: ubuntu-latest
    needs: build-wheels
    environment:
      name: pypi
      url: https://pypi.org/p/ai-blame
    permissions:
      id-token: write  # Required for OIDC trusted publishing
      contents: read
    
    steps:
      - name: Download all wheels
        uses: actions/download-artifact@v4
        with:
          path: dist
      
      - name: Publish to PyPI
        uses: PyO3/maturin-action@v1
        with:
          command: upload
          args: --non-interactive --skip-existing dist/**
```

### Phase 2: Python API Bindings (Optional, Future)

**When**: Only if there's demonstrated demand for programmatic Python usage.

#### Decision Criteria
- Are users requesting Python API access?
- Do users need to integrate ai-blame into Python scripts/tools?
- Is the CLI insufficient for their use cases?

#### If Needed
- Add PyO3 bindings for core functionality
- Create Python-friendly API wrappers
- Add Python examples and documentation
- Change maturin bindings from "bin" to "pyo3"

**Current recommendation**: Don't implement Python API bindings initially. The CLI is sufficient for most use cases.

## Original Python Version

### Deprecation Plan

1. **Archive the Python repository**
   - Add deprecation notice to Python repo README
   - Point users to Rust version
   - Keep Python repo available for historical reference

2. **Migration guide**
   - Create migration documentation
   - Show command equivalence
   - Explain configuration compatibility
   - Note performance improvements

3. **Maintain Python distribution of Rust version**
   - Python users get the same (better) tool
   - No need to maintain two codebases
   - Installation is even easier with pre-built wheels

### Communication

```markdown
# ai-blame (Python) - DEPRECATED

This Python implementation has been superseded by the Rust version at
https://github.com/ai4curation/ai-blame

## Why the change?

- 10-100x faster performance
- Single binary, no dependencies
- Better memory safety and reliability
- Still installable via pip/uv!

## Migration

The Rust version is a drop-in replacement:

```bash
# Old Python version
pip install ai-blame

# New Rust version (same command!)
pip install ai-blame
# or
uv add --dev ai-blame
```

All CLI commands remain the same. See the [migration guide](link) for details.
```

## Installation Methods After Implementation

### For Python Users
```bash
# Using pip
pip install ai-blame

# Using uv (recommended)
uv add --dev ai-blame

# Using pipx (for global install)
pipx install ai-blame
```

### For Rust Users
```bash
# From source
cargo install --path .

# From crates.io
cargo install ai-blame

# Using cargo-binstall
cargo binstall ai-blame
```

### For Other Users
```bash
# Homebrew
brew install ai-blame

# Direct binary download
curl -LsSf https://github.com/ai4curation/ai-blame/releases/latest/download/install.sh | sh
```

## Benefits of This Approach

1. **No Python API needed initially**: CLI covers 99% of use cases
2. **Easy Python installation**: Just like any Python package
3. **No Rust required**: Pre-built binaries in wheels
4. **Single codebase**: One tool, multiple distribution methods
5. **Better performance**: Everyone gets the fast Rust version
6. **Familiar workflow**: Python devs use pip/uv as usual
7. **Following proven model**: Ruff and uv show this works

## Timeline

1. **Week 1**: Set up maturin and pyproject.toml
2. **Week 2**: Create wheel-building CI/CD pipeline
3. **Week 3**: Test pre-release on TestPyPI
4. **Week 4**: Publish to PyPI
5. **Week 5**: Update documentation and announce
6. **Week 6**: Archive Python version with migration guide

## Questions & Answers

### Q: Do we need a Python API?
**A**: Not initially. The CLI is the primary interface, and it's what 99% of users need. We can add Python bindings later if there's demand.

### Q: Will Python users accept a Rust tool?
**A**: Yes! Ruff and uv prove that Python developers embrace Rust tools when they offer better performance and the same easy installation.

### Q: What about users who want the old Python version?
**A**: The Python repo stays available. But the new version is better in every way and installs just as easily, so migration should be painless.

### Q: How do we handle breaking changes?
**A**: Follow semantic versioning. The CLI API is the contract. Internal implementation (Python vs Rust) doesn't matter to users.

### Q: What about Windows/ARM support?
**A**: Maturin builds wheels for all major platforms and architectures. We get better cross-platform support than the Python version.

## References

- [Ruff on PyPI](https://pypi.org/project/ruff/)
- [uv on PyPI](https://pypi.org/project/uv/)
- [maturin documentation](https://www.maturin.rs/)
- [Astral's blog on Rust for Python tools](https://astral.sh/)
- [PyO3/maturin-action](https://github.com/PyO3/maturin-action)

## Conclusion

Following the Astral model, we should:

1. ✅ **Do**: Use maturin to distribute the Rust binary via PyPI
2. ✅ **Do**: Enable `uv add --dev ai-blame` for Python users
3. ✅ **Do**: Keep CLI as the primary interface
4. ❌ **Don't**: Create Python API bindings (unless needed)
5. ✅ **Do**: Archive the Python version with a migration guide
6. ✅ **Do**: Maintain a single, high-quality Rust codebase

This approach provides the best experience for all users: Python developers get easy installation with familiar tools, Rust developers get native access, and everyone benefits from improved performance and reliability.
