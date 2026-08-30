#!/usr/bin/env bash
# Helper: run a bash command with all toolchain paths set.
# Avoids the PowerShell/Bash quoting hell on Windows.
set -u
# Order matters: put miniforge3 (real python) BEFORE WindowsApps (stub).
export PATH="/c/Program Files/Git/bin:/c/Program Files/Go/bin:/c/Program Files/dotnet:/p/python/miniforge3:/c/Users/Administrator/AppData/Local/Microsoft/WindowsApps:/c/Windows/System32:$PATH"
mkdir -p /usr/local/bin 2>/dev/null || true
ln -sf /p/python/miniforge3/python.exe /usr/local/bin/python3 2>/dev/null || true
export CGO_ENABLED=1
exec "$@"
