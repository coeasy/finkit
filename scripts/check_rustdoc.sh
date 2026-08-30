#!/usr/bin/env bash
# check_rustdoc.sh — ADR 0011 enforcement.
#
# Runs `cargo doc --all-features --no-deps` and fails on any warnings.
# Should be invoked from the workspace root.
#
# Usage:
#   ./scripts/check_rustdoc.sh
#
# Exit codes:
#   0 — no warnings, all public re-exports documented.
#   1 — rustdoc reported warnings; inspect `target/doc_warnings.txt`.
set -euo pipefail

cd "$(dirname "$0")/.."

LOG="$(pwd)/target/doc_warnings.txt"
mkdir -p target

# `--no-deps` keeps the build time bounded (we only care about the crate itself).
RUSTDOCFLAGS="-D warnings" cargo +stable doc --workspace --all-features --no-deps 2>&1 | tee "$LOG"

# Strip the doctest noise that comes from examples in macros: `error: could not
# compile` lines are still surfaced as warnings and should fail the build.
if grep -E "^(warning|error):" "$LOG" >/dev/null; then
    echo
    echo "rustdoc check FAILED: warnings/errors detected."
    echo "See $LOG for details."
    exit 1
fi

echo
echo "rustdoc check PASSED: public surface is fully documented."
