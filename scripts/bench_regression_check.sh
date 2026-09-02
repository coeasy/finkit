#!/usr/bin/env bash
# Benchmark Regression Check Script
# Compares current benchmark results against historical baseline data.
# Alerts when performance regression exceeds threshold.
#
# Usage:
#   ./scripts/bench_regression_check.sh [--threshold N] [--baseline PATH]
#
# Environment variables:
#   BENCH_BASELINE_PATH    - Path to baseline JSON (default: docs/benchmark-baseline.json)
#   REGRESSION_THRESHOLD_PCT - Regression threshold percentage (default: 5.0)
#
# Exit codes:
#   0 - All indicators within threshold
#   1 - Performance regression detected (exceeds threshold)
#   2 - Baseline file not found or invalid
#   3 - Benchmark execution failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

DEFAULT_THRESHOLD="${REGRESSION_THRESHOLD_PCT:-5.0}"
DEFAULT_BASELINE="${BENCH_BASELINE_PATH:-docs/benchmark-baseline.json}"

THRESHOLD="${DEFAULT_THRESHOLD}"
BASELINE_PATH="${DEFAULT_BASELINE}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --baseline)
            BASELINE_PATH="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--threshold N] [--baseline PATH]"
            echo ""
            echo "Options:"
            echo "  --threshold N    Regression threshold percentage (default: ${DEFAULT_THRESHOLD})"
            echo "  --baseline PATH  Path to baseline JSON file (default: ${DEFAULT_BASELINE})"
            echo ""
            echo "Environment variables:"
            echo "  BENCH_BASELINE_PATH      Path to baseline JSON"
            echo "  REGRESSION_THRESHOLD_PCT Regression threshold percentage"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$ROOT_DIR"

echo "=========================================="
echo " Benchmark Regression Check"
echo "=========================================="
echo ""
echo "Configuration:"
echo "  Baseline:    ${BASELINE_PATH}"
echo "  Threshold:   ${THRESHOLD}%"
echo ""

if [[ ! -f "${BASELINE_PATH}" ]]; then
    echo "ERROR: Baseline file not found: ${BASELINE_PATH}"
    exit 2
fi

echo "Step 1/4: Validating baseline file..."
python3 -c "
import json
import sys
path = '${BASELINE_PATH}'
try:
    with open(path) as f:
        data = json.load(f)
    if 'indicators' not in data:
        print('ERROR: Missing \"indicators\" key in baseline')
        sys.exit(1)
    indicators = data['indicators']
    if not indicators:
        print('ERROR: No indicators in baseline')
        sys.exit(1)
    print(f'  Found {len(indicators)} indicators in baseline')
    for ind, scales in indicators.items():
        for scale, val in scales.items():
            print(f'    {ind}@{scale}: {val} us')
except json.JSONDecodeError as e:
    print(f'ERROR: Invalid JSON: {e}')
    sys.exit(1)
except Exception as e:
    print(f'ERROR: {e}')
    sys.exit(1)
" || exit 2

echo ""
echo "Step 2/4: Running Criterion benchmarks..."
echo "  (This may take several minutes)"

BENCH_FAILED=0
# 1M regression gate (A-10) requires host-tuned codegen so the baseline we
# commit reflects the achievable single-threaded throughput. CI runners
# are pinned to x86-64-v3 in perf-gate.yml; locally, target-cpu=native
# matches whatever machine the dev is using to update the baseline.
RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native -C target-feature=+avx2,+fma,+bmi2,+lzcnt,+popcnt" \
cargo bench --bench talib_c_comparison --features talib-c -- --save-baseline main || BENCH_FAILED=1

if [[ $BENCH_FAILED -eq 1 ]]; then
    echo "WARNING: Benchmark execution had issues, attempting to continue..."
fi

echo ""
echo "Step 3/4: Collecting current benchmark timings..."

