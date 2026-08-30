#!/usr/bin/env python3
"""
Debug MACD and ATR calculation differences between TA-Lib and AlphaTA
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
n = 10000
close = np.cumsum(np.random.randn(n)) + 100
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))

print("=" * 80)
print("MACD & ATR Debug Analysis")
print("=" * 80)

# 1. MACD Analysis
print("\n[1] MACD Analysis")
print("-" * 80)
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_result = alpha_ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd, alpha_signal, alpha_hist = np.array(alpha_result[0]), np.array(alpha_result[1]), np.array(alpha_result[2])

# Find first valid index
first_valid = 25  # MACD needs slowperiod-1 = 25
print(f"First valid index: {first_valid}")
print(f"TA-Lib MACD[{first_valid}]: {talib_macd[first_valid]:.6f}")
print(f"AlphaTA MACD[{first_valid}]: {alpha_macd[first_valid]:.6f}")
print(f"Diff: {abs(talib_macd[first_valid] - alpha_macd[first_valid]):.6f}")

# Check if difference grows over time
print("\nMACD difference at different indices:")
for idx in [25, 100, 500, 1000, 5000]:
    diff = abs(talib_macd[idx] - alpha_macd[idx])
    print(f"  Index {idx}: diff = {diff:.6f}")

# 2. EMA Analysis (MACD components)
print("\n[2] EMA Analysis (MACD components)")
print("-" * 80)

# Calculate EMA manually to understand initialization
def manual_ema(data, period):
    """Calculate EMA with SMA initialization"""
    ema = np.zeros_like(data)
    ema[0] = data[0]
    multiplier = 2.0 / (period + 1)
    
    # Use SMA for first period values
    if len(data) >= period:
        ema[period-1] = np.mean(data[:period])
    
    for i in range(period, len(data)):
        ema[i] = (data[i] - ema[i-1]) * multiplier + ema[i-1]
    
    return ema

ema12_manual = manual_ema(close, 12)
ema26_manual = manual_ema(close, 26)

# TA-Lib EMA
talib_ema12 = talib.EMA(close, timeperiod=12)
talib_ema26 = talib.EMA(close, timeperiod=26)

# AlphaTA EMA
alpha_ema12 = np.array(alpha_ta.ema(close, timeperiod=12))
alpha_ema26 = np.array(alpha_ta.ema(close, timeperiod=26))

print(f"EMA(12) at index 11:")
print(f"  Manual (SMA init): {ema12_manual[11]:.6f}")
print(f"  TA-Lib:            {talib_ema12[11]:.6f}")
print(f"  AlphaTA:           {alpha_ema12[11]:.6f}")

print(f"\nEMA(26) at index 25:")
print(f"  Manual (SMA init): {ema26_manual[25]:.6f}")
print(f"  TA-Lib:            {talib_ema26[25]:.6f}")
print(f"  AlphaTA:           {alpha_ema26[25]:.6f}")

# 3. ATR Analysis
print("\n[3] ATR Analysis")
print("-" * 80)
talib_atr = talib.ATR(high, low, close, timeperiod=14)
alpha_atr = np.array(alpha_ta.atr(high, low, close, timeperiod=14))

# Calculate True Range manually
tr = np.maximum(high[1:] - low[1:], 
                np.maximum(np.abs(high[1:] - close[:-1]),
                          np.abs(low[1:] - close[:-1])))

print(f"True Range at index 1:")
print(f"  Manual: {tr[0]:.6f}")

# Check ATR initialization
print(f"\nATR at index 13 (first valid):")
print(f"  TA-Lib:  {talib_atr[13]:.6f}")
print(f"  AlphaTA: {alpha_atr[13]:.6f}")
print(f"  Diff:    {abs(talib_atr[13] - alpha_atr[13]):.6f}")

print("\nATR difference at different indices:")
for idx in [13, 50, 100, 500, 1000]:
    diff = abs(talib_atr[idx] - alpha_atr[idx])
    print(f"  Index {idx}: diff = {diff:.6f}")

# 4. Check if ATR uses SMA or RMA for initialization
print("\n[4] ATR Initialization Method")
print("-" * 80)
# First 14 TR values
tr_14 = tr[:14]
sma_14 = np.mean(tr_14)
print(f"SMA of first 14 TR: {sma_14:.6f}")
print(f"TA-Lib ATR[13]:     {talib_atr[13]:.6f}")
print(f"AlphaTA ATR[13]:    {alpha_atr[13]:.6f}")

# Check if they match SMA initialization
if abs(talib_atr[13] - sma_14) < 0.01:
    print("TA-Lib uses SMA for ATR initialization")
elif abs(alpha_atr[13] - sma_14) < 0.01:
    print("AlphaTA uses SMA for ATR initialization")
else:
    print("Both use different initialization method")

print("\n" + "=" * 80)
