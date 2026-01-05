# Quick Reference: Python Distribution

## For End Users

### Install via Python Package Managers

```bash
# Using pip
pip install ai-blame

# Using uv (faster)
uv add --dev ai-blame

# Using pipx (global install)
pipx install ai-blame
```

### Usage

```bash
# All CLI commands work the same
ai-blame --help
ai-blame report
ai-blame annotate --dry-run
```

## For Developers

### Local Development

```bash
# Install maturin
pip install maturin

# Build in dev mode (editable install)
maturin develop

# Test your changes
ai-blame --version
```

### Build a Release Wheel

```bash
# Build optimized wheel
maturin build --release

# Wheel will be in target/wheels/
ls target/wheels/

# Test install in a fresh venv
python -m venv test-env
source test-env/bin/activate
pip install target/wheels/*.whl
ai-blame --version
```

### Quick Test Script

```bash
./test-python-build.sh
```

## For Release Managers

### Publishing to PyPI

Wheels are automatically built and published when you create a GitHub release.

**Prerequisites**: Configure trusted publishing on PyPI:
1. Go to PyPI → **Publishing** → **Add a new pending publisher**
2. Set:
   - **PyPI Project Name**: `ai-blame`
   - **Owner**: Your GitHub username/org
   - **Repository name**: `ai-blame.rs`
   - **Workflow name**: `python-wheels.yml`
   - **Environment name**: `pypi`

**To publish**:
1. **Tag the release**: `git tag v0.1.0 && git push --tags`
2. **Create GitHub release**: Go to Releases → Draft a new release
3. **Publish**: The `python-wheels.yml` workflow will:
   - Build wheels for Linux (x86_64, aarch64)
   - Build wheels for Windows (x64, x86)
   - Build wheels for macOS (x86_64, aarch64)
   - Build source distribution (sdist)
   - Publish all to PyPI using OIDC trusted publishing (no token needed)

### Manual Publishing (if needed)

```bash
# Build all wheels locally (requires appropriate platforms)
maturin build --release --target x86_64-unknown-linux-gnu

# Upload to PyPI
maturin upload
```

### Test PyPI (for testing)

```bash
# Publish to Test PyPI first
maturin publish --repository testpypi

# Test install
pip install --index-url https://test.pypi.org/simple/ ai-blame
```

## Architecture

### How It Works

1. **pyproject.toml**: Configures maturin as the build backend with `bindings = "bin"`
2. **Cargo.toml**: Defines the Rust binary target `[[bin]]`
3. **maturin**: 
   - Compiles the Rust binary for the target platform
   - Packages it into a Python wheel
   - The wheel installs the binary into the venv's bin/ directory
4. **Result**: Users get the Rust binary via `pip install`, no Rust toolchain needed

### No Python Code

This package contains **zero Python code**. It's purely a distribution mechanism for the Rust binary. The wheel includes:
- The compiled `ai-blame` binary
- Metadata (version, license, dependencies)
- No Python modules or packages

### Platform Support

Pre-built wheels for:
- **Linux**: x86_64, aarch64 (manylinux)
- **Windows**: x64, x86
- **macOS**: x86_64 (Intel), aarch64 (Apple Silicon)

## Comparison with Other Distribution Methods

| Method | Pros | Cons |
|--------|------|------|
| **PyPI (pip/uv)** | Easy for Python devs, no Rust needed | Slightly larger download |
| **Cargo** | Native Rust, smaller binary | Requires Rust toolchain |
| **Homebrew** | Easy for macOS users | Platform-specific |
| **Direct binary** | Maximum control | Manual updates |

## Following the Astral Model

This distribution strategy is inspired by successful Rust tools in the Python ecosystem:

- **Ruff**: Python linter written in Rust, distributed via PyPI
- **uv**: Python package manager written in Rust, distributed via PyPI
- **Pydantic Core**: Validation library written in Rust, distributed via PyPI

These tools prove that Python developers readily adopt Rust-based tools when:
1. Installation is easy (`pip install`)
2. Performance is significantly better
3. The interface (CLI/API) remains familiar

## FAQ

**Q: Do Python users need Rust installed?**
A: No! The wheel contains a pre-compiled binary. Just `pip install` and go.

**Q: Is there a Python API?**
A: Not currently. The CLI provides all functionality. If you need programmatic access, open an issue.

**Q: How big are the wheels?**
A: Typically around 1.4–3 MB, depending on platform and build options (for example, debug symbols). Still smaller than many Python packages with C extensions.

**Q: Does this work with virtual environments?**
A: Yes! The binary is installed in the venv's bin/ directory like any other CLI tool.

**Q: Can I use this with pipx?**
A: Yes! `pipx install ai-blame` installs it globally.

**Q: What about Windows?**
A: Full Windows support with pre-built wheels.

## Resources

- [maturin documentation](https://www.maturin.rs/)
- [PyPA Wheel specification](https://packaging.python.org/specifications/binary-distribution-format/)
- [Ruff's approach to Rust + Python](https://github.com/astral-sh/ruff)
- [uv's distribution model](https://github.com/astral-sh/uv)
