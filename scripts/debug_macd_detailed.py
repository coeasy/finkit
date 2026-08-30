#!/usr/bin/env python3
"""
MACD 深度诊断脚本 - 分析 MACD、Signal、Hist 的差异根源
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
n = 10000
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("MACD 深度诊断")
print("=" * 80)

# 计算 TA-Lib MACD
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# 计算 AlphaTA MACD
alpha_result = finkit.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd = np.array(alpha_result[0])
alpha_signal = np.array(alpha_result[1])
alpha_hist = np.array(alpha_result[2])

# 手动计算 EMA
def manual_ema(data, period):
    result = np.full_like(data, np.nan)
    k = 2.0 / (period + 1)
    
    # 第一个有效值 = SMA
    result[period-1] = np.mean(data[:period])
    
    # EMA 递推
    for i in range(period, len(data)):
        result[i] = data[i] * k + result[i-1] * (1 - k)
    
    return result

# 手动计算 EMA12 和 EMA26
ema12 = manual_ema(close, 12)
ema26 = manual_ema(close, 26)

# 手动计算 MACD line
manual_macd = ema12 - ema26

print("\n[1] EMA 对比")
print("-" * 80)
talib_ema12 = talib.EMA(close, timeperiod=12)
talib_ema26 = talib.EMA(close, timeperiod=26)

print(f"EMA12[100]:")
print(f"  TA-Lib:   {talib_ema12[100]:.10f}")
print(f"  Manual:   {ema12[100]:.10f}")
print(f"  差异:     {abs(talib_ema12[100] - ema12[100]):.2e}")

print(f"\nEMA26[100]:")
print(f"  TA-Lib:   {talib_ema26[100]:.10f}")
print(f"  Manual:   {ema26[100]:.10f}")
print(f"  差异:     {abs(talib_ema26[100] - ema26[100]):.2e}")

print("\n[2] MACD line 对比")
print("-" * 80)
print(f"MACD[100]:")
print(f"  TA-Lib:   {talib_macd[100]:.10f}")
print(f"  Manual:   {manual_macd[100]:.10f}")
print(f"  AlphaTA:  {alpha_macd[100]:.10f}")
print(f"  TA-Lib vs Manual:  {abs(talib_macd[100] - manual_macd[100]):.2e}")
print(f"  TA-Lib vs AlphaTA: {abs(talib_macd[100] - alpha_macd[100]):.2e}")

print(f"\nMACD[100:105]:")
for i in range(100, 105):
    print(f"  [{i}] TA-Lib: {talib_macd[i]:12.8f}  Manual: {manual_macd[i]:12.8f}  AlphaTA: {alpha_macd[i]:12.8f}")

print("\n[3] Signal line 对比")
print("-" * 80)
# 手动计算 Signal line（对 MACD line 的 EMA）
manual_signal = manual_ema(manual_macd[25:], 9)  # 从 slow_period-1 开始
manual_signal_full = np.full_like(close, np.nan)
manual_signal_full[25:] = manual_signal

print(f"Signal[100]:")
print(f"  TA-Lib:   {talib_signal[100]:.10f}")
print(f"  Manual:   {manual_signal_full[100]:.10f}")
print(f"  AlphaTA:  {alpha_signal[100]:.10f}")
print(f"  TA-Lib vs Manual:  {abs(talib_signal[100] - manual_signal_full[100]):.2e}")
print(f"  TA-Lib vs AlphaTA: {abs(talib_signal[100] - alpha_signal[100]):.2e}")

print("\n[4] Histogram 对比")
print("-" * 80)
manual_hist = manual_macd - manual_signal_full

print(f"Hist[100]:")
print(f"  TA-Lib:   {talib_hist[100]:.10f}")
print(f"  Manual:   {manual_hist[100]:.10f}")
print(f"  AlphaTA:  {alpha_hist[100]:.10f}")
print(f"  TA-Lib vs Manual:  {abs(talib_hist[100] - manual_hist[100]):.2e}")
print(f"  TA-Lib vs AlphaTA: {abs(talib_hist[100] - alpha_hist[100]):.2e}")

print("\n[5] 差异分析")
print("-" * 80)
# 找出差异最大的位置
macd_diff = np.abs(talib_macd - alpha_macd)
signal_diff = np.abs(talib_signal - alpha_signal)
hist_diff = np.abs(talib_hist - alpha_hist)

max_macd_idx = np.nanargmax(macd_diff)
max_signal_idx = np.nanargmax(signal_diff)
max_hist_idx = np.nanargmax(hist_diff)

print(f"MACD 最大差异位置 [{max_macd_idx}]:")
print(f"  TA-Lib:   {talib_macd[max_macd_idx]:.10f}")
print(f"  AlphaTA:  {alpha_macd[max_macd_idx]:.10f}")
print(f"  差异:     {macd_diff[max_macd_idx]:.2e}")

print(f"\nSignal 最大差异位置 [{max_signal_idx}]:")
print(f"  TA-Lib:   {talib_signal[max_signal_idx]:.10f}")
print(f"  AlphaTA:  {alpha_signal[max_signal_idx]:.10f}")
print(f"  差异:     {signal_diff[max_signal_idx]:.2e}")

print(f"\nHist 最大差异位置 [{max_hist_idx}]:")
print(f"  TA-Lib:   {talib_hist[max_hist_idx]:.10f}")
print(f"  AlphaTA:  {alpha_hist[max_hist_idx]:.10f}")
print(f"  差异:     {hist_diff[max_hist_idx]:.2e}")

print("\n[6] 检查 TA-Lib MACD 的实现细节")
print("-" * 80)
# TA-Lib 的 MACD 可能使用了不同的 Signal 计算方式
# 尝试：Signal 是对 (EMA12 - EMA26) 的 EMA，但可能使用了不同的起始位置

# 方法1：从索引 25 开始计算 Signal
signal_method1 = manual_ema(manual_macd[25:], 9)
signal_method1_full = np.full_like(close, np.nan)
signal_method1_full[25:] = signal_method1

# 方法2：从索引 33 开始计算 Signal（slow_period + signal_period - 2）
signal_method2 = manual_ema(manual_macd[33:], 9)
signal_method2_full = np.full_like(close, np.nan)
signal_method2_full[33:] = signal_method2

print(f"Signal[100] 不同计算方法:")
print(f"  TA-Lib:              {talib_signal[100]:.10f}")
print(f"  方法1 (从25开始):    {signal_method1_full[100]:.10f}")
print(f"  方法2 (从33开始):    {signal_method2_full[100]:.10f}")
print(f"  AlphaTA:             {alpha_signal[100]:.10f}")

print("\n" + "=" * 80)
print("诊断完成")
print("=" * 80)
