#!/bin/bash
set -e

# Setup: Make ai-blame available in PATH and cd to repo
export PATH="/Users/cjm/repos/ai-blame.rs/target/release:$PATH"
cd /Users/cjm/repos/ai-blame.rs

echo "🔍 AI-Blame Demo: Real Project Traces"
echo "====================================="
echo "Extracting AI provenance from execution traces"
sleep 2

# Show we're in the repo
echo ""
echo "$ pwd"
sleep 0.5
pwd
sleep 1.5

# ============ PHASE 1: SETUP ============
echo ""
echo "📋 PHASE 1: Setup - Create a starter config"
sleep 1.5
echo "$ ai-blame init --flavor sidecar"
sleep 1
ai-blame init --flavor sidecar 2>&1 | head -5
sleep 2

# ============ PHASE 2: DISCOVERY ============
echo ""
echo "🔎 PHASE 2: Discovery - What traces do we have?"
sleep 1.5

echo ""
echo "Quick overview with stats:"
sleep 1
echo "$ ai-blame stats"
sleep 1
ai-blame stats
sleep 3

echo ""
echo "See timeline of recent changes:"
sleep 1
echo "$ ai-blame timeline -n 12"
sleep 1
ai-blame timeline -n 12
sleep 3

echo ""
echo "List AI sessions that worked on this project:"
sleep 1
echo "$ ai-blame transcript list -n 6 --columns SATMO"
sleep 1
ai-blame transcript list -n 6 --columns SATMO
sleep 3

# ============ PHASE 3: CENTERPIECE - BLAME ANALYSIS ============
echo ""
echo "🔬 PHASE 3: Blame Analysis - Line-level attribution"
sleep 1.5

echo ""
echo "Which AI model edited each line of README.md?"
sleep 1
echo "$ ai-blame blame README.md | head -n 30"
sleep 1
ai-blame blame README.md 2>/dev/null | head -n 30
sleep 3

echo ""
echo "Let's see blame for a source file (src/main.rs):"
sleep 1
echo "$ ai-blame blame src/main.rs"
sleep 1
ai-blame blame src/main.rs 2>/dev/null
sleep 2

echo ""
echo "Show blame with block boundaries (consecutive lines by same model):"
sleep 1
echo "$ ai-blame blame Cargo.toml --blocks | head -n 35"
sleep 1
ai-blame blame Cargo.toml --blocks 2>/dev/null | head -n 35
sleep 3

echo ""
echo "Deep dive: View a specific session that modified files"
sleep 1
echo "$ ai-blame transcript view bd8251a4 | head -n 45"
sleep 1
ai-blame transcript view bd8251a4 2>/dev/null | head -n 45
sleep 3

# ============ PHASE 4: OUTPUT & ANNOTATION ============
echo ""
echo "📝 PHASE 4: Output - Generate and preview annotations"
sleep 1.5

echo ""
echo "Preview what would be annotated (no changes yet):"
sleep 1
echo "$ ai-blame report --initial-and-recent | head -n 40"
sleep 1
ai-blame report --initial-and-recent 2>/dev/null | head -n 40
sleep 3

echo ""
echo "Test annotation with --dry-run (safe preview):"
sleep 1
echo "$ ai-blame annotate --dry-run --initial-and-recent | head -n 25"
sleep 1
ai-blame annotate --dry-run --initial-and-recent 2>/dev/null | head -n 25
sleep 3

# ============ SUMMARY ============
echo ""
echo "✨ Commands Demonstrated:"
echo ""
echo "Setup:"
echo "  • init          — Create .ai-blame.yaml config"
echo ""
echo "Discovery:"
echo "  • stats         — 80 traces, 378 edits"
echo "  • timeline      — Recent changes"
echo "  • transcript list — AI sessions"
echo ""
echo "Analysis (The Centerpiece):"
echo "  • blame         — Line-level model attribution"
echo "  • blame --blocks — Show consecutive line blocks"
echo "  • transcript view — Full session details"
echo ""
echo "Output:"
echo "  • report        — Preview annotations"
echo "  • annotate --dry-run — Test before applying"
echo ""
echo "Ready to use: ai-blame annotate  (or --help for options)"
sleep 2

# Clean up
rm -f .ai-blame.yaml
