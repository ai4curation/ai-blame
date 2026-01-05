# Work with Test Data

This guide shows how to use `ai-blame` with trace data from different locations, useful for testing or working with multiple projects.

## The Problem

By default, `ai-blame` looks for traces based on your current working directory:

```
~/.claude/projects/-Users-you-current-project/
```

But sometimes you need to:

- Test with sample trace data
- Process traces from a different machine
- Work with traces stored in a non-standard location

## Using `--dir` and `--home`

The `--dir` and `--home` options let you specify where to look for traces:

```bash
# Look for traces as if you were in a different directory
ai-blame stats --dir /path/to/project

# Look for traces in a different home directory
ai-blame stats --dir /path/to/project --home /other/home
```

### How It Works

The trace directory is computed as:

```
$home/.claude/projects/<encoded-dir>/
```

Where `<encoded-dir>` is a filesystem-safe encoding of the absolute path (path separators become `-`, and punctuation like `.` may also be normalized to `-`).

## Testing with Sample Data

### Project Structure

Set up test data like this:

```
tests/data/
├── .claude/
│   └── projects/
│       └── -Users-me-tests-data/
│           ├── session1.jsonl
│           └── session2.jsonl
├── testdir1/
│   ├── foo.yaml
│   └── bar.txt
└── testdir2/
    └── config.yaml
```

!!! note
    The encoded folder name must match the absolute path of your test data directory.


### Running Against Test Data

```bash
# From the project root
ai-blame stats --dir tests/data --home tests/data

# Or with absolute paths
ai-blame stats \
  --dir /Users/me/project/tests/data \
  --home /Users/me/project/tests/data
```

### Repo-local fixtures (recommended for reproducible tests)

If you keep synthetic/redacted fixture traces in the repo (for example under
`tests/data/traces/`), prefer `--trace-dir` directly. This avoids the
`<encoded-dir>` naming requirement (which depends on absolute paths and varies
across machines):

```bash
ai-blame stats --trace-dir tests/data/traces
ai-blame report --trace-dir tests/data/traces --pattern "src/main.rs"
```

### Repo-local example project + fixture home (good for manual CLI testing)

This repository also includes a small example project plus a matching “fixture home”
that contains trace JSONLs under a `.claude/projects/<encoded-path>/` directory.
This is useful when you want to test the real CLI path resolution logic (`--home`)
without relying on your actual `~/.claude`.

Layout:

```
tests/examples/
├── simple/                # Example project files
└── simple-home/           # Fixture HOME directory (contains .claude/)
    └── .claude/
        └── projects/
            └── <encoded-path-for-tests/examples/simple>/
                ├── <session>.jsonl
                ├── agent-<...>.jsonl
                └── ...
```

Run it:

```bash
cd tests/examples/simple

ai-blame stats --home ../simple-home
ai-blame blame main.py --home ../simple-home
ai-blame blame main.py --home ../simple-home --show-agent
ai-blame blame main.py --home ../simple-home --blocks --show-agent
```

If you need to recreate/update this fixture from a local project, you generally:

- Copy project files into `tests/examples/simple/`
- Copy matching `~/.claude/projects/<encoded-project>/` JSONLs into
  `tests/examples/simple-home/.claude/projects/<encoded-simple>/`
- Rewrite `toolUseResult.filePath` entries so they point at the repo-local example path

## Using `--trace-dir` Directly

If you know the exact trace directory, use `--trace-dir`:

```bash
ai-blame stats --trace-dir ~/.claude/projects/-Users-alice-other-project/
```

This overrides `--dir` and `--home`.


