#!/usr/bin/env python3
"""
调查 TA-Lib MACD 的真实实现
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import talib
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# 使用简单数据
close = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 
                  11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
                  21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
                  31.0, 32.0, 33.0, 34.0, 35.0], dtype=np.float64)

print("=" * 80)
print("TA-Lib MACD 实现调查")
print("=" * 80)

# 计算 EMA
ema12 = talib.EMA(close, timeperiod=12)
ema26 = talib.EMA(close, timeperiod=26)

print("\n[1] EMA 值")
print("-" * 80)
for i in range(25, 35):
    print(f"EMA12[{i}]: {ema12[i]:.10f}, EMA26[{i}]: {ema26[i]:.10f}, Diff: {ema12[i] - ema26[i]:.10f}")

# 计算 MACD
macd, signal, hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

print("\n[2] MACD 值")
print("-" * 80)
for i in range(25, 35):
    print(f"MACD[{i}]: {macd[i]:.10f}, Signal[{i}]: {signal[i]:.10f}, Hist[{i}]: {hist[i]:.10f}")

print("\n[3] 对比 EMA 差值与 MACD")
print("-" * 80)
for i in range(33, 35):
    ema_diff = ema12[i] - ema26[i]
    print(f"Index {i}:")
    print(f"  EMA12 - EMA26 = {ema_diff:.10f}")
    print(f"  MACD          = {macd[i]:.10f}")
    print(f"  Diff          = {abs(ema_diff - macd[i]):.10e}")

# 尝试不同的 MACD 计算方法
print("\n[4] 尝试不同的 MACD 计算方法")
print("-" * 80)

# 方法1: 标准 EMA 差值
method1 = ema12 - ema26
print(f"方法1 (EMA12 - EMA26)[33]: {method1[33]:.10f}")
print(f"TA-Lib MACD[33]:           {macd[33]:.10f}")
print(f"差异: {abs(method1[33] - macd[33]):.10e}")

# 方法2: 使用不同的 EMA 实现
# TA-Lib 可能使用了不同的 EMA 初始值
print("\n检查 TA-Lib EMA 的初始值：")
print(f"EMA12[11] (第一个有效值): {ema12[11]:.10f}")
print(f"前12个值的平均: {np.mean(close[:12]):.10f}")
print(f"差异: {abs(ema12[11] - np.mean(close[:12])):.10e}")

print("\n" + "=" * 80)
print("调查完成")
print("=" * 80)
