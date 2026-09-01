#!/usr/bin/env bash
# CI performance gate: run TA-Lib C benchmarks, generate report, enforce speed threshold.
#
# Usage:
#   ./scripts/bench_gate.sh
#
# Exit codes:
#   0 - All indicators within threshold (FTA not >20% slower than TA-Lib C)
#   1 - Performance gate failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

echo "=========================================="
echo " FTA Performance Gate (vs TA-Lib C)"
echo "=========================================="

echo ""
echo "Step 1/5: Running Criterion benchmarks..."
# 1M regression gate (A-10) requires host-tuned codegen so the baseline we
# commit reflects the achievable single-threaded throughput. The bench
# profile (Cargo.toml) already sets lto = "fat" and opt-level = 3; we add
# target-cpu=native + AVX2/FMA/BMI2 features on top.
RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native -C target-feature=+avx2,+fma,+bmi2,+lzcnt,+popcnt" \
cargo bench --bench talib_c_comparison --features talib-c

echo ""
echo "Step 2/5: Generating benchmark report..."
python scripts/bench_report.py

echo ""
echo "Step 3/5: Checking TA-Lib gate (>20% slower => fail)..."
python scripts/bench_report.py --gate

echo ""
echo "Step 4/5: Checking regression gate (>5% vs baseline => fail)..."
python scripts/bench_report.py --regression-gate

echo ""
echo "Step 5/5: Checking 1M ns/bar SLA (A-10 linear regression gate)..."
python scripts/bench_report.py --sla-1m

echo ""
echo "All performance checks PASSED."
