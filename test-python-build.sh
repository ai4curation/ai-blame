#!/bin/bash
# Test script for Python wheel building
# This script verifies that maturin can build wheels successfully
#
# Usage:
#   ./test-python-build.sh
#
# Prerequisites:
#   - Python 3.9+
#   - pip
#   - Rust toolchain (cargo)

set -e

echo "==================================="
echo "Testing Python Wheel Building"
echo "==================================="

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo "maturin not found, attempting to install..."
    if ! python -m pip --version >/dev/null 2>&1; then
        echo "❌ Error: python -m pip is not available. Please install pip for your Python interpreter."
        exit 1
    fi
    python -m pip install --user maturin
    # Add user bin directory to PATH if needed
    export PATH="$HOME/.local/bin:$PATH"
fi

echo ""
echo "Step 1: Building wheel with maturin..."
echo "--------------------------------------"
maturin build --release

echo ""
echo "Step 2: Listing generated wheels..."
echo "------------------------------------"
if ls target/wheels/*.whl 1> /dev/null 2>&1; then
    ls -lh target/wheels/*.whl
    echo ""
    echo "✅ Wheel built successfully!"
else
    echo "❌ No wheels found in target/wheels/"
    exit 1
fi

echo ""
echo "Step 3: Testing wheel installation..."
echo "--------------------------------------"
# Create a test virtual environment using mktemp for security
VENV_DIR=$(mktemp -d)
trap 'rm -rf "$VENV_DIR"' EXIT  # Cleanup on exit

python -m venv "$VENV_DIR"

# Activate the virtual environment (handle POSIX and Windows layouts)
if [ -f "$VENV_DIR/bin/activate" ]; then
    # POSIX-style venv (Linux/macOS)
    # shellcheck source=/dev/null
    source "$VENV_DIR/bin/activate"
elif [ -f "$VENV_DIR/Scripts/activate" ]; then
    # Windows-style venv (e.g., Git Bash with Windows Python)
    # shellcheck source=/dev/null
    source "$VENV_DIR/Scripts/activate"
else
    echo "❌ Could not find virtual environment activation script in '$VENV_DIR'." >&2
    echo "   Expected either 'bin/activate' (POSIX) or 'Scripts/activate' (Windows)." >&2
    exit 1
fi

# Install the most recent wheel (in case multiple builds exist)
WHEEL_FILE=$(ls -t target/wheels/*.whl 2>/dev/null | head -n1)
if [ -z "$WHEEL_FILE" ]; then
    echo "❌ No wheel files found in target/wheels/"
    exit 1
fi
echo "Installing $WHEEL_FILE..."
pip install "$WHEEL_FILE"

# Test the CLI
echo ""
echo "Testing ai-blame CLI..."
ai-blame --version
ai-blame --help > /dev/null

echo ""
echo "✅ CLI works correctly!"

# Cleanup happens via trap
deactivate

echo ""
echo "==================================="
echo "✅ All tests passed!"
echo "==================================="
echo ""
echo "Next steps:"
echo "  1. The wheel is ready at target/wheels/*.whl"
echo "  2. Test in your own environment: pip install target/wheels/*.whl"
echo "  3. When ready, create a GitHub release to publish to PyPI"
echo ""
