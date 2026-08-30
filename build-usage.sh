#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit — true one-click build + verify for all 7 language bindings.
#
# This is a thin forwarder to scripts/build-usage-packages.sh so that the
# common command `./build-usage.sh` works the same on every checkout.
#
# Usage:
#   ./build-usage.sh                  # all 7 languages
#   ./build-usage.sh python node      # subset
#   ./build-usage.sh --bench-talib    # run Finkit vs TA-Lib C head-to-head
#   ./build-usage.sh --no-bundle      # build only, skip the zip
#   ./build-usage.sh --no-verify      # skip install/verify step
#   ./build-usage.sh --json           # JSON manifest output
#   ./build-usage.sh --help           # full help
#
# Exit codes:
#   0  every requested language built + verified successfully
#   1  at least one language failed
#   2  invalid CLI argument
# ----------------------------------------------------------------------------
set -euo pipefail

ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TARGET="${ROOT}/scripts/build-usage-packages.sh"

if [[ ! -x "${TARGET}" ]]; then
  echo "[build-usage] ERROR: ${TARGET} not found or not executable" >&2
  echo "[build-usage]        run:  chmod +x ${TARGET}" >&2
  exit 127
fi

# Forward all args verbatim. The unified script is the source of truth.
exec "${TARGET}" "$@"
