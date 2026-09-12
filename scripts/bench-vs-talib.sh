#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit — one-click Finkit vs TA-Lib C head-to-head comparison.
#
# Steps:
#   1. Detect (and install if missing) TA-Lib C library + headers.
#   2. Record benchmark environment metadata for reproducibility.
#   3. Run `cargo bench --bench talib_c_comparison --features talib-c` to
#      collect Criterion JSON estimates.
#   4. Run scripts/bench_report.py to render a long-form report and a
#      machine-readable dist/bench/results.json (with per-indicator speedup).
#   5. Render a compact head-to-head summary table to dist/bench/summary.md.
#   6. (Optional) Run scripts/bench_vs_talib_precision.py to populate
#      delta_pp in the same results.json — only when --precision is passed.
#
# Outputs (under dist/bench/):
#   * finkit-vs-talib.md — full markdown report
#   * environment.json  — commit/toolchain/platform/TA-Lib metadata
#   * results.json      — machine-readable per-indicator data
#   * summary.md        — compact one-glance table
#   * precision.md      — (only with --precision) precision parity table
#   * precision.json    — (only with --precision) raw precision numbers
# ----------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
OUT="${ROOT}/dist/bench"
mkdir -p "${OUT}"

DO_PRECISION=0
BENCH_FILTER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --precision|--with-precision)
      DO_PRECISION=1
      shift
      ;;
    --bench-filter)
      shift
      BENCH_FILTER="${1:-}"
      [[ $# -gt 0 ]] && shift
      ;;
    -h|--help)
      sed -n '2,25p' "$0"
      exit 0
      ;;
    *)
      shift
      ;;
  esac
done

hdr()  { printf "\n\033[1;36m=== %s ===\033[0m\n" "$*"; }
ok()   { printf "  \033[32m[OK]\033[0m   %s\n" "$*"; }
err()  { printf "  \033[31m[FAIL]\033[0m %s\n" "$*"; }
warn() { printf "  \033[33m[WARN]\033[0m %s\n" "$*"; }

# ---- 1. TA-Lib C detection -----------------------------------------------
hdr "[bench-vs-talib] step 1: detect TA-Lib C"
if pkg-config --exists ta-lib 2>/dev/null; then
  ok "TA-Lib C found via pkg-config: $(pkg-config --modversion ta-lib)"
else
  warn "TA-Lib C not found; attempting platform install"
  case "$( uname -s )" in
    MINGW*|MSYS*|CYGWIN*)
      warn "Windows detected. Install the current TA-Lib 0.7.x release from:"
      warn "  https://github.com/TA-Lib/ta-lib/releases"
      warn "then set TA_LIBRARY_PATH and TA_INCLUDE_PATH if auto-discovery fails."
      if [[ -d "C:/ta-lib" ]]; then
        export TA_LIBRARY_PATH="C:/ta-lib/lib"
        export TA_INCLUDE_PATH="C:/ta-lib/include"
        ok "Found C:\\ta-lib; continuing with environment hints"
      else
        err "no TA-Lib installation found on Windows; cannot continue"
        exit 1
      fi
      ;;
    Darwin)
      if command -v brew >/dev/null 2>&1; then
        brew install ta-lib || { err "brew install ta-lib failed"; exit 1; }
      else
        err "no Homebrew; install TA-Lib manually"; exit 1
      fi
      ;;
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y libta-lib0-dev || {
          err "apt install failed; install the current TA-Lib release manually"; exit 1;
        }
      else
        err "no supported package manager; install TA-Lib manually"; exit 1
      fi
      ;;
    *)
      err "unsupported platform: $( uname -s )"; exit 1 ;;
  esac
fi

# ---- 2. reproducibility metadata -----------------------------------------
hdr "[bench-vs-talib] step 2: record benchmark environment"
python3 - "${ROOT}" "${OUT}/environment.json" <<'PY'
import json
import pathlib
import platform
import subprocess
import sys
from datetime import datetime, timezone

root = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])

