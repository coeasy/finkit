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
    import alpha_ta
except ImportError as e:
    print(f"ERROR: Failed to import alpha_ta: {e}")
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
    ("SMA", alpha_ta.sma, [close], {"timeperiod": 20}),
    ("EMA", alpha_ta.ema, [close], {"timeperiod": 20}),
    ("WMA", alpha_ta.wma, [close], {"timeperiod": 20}),
    ("DEMA", alpha_ta.dema, [close], {"timeperiod": 20}),
    ("TEMA", alpha_ta.tema, [close], {"timeperiod": 20}),
    ("KAMA", alpha_ta.kama, [close], {"timeperiod": 20}),
    ("MAMA", alpha_ta.mama, [close], {"fastlimit": 0.5, "slowlimit": 0.05}),
    ("T3", alpha_ta.t3, [close], {"timeperiod": 5, "vfactor": 0.7}),
    ("BBANDS", alpha_ta.bollinger_bands, [close], {"timeperiod": 20, "nbdevup": 2.0, "nbdevdn": 2.0}),
    ("SAR", alpha_ta.sar, [high, low], {"acceleration": 0.02, "maximum": 0.2}),
    ("MIDPOINT", alpha_ta.midpoint, [close], {"timeperiod": 14}),
    ("MIDPRICE", alpha_ta.midprice, [high, low], {"timeperiod": 14}),
]
test_category("Overlap Studies", overlap_tests)

# ============================================================================
# Momentum Indicators
# ============================================================================
momentum_tests = [
    ("RSI", alpha_ta.rsi, [close], {"timeperiod": 14}),
    ("MACD", alpha_ta.macd, [close], {"fastperiod": 12, "slowperiod": 26, "signalperiod": 9}),
    ("STOCH", alpha_ta.stoch, [high, low, close], {"fastk_period": 5, "slowk_period": 3, "slowd_period": 3}),
    ("WILLR", alpha_ta.willr, [high, low, close], {"timeperiod": 14}),
    ("ADX", alpha_ta.adx, [high, low, close], {"timeperiod": 14}),
    ("DX", alpha_ta.dx, [high, low, close], {"timeperiod": 14}),
    ("MOM", alpha_ta.mom, [close], {"timeperiod": 10}),
    ("ROC", alpha_ta.roc, [close], {"timeperiod": 10}),
    ("TRIX", alpha_ta.trix, [close], {"timeperiod": 30}),
    ("CMO", alpha_ta.chande_forecast_oscillator, [close], {"timeperiod": 14}),
    ("AROON", alpha_ta.aroon, [high, low], {"timeperiod": 14}),
    ("CCI", alpha_ta.cci, [high, low, close], {"timeperiod": 20}),
    ("BOP", alpha_ta.bop, [open_price, high, low, close], {}),
]
test_category("Momentum Indicators", momentum_tests)

# ============================================================================
# Volume Indicators
# ============================================================================
volume_tests = [
    ("AD", alpha_ta.ad, [high, low, close, volume], {}),
    ("ADOSC", alpha_ta.adosc, [high, low, close, volume], {"fastperiod": 3, "slowperiod": 10}),
    ("OBV", alpha_ta.obv, [close, volume], {}),
    ("MFI", alpha_ta.mfi, [high, low, close, volume], {"timeperiod": 14}),
]
test_category("Volume Indicators", volume_tests)

# ============================================================================
# Volatility Indicators
# ============================================================================
volatility_tests = [
    ("ATR", alpha_ta.atr, [high, low, close], {"timeperiod": 14}),
    ("NATR", alpha_ta.natr, [high, low, close], {"timeperiod": 14}),
    ("TRANGE", alpha_ta.trange, [high, low, close], {}),
]
test_category("Volatility Indicators", volatility_tests)

# ============================================================================
# Price Transform
# ============================================================================
price_tests = [
    ("AVGPRICE", alpha_ta.avgprice, [open_price, high, low, close], {}),
    ("MEDPRICE", alpha_ta.medprice, [high, low], {}),
    ("TYPPRICE", alpha_ta.typprice, [high, low, close], {}),
    ("WCLPRICE", alpha_ta.wclprice, [high, low, close], {}),
]
test_category("Price Transform", price_tests)

# ============================================================================
# Statistic Functions
# ============================================================================
stat_tests = [
    ("BETA", alpha_ta.beta, [close, close], {"timeperiod": 5}),
    ("CORREL", alpha_ta.correlation, [close, close], {"timeperiod": 5}),
    ("LINEARREG", alpha_ta.linear_reg, [close], {"timeperiod": 14}),
    ("TSF", alpha_ta.tsf, [close], {"timeperiod": 14}),
    ("STDDEV", alpha_ta.std_dev, [close], {"timeperiod": 5}),
    ("VAR", alpha_ta.var, [close], {"timeperiod": 5}),
]
test_category("Statistic Functions", stat_tests)

# ============================================================================
# Cycle Indicators
# ============================================================================
cycle_tests = [
    ("HT_DCPeriod", alpha_ta.ht_dcperiod, [close], {}),
    ("HT_DCPhase", alpha_ta.ht_dcphase, [close], {}),
    ("HT_Trendline", alpha_ta.ht_trendline, [close], {}),
    ("HT_Trendmode", alpha_ta.ht_trendmode, [close], {}),
]
test_category("Cycle Indicators", cycle_tests)

# ============================================================================
# Pattern Recognition (Candlestick)
# ============================================================================
pattern_tests = [
    ("CDL_DOJI", alpha_ta.cdl_doji, [open_price, high, low, close], {}),
    ("CDL_HAMMER", alpha_ta.cdl_hammer, [open_price, high, low, close], {}),
    ("CDL_ENGULFING", alpha_ta.cdl_engulfing, [open_price, high, low, close], {}),
    ("CDL_MORNING_STAR", alpha_ta.cdl_morning_star, [open_price, high, low, close], {}),
    ("CDL_EVENING_STAR", alpha_ta.cdl_evening_star, [open_price, high, low, close], {}),
]
test_category("Pattern Recognition", pattern_tests)

# ============================================================================
# Advanced Indicators
# ============================================================================
advanced_tests = [
    ("ICHIMOKU", alpha_ta.ichimoku, [high, low, close], {}),
    ("SUPERTREND", alpha_ta.supertrend, [high, low, close], {"period": 10, "multiplier": 3.0}),
    ("DONCHIAN", alpha_ta.donchian, [high, low], {"period": 20}),
    ("VWAP", alpha_ta.vwap, [high, low, close, volume], {}),
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
