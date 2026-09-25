#!/usr/bin/env bash
# check_rustdoc.sh — enforces ADR 0011 (see the policy note at the top of
# core/src/lib.rs): the public surface must be documented, and `cargo doc` must
# emit no warnings at all.
#
# This is the single implementation of that gate: `make check-rustdoc` and the
# `doc` job in .github/workflows/ci.yml both invoke this script, so a local run
# and CI cannot disagree about what "documented" means.
#
# Why RUSTDOCFLAGS and not RUSTFLAGS: rustdoc reads RUSTDOCFLAGS.  Setting only
# RUSTFLAGS leaves rustdoc lints (broken_intra_doc_links, invalid_html_tags,
# private_intra_doc_links) at their default `warn` level, so a crate with 100+
# broken links in its API reference still builds "successfully".
#
# Usage:
#   ./scripts/check_rustdoc.sh
#
# Exit codes:
#   0 — no rustdoc warnings; the public surface is fully documented.
#   1 — rustdoc reported warnings or errors (full log: target/doc_warnings.txt).
set -uo pipefail

cd "$(dirname "$0")/.."

LOG="$(pwd)/target/doc_warnings.txt"
mkdir -p target

# `--no-deps` keeps the run bounded to the crate we own; `--locked` makes the
# dependency set reproducible.
RUSTDOCFLAGS="-D warnings" cargo doc -p finkit --no-deps --locked 2>&1 | tee "$LOG"
status=${PIPESTATUS[0]}

# `-D warnings` already turns every lint into an error, so a non-zero status is
# normally sufficient.  The grep keeps the check honest if a future toolchain
# demotes a lint back to a warning, and catches `error[E0xxx]:` diagnostics that
# do not fail the process.
if [ "$status" -ne 0 ] || grep -qE "^(warning|error)(\[[^]]*\])?:" "$LOG"; then
    echo
    echo "rustdoc check FAILED: warnings/errors detected."
    echo "See $LOG for details."
    exit 1
fi

echo
echo "rustdoc check PASSED: no rustdoc warnings, public surface documented."
