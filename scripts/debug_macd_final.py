#!/usr/bin/env python3
"""
MACD 最终精度诊断
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

# 生成测试数据
np.random.seed(42)
n = 10000
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("MACD 最终精度诊断")
print("=" * 80)

# 手动计算 MACD
def manual_ema(data, period):
    result = np.full_like(data, np.nan)
    if len(data) < period:
        return result
    
    # 初始值 = SMA
    result[period-1] = np.mean(data[:period])
    
    # EMA: EMA[i] = data[i] * k + EMA[i-1] * (1-k)
    k = 2.0 / (period + 1)
    for i in range(period, len(data)):
        result[i] = data[i] * k + result[i-1] * (1 - k)
    
    return result

# 计算 EMA12 和 EMA26
ema12 = manual_ema(close, 12)
ema26 = manual_ema(close, 26)

print("\n[1] EMA 对比")
print("-" * 80)
talib_ema12 = talib.EMA(close, timeperiod=12)
talib_ema26 = talib.EMA(close, timeperiod=26)

print(f"TA-Lib EMA12[25]:  {talib_ema12[25]:.10f}")
print(f"AlphaTA EMA12[25]: {ema12[25]:.10f}")
print(f"Diff: {abs(talib_ema12[25] - ema12[25]):.2e}")

print(f"\nTA-Lib EMA26[25]:  {talib_ema26[25]:.10f}")
print(f"AlphaTA EMA26[25]: {ema26[25]:.10f}")
print(f"Diff: {abs(talib_ema26[25] - ema26[25]):.2e}")

# 计算 MACD line
macd_line = ema12 - ema26

print("\n[2] MACD Line 对比")
print("-" * 80)
talib_macd, _, _ = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_result = alpha_ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd = np.array(alpha_result[0])

print(f"TA-Lib MACD[33]:    {talib_macd[33]:.10f}")
print(f"Manual MACD[33]:    {macd_line[33]:.10f}")
print(f"AlphaTA MACD[33]:   {alpha_macd[33]:.10f}")
print(f"Diff (TA-Lib vs Manual): {abs(talib_macd[33] - macd_line[33]):.2e}")
print(f"Diff (TA-Lib vs AlphaTA): {abs(talib_macd[33] - alpha_macd[33]):.2e}")

# 计算 Signal line
print("\n[3] Signal Line 对比")
print("-" * 80)

# 找到第一个有效的 MACD 索引
first_macd_idx = 25  # slow_period - 1
macd_valid = macd_line[first_macd_idx:]

# 对 MACD line 应用 EMA 得到 Signal line
signal_manual = np.full_like(close, np.nan)
if len(macd_valid) >= 9:
    signal_ema = manual_ema(macd_valid, 9)
    signal_manual[first_macd_idx:] = signal_ema

talib_signal = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)[1]
alpha_signal = np.array(alpha_result[1])

print(f"TA-Lib Signal[33]:    {talib_signal[33]:.10f}")
print(f"Manual Signal[33]:    {signal_manual[33]:.10f}")
print(f"AlphaTA Signal[33]:   {alpha_signal[33]:.10f}")
print(f"Diff (TA-Lib vs Manual): {abs(talib_signal[33] - signal_manual[33]):.2e}")
print(f"Diff (TA-Lib vs AlphaTA): {abs(talib_signal[33] - alpha_signal[33]):.2e}")

# 检查 TA-Lib MACD 的实现细节
print("\n[4] TA-Lib MACD 实现细节分析")
print("-" * 80)
print("TA-Lib MACD 可能使用不同的 EMA 初始值或平滑算法")
print("检查 EMA 的第一个有效值位置：")
print(f"TA-Lib EMA12 第一个有效值索引: {np.where(~np.isnan(talib_ema12))[0][0]}")
print(f"Manual EMA12 第一个有效值索引: {np.where(~np.isnan(ema12))[0][0]}")

print("\n检查 MACD 的第一个有效值位置：")
print(f"TA-Lib MACD 第一个有效值索引: {np.where(~np.isnan(talib_macd))[0][0]}")
print(f"AlphaTA MACD 第一个有效值索引: {np.where(~np.isnan(alpha_macd))[0][0]}")

print("\n检查 Signal 的第一个有效值位置：")
print(f"TA-Lib Signal 第一个有效值索引: {np.where(~np.isnan(talib_signal))[0][0]}")
print(f"AlphaTA Signal 第一个有效值索引: {np.where(~np.isnan(alpha_signal))[0][0]}")

print("\n" + "=" * 80)
print("诊断完成")
print("=" * 80)
