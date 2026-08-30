#!/usr/bin/env python3
"""
深度分析 MACD 和 ATR 精度差异
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import finkit
    import talib
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# 生成测试数据
np.random.seed(42)
n = 1000
close = np.cumsum(np.random.randn(n)) + 100
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))

print("=" * 80)
print("MACD 和 ATR 深度精度分析")
print("=" * 80)

# 1. MACD 深度分析
print("\n[1] MACD 深度分析")
print("-" * 80)

# 计算 EMA12 和 EMA26
talib_ema12 = talib.EMA(close, timeperiod=12)
talib_ema26 = talib.EMA(close, timeperiod=26)
alpha_ema12 = np.array(finkit.ema(close, timeperiod=12))
alpha_ema26 = np.array(finkit.ema(close, timeperiod=26))

print("EMA(12) 对比:")
print(f"  TA-Lib EMA12[25]:  {talib_ema12[25]:.10f}")
print(f"  AlphaTA EMA12[25]: {alpha_ema12[25]:.10f}")
print(f"  Diff: {abs(talib_ema12[25] - alpha_ema12[25]):.2e}")

print("\nEMA(26) 对比:")
print(f"  TA-Lib EMA26[25]:  {talib_ema26[25]:.10f}")
print(f"  AlphaTA EMA26[25]: {alpha_ema26[25]:.10f}")
print(f"  Diff: {abs(talib_ema26[25] - alpha_ema26[25]):.2e}")

# 手动计算 MACD line
manual_macd = talib_ema12 - talib_ema26
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

print("\nMACD Line 对比:")
print(f"  TA-Lib MACD[33]:      {talib_macd[33]:.10f}")
print(f"  Manual (EMA12-EMA26): {manual_macd[33]:.10f}")
print(f"  Diff: {abs(talib_macd[33] - manual_macd[33]):.2e}")

# 计算 Signal line (EMA9 of MACD)
manual_signal = talib.EMA(talib_macd[33:], timeperiod=9)
print("\nSignal Line 对比:")
print(f"  TA-Lib Signal[33]: {talib_signal[33]:.10f}")
print(f"  Manual EMA9:       {manual_signal[0]:.10f}")
print(f"  Diff: {abs(talib_signal[33] - manual_signal[0]):.2e}")

# 2. ATR 深度分析
print("\n[2] ATR 深度分析")
print("-" * 80)

# 计算 True Range
talib_tr = talib.TRANGE(high, low, close)
alpha_tr = np.array(finkit.trange(high, low, close))

print("True Range 对比:")
print(f"  TA-Lib TR[0]:  {talib_tr[0]:.10f}")
print(f"  AlphaTA TR[0]: {alpha_tr[0]:.10f}")
print(f"  Diff: {abs(talib_tr[0] - alpha_tr[0]):.2e}")

print(f"\n  TA-Lib TR[13]:  {talib_tr[13]:.10f}")
print(f"  AlphaTA TR[13]: {alpha_tr[13]:.10f}")
print(f"  Diff: {abs(talib_tr[13] - alpha_tr[13]):.2e}")

# 计算 ATR
talib_atr = talib.ATR(high, low, close, timeperiod=14)
alpha_atr = np.array(finkit.atr(high, low, close, timeperiod=14))

# 手动计算 ATR (SMA of TR)
manual_atr_sma = talib.SMA(talib_tr, timeperiod=14)

print("\nATR 对比:")
print(f"  TA-Lib ATR[13]:      {talib_atr[13]:.10f}")
print(f"  Manual SMA(TR,14):   {manual_atr_sma[13]:.10f}")
print(f"  AlphaTA ATR[13]:     {alpha_atr[13]:.10f}")
print(f"  Diff (TA-Lib vs Manual): {abs(talib_atr[13] - manual_atr_sma[13]):.2e}")
print(f"  Diff (TA-Lib vs AlphaTA): {abs(talib_atr[13] - alpha_atr[13]):.2e}")

# 检查 ATR 是否使用 Wilder's Smoothing (RMA)
manual_rma = talib.WMA(talib_tr, timeperiod=14)  # WMA 不是 RMA，但试试
print(f"\n  Manual WMA(TR,14):   {manual_rma[13]:.10f}")

# 尝试手动实现 RMA (Wilder's Smoothing)
def wilder_rma(data, period):
    """Wilder's Recursive Moving Average"""
    result = np.full_like(data, np.nan)
    if len(data) < period:
        return result
    
    # 初始值 = SMA
    result[period-1] = np.mean(data[:period])
    
    # Wilder's smoothing: RMA[i] = (RMA[i-1] * (period-1) + data[i]) / period
    for i in range(period, len(data)):
        result[i] = (result[i-1] * (period - 1) + data[i]) / period
    
    return result

manual_wilder_rma = wilder_rma(talib_tr, 14)
print(f"  Manual Wilder RMA:   {manual_wilder_rma[13]:.10f}")
print(f"  Diff (TA-Lib vs Wilder): {abs(talib_atr[13] - manual_wilder_rma[13]):.2e}")

# 3. 检查 EMA 实现差异
print("\n[3] EMA 实现细节分析")
print("-" * 80)

# 测试简单序列
test_data = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
talib_ema_test = talib.EMA(test_data, timeperiod=3)
alpha_ema_test = np.array(finkit.ema(test_data, timeperiod=3))

print("简单序列 EMA(3) 对比:")
print(f"  Input: {test_data}")
print(f"  TA-Lib EMA:  {talib_ema_test}")
print(f"  AlphaTA EMA: {alpha_ema_test}")

# 手动计算 EMA
# EMA[2] = SMA([1,2,3]) = 2.0
# EMA[3] = (3 - 2) * 2/4 + 2 = 2.5
# EMA[4] = (4 - 2.5) * 2/4 + 2.5 = 3.25
manual_ema = [np.nan, np.nan, 2.0]
k = 2.0 / (3 + 1)
for i in range(3, len(test_data)):
    manual_ema.append((test_data[i] - manual_ema[-1]) * k + manual_ema[-1])

print(f"  Manual EMA:  {manual_ema}")
print(f"  Diff (TA-Lib vs Manual at [4]): {abs(talib_ema_test[4] - manual_ema[4]):.2e}")

print("\n" + "=" * 80)
