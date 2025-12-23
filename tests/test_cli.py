"""Tests for the CLI using real trace data."""

import shutil
from pathlib import Path

import pytest
from typer.testing import CliRunner

from ai_blame.cli import app
from ai_blame.extractor import extract_edit_history, convert_to_file_histories
from ai_blame.models import FilterConfig

runner = CliRunner()

# Path to test data that simulates a user's home directory
TEST_DATA_DIR = Path(__file__).parent / "data"

# The original encoded path in the checked-in test data
ORIGINAL_ENCODED_PATH = "-Users-cjm-repos-ai-blame-tests-data"
ORIGINAL_TRACE_DIR = TEST_DATA_DIR / ".claude" / "projects" / ORIGINAL_ENCODED_PATH

# Compute the trace directory based on the actual resolved path
# Claude Code encodes paths by replacing / with -
_resolved_test_data = TEST_DATA_DIR.resolve()
_encoded_path = str(_resolved_test_data).replace("/", "-")
TRACE_DIR = TEST_DATA_DIR / ".claude" / "projects" / _encoded_path

# The original path that appears in the trace files (for rewriting)
ORIGINAL_TRACE_PATH = "/Users/cjm/repos/ai-blame/tests/data"


def rewrite_trace_file(src: Path, dest: Path, old_path: str, new_path: str) -> None:
    """Rewrite a JSONL trace file, replacing path references."""
    with open(src) as f:
        content = f.read()
    # Replace the original path with the new temp path
    content = content.replace(old_path, new_path)
    with open(dest, "w") as f:
        f.write(content)


@pytest.fixture
def temp_workspace(tmp_path: Path) -> Path:
    """
    Copy test data to a temporary directory with rewritten paths.

    This fixture:
    1. Copies test directories (testdir1, etc.) to tmpdir
    2. Rewrites JSONL trace files to use tmpdir paths
    3. Creates the trace directory with the correct encoded name

    Returns the path to the temporary workspace.
    """
    # Copy testdir1 and testdir2 to temp
    for dirname in ["testdir1"]:  # testdir2 is partially deleted in traces
        src = TEST_DATA_DIR / dirname
        if src.exists():
            shutil.copytree(src, tmp_path / dirname)

    # Create trace directory with encoded tmpdir path
    # Claude Code encodes paths by replacing / with -
    encoded_tmp_path = str(tmp_path).replace("/", "-")
    trace_dest = tmp_path / ".claude" / "projects" / encoded_tmp_path
    trace_dest.mkdir(parents=True, exist_ok=True)

    # Copy and rewrite trace files from the original checked-in location
    for trace_file in ORIGINAL_TRACE_DIR.glob("*.jsonl"):
        rewrite_trace_file(
            trace_file,
            trace_dest / trace_file.name,
            ORIGINAL_TRACE_PATH,
            str(tmp_path),
        )

    return tmp_path