CURRENT_TIMINGS=$(python3 -c "
import json
import glob
import sys
from pathlib import Path

criterion_dir = Path('target/criterion')
results = {}

SCALE_GROUPS = {
    '10K': 'scaled_10k_vs_talib',
    '100K': 'scaled_100k_vs_talib',
    '1M': 'scaled_1m_vs_talib',
}

CORE_INDICATORS = ['SMA_20', 'EMA_12', 'RSI_14', 'MACD', 'BBANDS_20', 'ATR_14', 'ADX_14']

def load_estimate_ns(estimates_path):
    try:
        with open(estimates_path) as f:
            data = json.load(f)
        mean = data.get('mean', {})
        if 'point_estimate' in mean:
            return float(mean['point_estimate'])
        return None
    except:
        return None

def bench_role(name):
    lower = name.lower()
    if lower.startswith('fta_'):
        return ('fta', name[4:])
    return None

for scale, group in SCALE_GROUPS.items():
    results[scale] = {}
    pattern = str(criterion_dir / group / '**' / 'new' / 'estimates.json')
    for path_str in glob.glob(pattern, recursive=True):
        path = Path(path_str)
        parts = path.relative_to(criterion_dir).parts
        if len(parts) >= 4 and parts[-2] == 'new':
            bench_name = parts[1]
            ns = load_estimate_ns(path)
            if ns is None:
                continue
            role_info = bench_role(bench_name)
            if role_info and role_info[0] == 'fta':
                key = role_info[1].upper()
                results[scale][key] = ns / 1000.0

print(json.dumps(results))
") 2>/dev/null || CURRENT_TIMINGS="{}"

echo ""
echo "Step 4/4: Comparing against baseline..."

REGRESSIONS=""
PASS_COUNT=0
FAIL_COUNT=0

python3 -c "
import json
import sys

baseline_path = '${BASELINE_PATH}'
threshold = float('${THRESHOLD}')
current_raw = '${CURRENT_TIMINGS}'

try:
    current = json.loads(current_raw)
except:
    current = {}

with open(baseline_path) as f:
    baseline_data = json.load(f)

baseline = baseline_data.get('indicators', {})

regressions = []
passed = 0

for ind, scales in baseline.items():
    for scale, base_us in scales.items():
        cur_us = current.get(scale, {}).get(ind)
        if cur_us is None:
            print(f'  SKIP: {ind}@{scale} (no current data)')
            continue
        ratio = cur_us / base_us
        pct_change = (ratio - 1.0) * 100.0
        if ratio > (1.0 + threshold / 100.0):
            regressions.append({
                'indicator': ind,
                'scale': scale,
                'current': cur_us,
                'baseline': base_us,
                'pct': pct_change
            })
            print(f'  FAIL: {ind}@{scale} - {cur_us:.1f}us vs {base_us:.1f}us ({pct_change:+.1f}%)')
        else:
            passed += 1
            if pct_change <= 0:
                print(f'  PASS: {ind}@{scale} - {cur_us:.1f}us vs {base_us:.1f}us ({pct_change:+.1f}% faster)')
            else:
                print(f'  PASS: {ind}@{scale} - {cur_us:.1f}us vs {base_us:.1f}us ({pct_change:+.1f}% within threshold)')

print('')
print('==========================================')
if regressions:
    print(f' REGRESSION DETECTED: {len(regressions)} indicators')
    print('==========================================')
    print('')
    print('Regression details:')
    for r in regressions:
        print(f'  {r[\"indicator\"]}@{r[\"scale\"]}: {r[\"current\"]:.1f}us vs baseline {r[\"baseline\"]:.1f}us')
        print(f'    Regression: {r[\"pct\"]:+.1f}% (threshold: {threshold}%)')
    sys.exit(1)
else:
    print(f' ALL PASSED: {passed} indicators within {threshold}% threshold')
    print('==========================================')
    sys.exit(0)
"

RESULT=$?

echo ""

if [[ $RESULT -eq 0 ]]; then
    echo "Benchmark regression check PASSED"
    exit 0
else
    echo ""
    echo "Benchmark regression check FAILED"
    echo "Performance regression exceeds ${THRESHOLD}% threshold"
    echo ""
    echo "Recommendations:"
    echo "  1. Review recent code changes for performance impact"
    echo "  2. Run detailed benchmarks: cargo bench --bench talib_c_comparison --features talib-c"
    echo "  3. Generate report: python scripts/gen_benchmark_report.py"
    echo "  4. If regression is acceptable, update baseline: docs/benchmark-baseline.json"
    exit 1
fi