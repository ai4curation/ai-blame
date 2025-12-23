"""Pytest configuration and fixtures for ai-blame tests."""

from pathlib import Path

import pytest

# Path to test data directory
TEST_DATA_DIR = Path(__file__).parent / "data"

# The original encoded path in the checked-in test data
ORIGINAL_ENCODED_PATH = "-Users-cjm-repos-ai-blame-tests-data"

# The original path that appears inside the trace files
ORIGINAL_TRACE_PATH = "/Users/cjm/repos/ai-blame/tests/data"


@pytest.fixture(scope="session", autouse=True)
def setup_trace_directory():
    """
    Set up the trace directory to match the current machine's path.

    The checked-in test data has traces in a directory named for the original
    developer's machine. This fixture creates a symlink (or copy) so that
    tests can find the traces regardless of the actual path.

    This runs once per test session and cleans up afterward.
    """
    # Compute what the encoded path should be for this machine
    resolved_test_data = TEST_DATA_DIR.resolve()
    expected_encoded = str(resolved_test_data).replace("/", "-")

    # Path where traces are stored in the repo
    original_trace_dir = TEST_DATA_DIR / ".claude" / "projects" / ORIGINAL_ENCODED_PATH

    # Path where traces need to be for the current machine
    expected_trace_dir = TEST_DATA_DIR / ".claude" / "projects" / expected_encoded

    # If the paths are the same, nothing to do
    if original_trace_dir.resolve() == expected_trace_dir.resolve():
        yield
        return

    # If expected path doesn't exist, create a symlink to the original
    if not expected_trace_dir.exists() and original_trace_dir.exists():
        expected_trace_dir.symlink_to(original_trace_dir.resolve())
        created_symlink = True
    else:
        created_symlink = False

    yield

    # Cleanup: remove the symlink we created
    if created_symlink and expected_trace_dir.is_symlink():
        expected_trace_dir.unlink()