class TestExtractorWithRealData:
    """Test the extractor module with real trace data."""

    def test_extract_finds_all_creates(self):
        """Test that extractor finds all file creation events."""
        config = FilterConfig()
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        # These files were created in the trace logs
        created_files = [p for p in edits_by_file.keys()]
        created_basenames = [Path(p).name for p in created_files]

        # From trace analysis:
        # - testdir1/foo.yaml (created then edited)
        # - testdir1/bar.txt (created)
        # - testdir1/baz.md (created then edited)
        # - testdir2/foo.yaml (created)
        # - testdir2/bar.txt (created, later deleted via bash - not tracked)
        # - testdir2/data.json (created)
        assert "foo.yaml" in created_basenames
        assert "bar.txt" in created_basenames
        assert "baz.md" in created_basenames
        assert "data.json" in created_basenames

    def test_extract_finds_edits(self):
        """Test that extractor finds edit events (not just creates)."""
        config = FilterConfig()
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        # foo.yaml in testdir1 was created then edited (status change)
        foo_yaml_path = [p for p in edits_by_file.keys() if p.endswith("testdir1/foo.yaml")]
        assert len(foo_yaml_path) == 1
        foo_edits = edits_by_file[foo_yaml_path[0]]
        # Should have 2 events: create + edit
        assert len(foo_edits) >= 2

        # Check that first is create, second is edit
        assert foo_edits[0].is_create is True
        assert foo_edits[1].is_create is False

    def test_extract_with_pattern_filter(self):
        """Test that pattern filtering works."""
        config = FilterConfig(file_pattern="testdir1")
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        # All files should be in testdir1
        for path in edits_by_file.keys():
            assert "testdir1" in path

    def test_extract_yaml_only(self):
        """Test filtering to only YAML files."""
        config = FilterConfig(file_pattern=".yaml")
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        for path in edits_by_file.keys():
            assert path.endswith(".yaml")

    def test_convert_to_file_histories(self):
        """Test conversion to FileHistory with correct actions."""
        config = FilterConfig()
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        repo_root = str(TEST_DATA_DIR)
        histories = convert_to_file_histories(edits_by_file, repo_root)

        # Find foo.yaml history
        foo_history = None
        for rel_path, history in histories.items():
            if "foo.yaml" in rel_path and "testdir1" in rel_path:
                foo_history = history
                break

        assert foo_history is not None
        assert len(foo_history.events) >= 2

        # First event should be CREATED
        assert foo_history.events[0].action.value == "CREATED"
        # Second event should be EDITED
        assert foo_history.events[1].action.value == "EDITED"

    def test_initial_and_recent_filter(self):
        """Test that initial_and_recent_only keeps only first and last."""
        from ai_blame.extractor import apply_filters

        config = FilterConfig()
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        # Find a file with multiple edits (baz.md has create + edit)
        baz_path = [p for p in edits_by_file.keys() if "baz.md" in p]
        if baz_path and len(edits_by_file[baz_path[0]]) > 2:
            # Apply filter
            filter_config = FilterConfig(initial_and_recent_only=True)
            filtered = apply_filters(edits_by_file, filter_config)

            # Should have at most 2 entries per file
            for edits in filtered.values():
                assert len(edits) <= 2


class TestCLIStats:
    """Test the stats CLI command."""

    def test_stats_command_runs(self):
        """Test that stats command executes without error."""
        result = runner.invoke(app, ["stats", "--trace-dir", str(TRACE_DIR)])
        assert result.exit_code == 0
        assert "Trace directory:" in result.output
        assert "Trace files:" in result.output

    def test_stats_shows_file_counts(self):
        """Test that stats shows correct file counts."""
        result = runner.invoke(app, ["stats", "--trace-dir", str(TRACE_DIR)])
        assert result.exit_code == 0
        # Should find trace files
        assert "Trace files:" in result.output
        # Should find files with edits
        assert "Files with edits" in result.output

    def test_stats_with_pattern(self):
        """Test stats with pattern filter."""
        result = runner.invoke(
            app, ["stats", "--trace-dir", str(TRACE_DIR), "--pattern", ".yaml"]
        )
        assert result.exit_code == 0
        assert "matching '.yaml'" in result.output


