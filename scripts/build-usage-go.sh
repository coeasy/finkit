#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit Go usage-package builder.
#
# Produces `libfinkit_go.so` (or `.dylib`/`.dll`) plus the Go wrapper
# sources, in dist/go/<platform>/. The Go module is `github.com/coeasy/finkit`
# and the wrapper re-exports the inner `go/ta` package.
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/go-binding"

case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)  PLATFORM="macos-arm64" ;;
      x86_64) PLATFORM="macos-x64" ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64" ;;
    esac ;;
  *) echo "unsupported platform: $( uname -s )" >&2; exit 1 ;;
esac

OUT_DIR="${ROOT}/dist/go/${PLATFORM}"
mkdir -p "${OUT_DIR}/lib"

# 1. Build the cgo shared lib -------------------------------------------
echo "[build-usage-go] cargo build --release -p finkit-go"
( cd "${ROOT}" && cargo build --release -p finkit-go )

case "${PLATFORM}" in
  windows-x64)  NATIVE="finkit_go.dll"            ;;
  macos-*)      NATIVE="libfinkit_go.dylib"       ;;
  linux-*)      NATIVE="libfinkit_go.so"          ;;
esac

if [[ ! -f "${ROOT}/target/release/${NATIVE}" ]]; then
  echo "[build-usage-go] ERROR: native lib not found: ${NATIVE}" >&2
  exit 1
fi
cp "${ROOT}/target/release/${NATIVE}" "${OUT_DIR}/lib/"
echo "[build-usage-go] staged lib/${NATIVE}"

# 2. Copy the Go wrapper sources ----------------------------------------
rm -rf "${OUT_DIR}/finkit"
mkdir -p "${OUT_DIR}/finkit"
cp -r "${BINDING_DIR}/go/." "${OUT_DIR}/finkit/"

# 2a. Drop references to JIT/SIMD functions that the Rust finkit-go crate
#     does not export (they would otherwise break cgo linking).
python "${SCRIPT_DIR}/drop_jit_simd.py" "${OUT_DIR}/finkit/ta/ta.go" 2>/dev/null \
  || "${ROOT}/.test_venv/Scripts/python.exe" "${SCRIPT_DIR}/drop_jit_simd.py" "${OUT_DIR}/finkit/ta/ta.go" 2>/dev/null \
  || "${ROOT}/.test_venv/bin/python" "${SCRIPT_DIR}/drop_jit_simd.py" "${OUT_DIR}/finkit/ta/ta.go" \
  || echo "[build-usage-go] WARN: drop_jit_simd.py failed; continuing"

# 2b. (line ending fix is done at the end of the script, AFTER all python
#     file rewrites have happened, because python on Windows re-introduces
#     CRLF on every write.)

# 3. Drop anything that isn't part of the published surface --------------
rm -rf "${OUT_DIR}/finkit/ta/ta_test.go" 2>/dev/null || true

# 4. Fix cgo LDFLAGS to point at the staged native lib --------------------
# The source ta.go references `../target/release` (relative to the source
# tree). When installed as a Go module, that path doesn't exist; rewrite
# the LDFLAGS to point at the lib/ directory we just created.
python "${SCRIPT_DIR}/patch_go_cgo_ldflags.py" "${OUT_DIR}/finkit/ta/ta.go" 2>/dev/null \
  || "${ROOT}/.test_venv/Scripts/python.exe" "${SCRIPT_DIR}/patch_go_cgo_ldflags.py" "${OUT_DIR}/finkit/ta/ta.go" 2>/dev/null \
  || "${ROOT}/.test_venv/bin/python" "${SCRIPT_DIR}/patch_go_cgo_ldflags.py" "${OUT_DIR}/finkit/ta/ta.go" \
  || { tmp="${OUT_DIR}/finkit/ta/ta.go.lf"; tr -d '\r' < "${OUT_DIR}/finkit/ta/ta.go" > "$tmp" && mv "$tmp" "${OUT_DIR}/finkit/ta/ta.go"; sed -i 's|-L\.\./target/release|-L${SRCDIR}/../../lib|' "${OUT_DIR}/finkit/ta/ta.go"; }

