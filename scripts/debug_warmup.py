#!/usr/bin/env python3
"""
Debug warmup period requirements for MACD and ATR
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import alpha_ta
    import talib
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# Generate test data
np.random.seed(42)
n = 100
close = np.cumsum(np.random.randn(n)) + 100
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))

print("=" * 80)
print("Warmup Period Analysis")
print("=" * 80)

# 1. MACD warmup analysis
print("\n[1] MACD Warmup Analysis (12, 26, 9)")
print("-" * 80)
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# Find first non-NaN index
first_valid_macd = np.where(~np.isnan(talib_macd))[0]
first_valid_signal = np.where(~np.isnan(talib_signal))[0]
first_valid_hist = np.where(~np.isnan(talib_hist))[0]

if len(first_valid_macd) > 0:
    print(f"TA-Lib MACD first valid index: {first_valid_macd[0]}")
else:
    print("TA-Lib MACD: all NaN")

if len(first_valid_signal) > 0:
    print(f"TA-Lib Signal first valid index: {first_valid_signal[0]}")
else:
    print("TA-Lib Signal: all NaN")

if len(first_valid_hist) > 0:
    print(f"TA-Lib Hist first valid index: {first_valid_hist[0]}")
else:
    print("TA-Lib Hist: all NaN")

# Expected warmup calculation
expected_warmup = 26 + 9 - 2  # slow_period + signal_period - 2
print(f"\nExpected warmup (slow + signal - 2): {expected_warmup}")
print(f"Expected first valid index: {expected_warmup}")

# Check AlphaTA
alpha_result = alpha_ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd = np.array(alpha_result[0])
first_valid_alpha = np.where(~np.isnan(alpha_macd))[0]
if len(first_valid_alpha) > 0:
    print(f"AlphaTA MACD first valid index: {first_valid_alpha[0]}")

# 2. ATR warmup analysis
print("\n[2] ATR Warmup Analysis (period=14)")
print("-" * 80)
talib_atr = talib.ATR(high, low, close, timeperiod=14)

first_valid_atr = np.where(~np.isnan(talib_atr))[0]
if len(first_valid_atr) > 0:
    print(f"TA-Lib ATR first valid index: {first_valid_atr[0]}")
else:
    print("TA-Lib ATR: all NaN")

# Expected warmup for ATR
print(f"\nExpected warmup for ATR: {14 - 1} (period - 1)")
print(f"Expected first valid index: {14 - 1}")

# Check AlphaTA
alpha_atr = np.array(alpha_ta.atr(high, low, close, timeperiod=14))
first_valid_alpha_atr = np.where(~np.isnan(alpha_atr))[0]
if len(first_valid_alpha_atr) > 0:
    print(f"AlphaTA ATR first valid index: {first_valid_alpha_atr[0]}")

# 3. EMA warmup analysis
print("\n[3] EMA Warmup Analysis (period=14)")
print("-" * 80)
talib_ema = talib.EMA(close, timeperiod=14)
first_valid_ema = np.where(~np.isnan(talib_ema))[0]
if len(first_valid_ema) > 0:
    print(f"TA-Lib EMA first valid index: {first_valid_ema[0]}")

alpha_ema = np.array(alpha_ta.ema(close, timeperiod=14))
first_valid_alpha_ema = np.where(~np.isnan(alpha_ema))[0]
if len(first_valid_alpha_ema) > 0:
    print(f"AlphaTA EMA first valid index: {first_valid_alpha_ema[0]}")

print("\n" + "=" * 80)