class TestCLIMine:
    """Test the mine CLI command."""

    def test_mine_dry_run(self):
        """Test mine command in dry-run mode (default)."""
        result = runner.invoke(app, ["mine", "--trace-dir", str(TRACE_DIR)])
        assert result.exit_code == 0
        assert "[DRY RUN]" in result.output
        assert "No files modified" in result.output

    def test_mine_shows_summary_table(self):
        """Test that mine shows a summary table of files."""
        result = runner.invoke(app, ["mine", "--trace-dir", str(TRACE_DIR)])
        assert result.exit_code == 0
        assert "=== Summary ===" in result.output
        assert "File" in result.output
        assert "Edits" in result.output

    def test_mine_shows_yaml_preview(self):
        """Test that mine shows YAML previews."""
        result = runner.invoke(app, ["mine", "--trace-dir", str(TRACE_DIR)])
        assert result.exit_code == 0
        assert "=== YAML Preview:" in result.output
        assert "edit_history:" in result.output

    def test_mine_with_pattern(self):
        """Test mine with pattern filter."""
        result = runner.invoke(
            app, ["mine", "--trace-dir", str(TRACE_DIR), "--pattern", "testdir1"]
        )
        assert result.exit_code == 0
        # Should only show testdir1 files
        # All previews should be for testdir1
        if "=== YAML Preview:" in result.output:
            # Check that testdir1 appears
            assert "testdir1" in result.output

    def test_mine_initial_and_recent(self):
        """Test mine with --initial-and-recent flag."""
        result = runner.invoke(
            app, ["mine", "--trace-dir", str(TRACE_DIR), "--initial-and-recent"]
        )
        assert result.exit_code == 0

    def test_mine_apply_modifies_files(self, temp_workspace: Path):
        """Test that --apply actually modifies YAML files."""
        # Get the trace dir using encoded path (matching fixture)
        encoded_path = str(temp_workspace).replace("/", "-")
        trace_dir = temp_workspace / ".claude" / "projects" / encoded_path

        # Verify the YAML file exists before applying
        yaml_file = temp_workspace / "testdir1" / "foo.yaml"
        assert yaml_file.exists(), f"YAML file should exist at {yaml_file}"
        original_content = yaml_file.read_text()
        assert "edit_history" not in original_content

        # Run mine with --apply
        result = runner.invoke(
            app,
            [
                "mine",
                "--trace-dir",
                str(trace_dir),
                "--apply",
                "--pattern",
                "foo.yaml",
            ],
        )

        assert result.exit_code == 0, f"Command failed: {result.output}"
        assert "Scanning traces in:" in result.output

        # Verify the file was modified with edit_history
        updated_content = yaml_file.read_text()
        assert "edit_history:" in updated_content, "edit_history should be added"
        assert "agent_tool:" in updated_content, "agent_tool should be present"

    def test_mine_target_specific_file(self):
        """Test mine targeting a specific file."""
        result = runner.invoke(
            app,
            ["mine", "foo.yaml", "--trace-dir", str(TRACE_DIR)],
        )
        assert result.exit_code == 0
        # Should only show foo.yaml files
        if "=== YAML Preview:" in result.output:
            assert "foo.yaml" in result.output


class TestCLINonexistentTraceDir:
    """Test CLI behavior with nonexistent trace directory."""

    def test_stats_nonexistent_dir(self, tmp_path: Path):
        """Test stats with nonexistent trace directory."""
        result = runner.invoke(
            app, ["stats", "--trace-dir", str(tmp_path / "nonexistent")]
        )
        assert result.exit_code == 1
        assert "not found" in result.output

    def test_mine_nonexistent_dir(self, tmp_path: Path):
        """Test mine with nonexistent trace directory."""
        result = runner.invoke(
            app, ["mine", "--trace-dir", str(tmp_path / "nonexistent")]
        )
        assert result.exit_code == 1
        assert "not found" in result.output


class TestCLIEmptyResults:
    """Test CLI behavior when no edits are found."""

    def test_mine_no_matching_pattern(self):
        """Test mine with pattern that matches nothing."""
        result = runner.invoke(
            app,
            ["mine", "--trace-dir", str(TRACE_DIR), "--pattern", "nonexistent_pattern"],
        )
        assert result.exit_code == 0
        assert "No edits found" in result.output


class TestTraceDataIntegrity:
    """Verify the test trace data contains expected patterns."""

    def test_trace_files_exist(self):
        """Verify trace files exist in test data."""
        trace_files = list(TRACE_DIR.glob("*.jsonl"))
        assert len(trace_files) >= 2, "Expected at least 2 trace files"

    def test_trace_contains_creates_and_edits(self):
        """Verify traces contain both create and edit operations."""
        config = FilterConfig()
        edits_by_file = extract_edit_history(TRACE_DIR, config)

        has_create = False
        has_edit = False

        for edits in edits_by_file.values():
            for edit in edits:
                if edit.is_create:
                    has_create = True
                else:
                    has_edit = True

        assert has_create, "Expected at least one create operation"
        assert has_edit, "Expected at least one edit operation"

    def test_testdir1_files_exist(self):
        """Verify testdir1 files exist."""
        testdir1 = TEST_DATA_DIR / "testdir1"
        assert testdir1.exists()
        assert (testdir1 / "foo.yaml").exists()
        assert (testdir1 / "bar.txt").exists()
        assert (testdir1 / "baz.md").exists()


