#!/usr/bin/env bash
set -euo pipefail

# Canonical version checker. Keep this wrapper for existing local/CI callers.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$ROOT/scripts/check_versions.py" "$@"