# 5. Pin the module path so the wrapper is self-contained ---------------
cat > "${OUT_DIR}/finkit/go.mod" <<EOF
module github.com/coeasy/finkit

go 1.21
EOF

# 6. Generate a top-level re-export so consumers can do ------------------
#     import "github.com/coeasy/finkit"
#     finkit.Sma(...)
# instead of having to import the inner `ta` package.
cat > "${OUT_DIR}/finkit/finkit.go" <<'EOF'
// Package finkit is a thin re-export of the underlying `ta` package so
// downstream code can `import "github.com/coeasy/finkit"` and call
// `finkit.Sma(...)` without ever knowing about the internal `ta` subpackage.
package finkit

import "github.com/coeasy/finkit/go/ta"

// Re-exported indicator result types
type (
	MacdResult   = ta.MacdResult
	BbandsResult = ta.BbandsResult
	StochResult  = ta.StochResult
	AroonResult  = ta.AroonResult
	HtPhasorResult = ta.HtPhasorResult
	HtSineResult   = ta.HtSineResult
)

// Re-exported indicator functions
var (
	Version          = ta.Version
	Sma              = ta.Sma
	Ema              = ta.Ema
	Wma              = ta.Wma
	Dema             = ta.Dema
	Tema             = ta.Tema
	Kama             = ta.Kama
	T3               = ta.T3
	Rsi              = ta.Rsi
	Macd             = ta.Macd
	Stoch            = ta.Stoch
	Adx              = ta.Adx
	Aroon            = ta.Aroon
	Cci              = ta.Cci
	Mom              = ta.Mom
	Roc              = ta.Roc
	Willr            = ta.Willr
	Obv              = ta.Obv
	Ad               = ta.Ad
	AdOsc            = ta.AdOsc
	Atr              = ta.Atr
	Natr             = ta.Natr
	Trange           = ta.Trange
	Bbands           = ta.Bbands
	HtDcPeriod       = ta.HtDcPeriod
	HtDcPhase        = ta.HtDcPhase
	HtPhasor         = ta.HtPhasor
	HtSine           = ta.HtSine
	HtTrendMode      = ta.HtTrendMode
	HtTrendLine      = ta.HtTrendLine
	ZScore           = ta.ZScore
	Beta             = ta.Beta
	Correlation      = ta.Correlation
	StdDev           = ta.StdDev
	LinearReg        = ta.LinearReg
	Tsf              = ta.Tsf
	FormulaEval      = ta.FormulaEval
	FormulaValidate  = ta.FormulaValidate
	FormulaEvalZeroCopy = ta.FormulaEvalZeroCopy
)
EOF

# 7. The lib was placed under OUT_DIR/lib but cgo now looks for it at
# finkit/../../lib relative to ta.go, which is exactly OUT_DIR/lib.
# Nothing to copy.

# 8. Strip CRLF line endings on every .go file in the staged module.
#    cgo refuses to parse CRLF source; the cp from git-bash on Windows
#    would have inserted CRLF, and the python script above would have
#    re-introduced it on every write.  We do this LAST.
echo "[build-usage-go] normalizing .go line endings to LF"
find "${OUT_DIR}/finkit" -type f -name '*.go' -print0 | while IFS= read -r -d '' f; do
  if grep -lU $'\r' "$f" >/dev/null 2>&1; then
    tmp="${f}.lf"
    tr -d '\r' < "$f" > "$tmp" && mv "$tmp" "$f"
  fi
done

echo
echo "[build-usage-go] done. Artifacts in ${OUT_DIR}:"
find "${OUT_DIR}" -maxdepth 2 -type f | sed "s|^|  |"