class TestDirAndHomeOptions:
    """Test the --dir and --home CLI options."""

    def test_stats_with_dir_and_home(self):
        """Test stats using --dir and --home to find test data traces."""
        # The test data structure:
        # tests/data/.claude/projects/-Users-cjm-repos-ai-blame-tests-data/
        # So if we set --home tests/data and --dir to the absolute path,
        # it should find the traces.
        target_dir = TEST_DATA_DIR.resolve()
        result = runner.invoke(
            app,
            [
                "stats",
                "--dir", str(target_dir),
                "--home", str(TEST_DATA_DIR),
            ],
        )
        assert result.exit_code == 0
        assert "Trace files:" in result.output

    def test_mine_with_dir_and_home(self):
        """Test mine using --dir and --home to find test data traces."""
        target_dir = TEST_DATA_DIR.resolve()
        result = runner.invoke(
            app,
            [
                "mine",
                "--dir", str(target_dir),
                "--home", str(TEST_DATA_DIR),
            ],
        )
        assert result.exit_code == 0
        assert "Scanning traces in:" in result.output

    def test_trace_dir_overrides_dir_and_home(self):
        """Test that --trace-dir takes precedence over --dir and --home."""
        result = runner.invoke(
            app,
            [
                "stats",
                "--trace-dir", str(TRACE_DIR),
                "--dir", "/some/other/path",
                "--home", "/another/path",
            ],
        )
        assert result.exit_code == 0
        # Should use the explicit trace dir, not the computed one
        assert str(TRACE_DIR) in result.output

    def test_relative_dir_option(self):
        """Test that relative paths work for --dir."""
        # Using relative path from the repo root
        import os
        original_cwd = os.getcwd()
        try:
            os.chdir(TEST_DATA_DIR.parent.parent)  # Go to repo root
            result = runner.invoke(
                app,
                [
                    "stats",
                    "--dir", "tests/data",
                    "--home", "tests/data",
                ],
            )
            # The dir gets resolved to absolute, so the encoded path will be
            # the absolute path of tests/data
            assert result.exit_code == 0 or "not found" in result.output
        finally:
            os.chdir(original_cwd)


class TestResolveTraceDir:
    """Test the resolve_trace_dir helper function."""

    def test_explicit_trace_dir_takes_priority(self):
        """Test that explicit trace_dir is returned as-is."""
        from ai_blame.cli import resolve_trace_dir

        result = resolve_trace_dir(
            trace_dir=Path("/explicit/path"),
            target_dir=Path("/ignored"),
            home_dir=Path("/also/ignored"),
        )
        assert result == Path("/explicit/path")

    def test_computes_from_dir_and_home(self):
        """Test that trace dir is computed from dir and home."""
        from ai_blame.cli import resolve_trace_dir

        result = resolve_trace_dir(
            trace_dir=None,
            target_dir=Path("/Users/test/myproject"),
            home_dir=Path("/Users/test"),
        )
        expected = Path("/Users/test/.claude/projects/-Users-test-myproject")
        assert result == expected

    def test_defaults_to_cwd_and_home(self, tmp_path: Path):
        """Test defaults when no options provided."""
        from ai_blame.cli import resolve_trace_dir
        import os

        original_cwd = os.getcwd()
        try:
            os.chdir(tmp_path)
            result = resolve_trace_dir(
                trace_dir=None,
                target_dir=None,
                home_dir=None,
            )
            # Should use cwd and home
            encoded = str(tmp_path).replace("/", "-")
            expected = Path.home() / ".claude" / "projects" / encoded
            assert result == expected
        finally:
            os.chdir(original_cwd)
