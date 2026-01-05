set shell := ["bash", "-euo", "pipefail", "-c"]

# Show available recipes
default:
  @just --list

# Check: formatting, build warnings, lints (no test execution)
check:
  cargo fmt --all -- --check
  RUSTFLAGS="-D warnings" cargo build --workspace
  cargo clippy --workspace --all-targets -- -D warnings

# Run all checks (format, build, lint) and tests
test: check
  cargo test --workspace

# Build debug + release for the whole workspace
build:
  cargo build --workspace
  cargo build --workspace --release

# Install the main CLI locally (from the workspace root)
install:
  cargo install --path . --locked

ui:
  cargo run -p ai-blame-ui

# Everything (check, test, build)
all: check test build


