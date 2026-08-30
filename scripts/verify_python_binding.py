#!/usr/bin/env python3
"""
AlphaTA Python Binding - Comprehensive Function Verification
Tests all major indicator categories to ensure the wheel is functional.
"""
import sys
import numpy as np

# Add wheel path
sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import finkit
except ImportError as e:
    print(f"ERROR: Failed to import finkit: {e}")
    sys.exit(1)

print("=" * 70)
print("AlphaTA Python Binding Verification")
print("=" * 70)

# Generate test data
np.random.seed(42)
n = 500
close = np.cumsum(np.random.randn(n)) + 100
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))
open_price = close + np.random.randn(n) * 0.5
volume = np.abs(np.random.randn(n)) * 1000 + 500

test_results = []

def test_category(category, tests):
    """Test a category of functions"""
    print(f"\n[{category}]")
    passed = 0
    failed = 0
    
    for name, func, args, kwargs in tests:
        try:
            result = func(*args, **kwargs)
            if isinstance(result, tuple):
                # Multiple outputs
                for i, r in enumerate(result):
                    arr = np.asarray(r, dtype=float)
                    if arr.ndim != 1:
                        raise ValueError(f"Output {i} is not 1-D array")
                    if len(arr) != n:
                        raise ValueError(f"Output {i} length mismatch: {len(arr)} != {n}")
            else:
                # Single output
                arr = np.asarray(result, dtype=float)
                if arr.ndim != 1:
                    raise ValueError(f"Result is not 1-D array: {type(result)}")
                if len(arr) != n:
                    raise ValueError(f"Length mismatch: {len(arr)} != {n}")
            
            print(f"  ✓ {name}")
            passed += 1
        except Exception as e:
            print(f"  ✗ {name}: {e}")
            failed += 1
    
    test_results.append((category, passed, failed))
    return passed, failed

# ============================================================================
# Overlap Studies
# ============================================================================
overlap_tests = [
    ("SMA", finkit.sma, [close], {"timeperiod": 20}),
    ("EMA", finkit.ema, [close], {"timeperiod": 20}),
    ("WMA", finkit.wma, [close], {"timeperiod": 20}),
    ("DEMA", finkit.dema, [close], {"timeperiod": 20}),
    ("TEMA", finkit.tema, [close], {"timeperiod": 20}),
    ("KAMA", finkit.kama, [close], {"timeperiod": 20}),
    ("MAMA", finkit.mama, [close], {"fastlimit": 0.5, "slowlimit": 0.05}),
    ("T3", finkit.t3, [close], {"timeperiod": 5, "vfactor": 0.7}),
    ("BBANDS", finkit.bollinger_bands, [close], {"timeperiod": 20, "nbdevup": 2.0, "nbdevdn": 2.0}),
    ("SAR", finkit.sar, [high, low], {"acceleration": 0.02, "maximum": 0.2}),
    ("MIDPOINT", finkit.midpoint, [close], {"timeperiod": 14}),
    ("MIDPRICE", finkit.midprice, [high, low], {"timeperiod": 14}),
]
test_category("Overlap Studies", overlap_tests)

# ============================================================================
# Momentum Indicators
# ============================================================================
momentum_tests = [
    ("RSI", finkit.rsi, [close], {"timeperiod": 14}),
    ("MACD", finkit.macd, [close], {"fastperiod": 12, "slowperiod": 26, "signalperiod": 9}),
    ("STOCH", finkit.stoch, [high, low, close], {"fastk_period": 5, "slowk_period": 3, "slowd_period": 3}),
    ("WILLR", finkit.willr, [high, low, close], {"timeperiod": 14}),
    ("ADX", finkit.adx, [high, low, close], {"timeperiod": 14}),
    ("DX", finkit.dx, [high, low, close], {"timeperiod": 14}),
    ("MOM", finkit.mom, [close], {"timeperiod": 10}),
    ("ROC", finkit.roc, [close], {"timeperiod": 10}),
    ("TRIX", finkit.trix, [close], {"timeperiod": 30}),
    ("CMO", finkit.chande_forecast_oscillator, [close], {"timeperiod": 14}),
    ("AROON", finkit.aroon, [high, low], {"timeperiod": 14}),
    ("CCI", finkit.cci, [high, low, close], {"timeperiod": 20}),
    ("BOP", finkit.bop, [open_price, high, low, close], {}),
]
test_category("Momentum Indicators", momentum_tests)