def cmd(*args):
    try:
        return subprocess.check_output(args, cwd=root, text=True, stderr=subprocess.STDOUT).strip()
    except Exception:
        return None

payload = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "commit": cmd("git", "rev-parse", "HEAD"),
    "dirty": bool(cmd("git", "status", "--porcelain")),
    "platform": platform.platform(),
    "machine": platform.machine(),
    "processor": platform.processor(),
    "python": platform.python_version(),
    "rustc": cmd("rustc", "--version", "--verbose"),
    "cargo": cmd("cargo", "--version"),
    "talib": cmd("pkg-config", "--modversion", "ta-lib"),
}
out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
PY
ok "environment metadata written to ${OUT}/environment.json"

# ---- 3. cargo bench -------------------------------------------------------
hdr "[bench-vs-talib] step 3: cargo bench --features talib-c"
cd "${ROOT}/core"
if [[ -n "${BENCH_FILTER}" ]]; then
  cargo bench --bench talib_c_comparison --features talib-c -- "${BENCH_FILTER}"
else
  cargo bench --bench talib_c_comparison --features talib-c
fi
cd "${ROOT}"

# ---- 4. parse Criterion JSON + emit reports ------------------------------
hdr "[bench-vs-talib] step 4: bench_report.py -> markdown + JSON"
JSON_OUT="${OUT}/results.json"
python3 "${SCRIPT_DIR}/bench_report.py" \
    --criterion-dir "${ROOT}/target/criterion" \
    --output       "${OUT}/finkit-vs-talib.md" \
    --json-out     "${JSON_OUT}"

# ---- 5. compact summary table --------------------------------------------
hdr "[bench-vs-talib] step 5: render dist/bench/summary.md"
python3 - <<PY > "${OUT}/summary.md"
import json, pathlib, datetime
res_path = pathlib.Path("${JSON_OUT}")
res = json.loads(res_path.read_text())
generated = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

print("# Finkit vs TA-Lib C — Head-to-Head Summary")
print()
print(f"> Auto-generated by `scripts/bench-vs-talib.sh` on {generated}")
print("> Compare results only when commit, compiler, CPU, build flags, dataset, and TA-Lib version are recorded. See `environment.json`.")
print()
print("| Indicator | Category | Finkit (us) | TA-Lib C (us) | Speedup | Δ (pp) | Status |")
print("|---|---|---|---|---|---|---|")

bench = res.get("benchmarks", {})
order = sorted(bench.keys())
for k in order:
    v = bench[k]
    delta = v.get("delta_pp")
    delta_s = "—" if delta is None else f"{delta:.2e}"
    print(f"| {k} | {v.get('category', '?')} | {v['fta_us']:.2f} | "
          f"{v['talib_us']:.2f} | {v['speedup']:.2f}x | {delta_s} | {v['status']} |")

print()
total = len(bench)
faster = sum(1 for v in bench.values() if v['status'] == "✅")
slower = sum(1 for v in bench.values() if v['status'] == "❌")
warn_n = sum(1 for v in bench.values() if v['status'] == "⚠️")
print(f"- **Total**: {total}")
print(f"- **Finkit faster**: {faster}")
print(f"- **Finkit within 25%**: {warn_n}")
print(f"- **Finkit >25% slower**: {slower}")
PY
ok "summary written to ${OUT}/summary.md"

# ---- 6. optional precision parity ----------------------------------------
if [[ "${DO_PRECISION}" -eq 1 ]]; then
  hdr "[bench-vs-talib] step 6: precision parity"
  if python3 "${SCRIPT_DIR}/bench_vs_talib_precision.py" \
        --json-out "${OUT}/precision.json" \
        --results  "${JSON_OUT}" \
        --output   "${OUT}/precision.md"; then
    ok "precision data merged into ${JSON_OUT}"
  else
    warn "precision step failed (likely missing Finkit or TA-Lib Python package);"
    warn "continuing without precision"
  fi
fi

hdr "[bench-vs-talib] DONE"
ls -lh "${OUT}/"