# ============================================================================
# Volume Indicators
# ============================================================================
volume_tests = [
    ("AD", finkit.ad, [high, low, close, volume], {}),
    ("ADOSC", finkit.adosc, [high, low, close, volume], {"fastperiod": 3, "slowperiod": 10}),
    ("OBV", finkit.obv, [close, volume], {}),
    ("MFI", finkit.mfi, [high, low, close, volume], {"timeperiod": 14}),
]
test_category("Volume Indicators", volume_tests)

# ============================================================================
# Volatility Indicators
# ============================================================================
volatility_tests = [
    ("ATR", finkit.atr, [high, low, close], {"timeperiod": 14}),
    ("NATR", finkit.natr, [high, low, close], {"timeperiod": 14}),
    ("TRANGE", finkit.trange, [high, low, close], {}),
]
test_category("Volatility Indicators", volatility_tests)

# ============================================================================
# Price Transform
# ============================================================================
price_tests = [
    ("AVGPRICE", finkit.avgprice, [open_price, high, low, close], {}),
    ("MEDPRICE", finkit.medprice, [high, low], {}),
    ("TYPPRICE", finkit.typprice, [high, low, close], {}),
    ("WCLPRICE", finkit.wclprice, [high, low, close], {}),
]
test_category("Price Transform", price_tests)

# ============================================================================
# Statistic Functions
# ============================================================================
stat_tests = [
    ("BETA", finkit.beta, [close, close], {"timeperiod": 5}),
    ("CORREL", finkit.correlation, [close, close], {"timeperiod": 5}),
    ("LINEARREG", finkit.linear_reg, [close], {"timeperiod": 14}),
    ("TSF", finkit.tsf, [close], {"timeperiod": 14}),
    ("STDDEV", finkit.std_dev, [close], {"timeperiod": 5}),
    ("VAR", finkit.var, [close], {"timeperiod": 5}),
]
test_category("Statistic Functions", stat_tests)

# ============================================================================
# Cycle Indicators
# ============================================================================
cycle_tests = [
    ("HT_DCPeriod", finkit.ht_dcperiod, [close], {}),
    ("HT_DCPhase", finkit.ht_dcphase, [close], {}),
    ("HT_Trendline", finkit.ht_trendline, [close], {}),
    ("HT_Trendmode", finkit.ht_trendmode, [close], {}),
]
test_category("Cycle Indicators", cycle_tests)

# ============================================================================
# Pattern Recognition (Candlestick)
# ============================================================================
pattern_tests = [
    ("CDL_DOJI", finkit.cdl_doji, [open_price, high, low, close], {}),
    ("CDL_HAMMER", finkit.cdl_hammer, [open_price, high, low, close], {}),
    ("CDL_ENGULFING", finkit.cdl_engulfing, [open_price, high, low, close], {}),
    ("CDL_MORNING_STAR", finkit.cdl_morning_star, [open_price, high, low, close], {}),
    ("CDL_EVENING_STAR", finkit.cdl_evening_star, [open_price, high, low, close], {}),
]
test_category("Pattern Recognition", pattern_tests)

# ============================================================================
# Advanced Indicators
# ============================================================================
advanced_tests = [
    ("ICHIMOKU", finkit.ichimoku, [high, low, close], {}),
    ("SUPERTREND", finkit.supertrend, [high, low, close], {"period": 10, "multiplier": 3.0}),
    ("DONCHIAN", finkit.donchian, [high, low], {"period": 20}),
    ("VWAP", finkit.vwap, [high, low, close, volume], {}),
]
test_category("Advanced Indicators", advanced_tests)

# ============================================================================
# Summary
# ============================================================================
print("\n" + "=" * 70)
print("VERIFICATION SUMMARY")
print("=" * 70)

total_passed = 0
total_failed = 0

for category, passed, failed in test_results:
    total_passed += passed
    total_failed += failed
    status = "✓ PASS" if failed == 0 else "✗ FAIL"
    print(f"{status}  {category}: {passed} passed, {failed} failed")

print("-" * 70)
overall_status = "✓ ALL TESTS PASSED" if total_failed == 0 else "✗ SOME TESTS FAILED"
print(f"{overall_status}")
print(f"Total: {total_passed} passed, {total_failed} failed")
print("=" * 70)

sys.exit(0 if total_failed == 0 else 1)
