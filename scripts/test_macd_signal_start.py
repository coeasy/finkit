#!/usr/bin/env python3
"""
测试不同的 Signal line 计算起点
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import talib
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# 生成随机测试数据
np.random.seed(42)
n = 100
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("Signal line 计算起点测试")
print("=" * 80)

# 获取 TA-Lib MACD
macd_talib, signal_talib, hist_talib = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# 计算 EMA
ema12 = talib.EMA(close, timeperiod=12)
ema26 = talib.EMA(close, timeperiod=26)

# 计算完整的 MACD 序列（包括 NaN 位置）
macd_full = ema12 - ema26

print("\n[1] 检查 MACD line 的值")
print("-" * 80)
print(f"MACD_full[25]: {macd_full[25]:.10f}")
print(f"MACD_full[32]: {macd_full[32]:.10f}")
print(f"MACD_full[33]: {macd_full[33]:.10f}")
print(f"TA-Lib MACD[33]: {macd_talib[33]:.10f}")
print(f"差异: {abs(macd_full[33] - macd_talib[33]):.2e}")

# 方法1: 从索引 25 开始计算 Signal（使用完整的 MACD 序列）
print("\n[2] 方法1: 从索引 25 开始计算 Signal")
print("-" * 80)
macd_from_25 = macd_full[25:]
signal_from_25 = talib.EMA(macd_from_25, timeperiod=9)
print(f"Signal[33]: {signal_from_25[33-25]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_25[33-25] - signal_talib[33]):.2e}")

# 方法2: 从索引 32 开始计算 Signal（使用完整的 MACD 序列）
print("\n[3] 方法2: 从索引 32 开始计算 Signal")
print("-" * 80)
macd_from_32 = macd_full[32:]
signal_from_32 = talib.EMA(macd_from_32, timeperiod=9)
print(f"Signal[33]: {signal_from_32[33-32]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_32[33-32] - signal_talib[33]):.2e}")

# 方法3: 从索引 33 开始计算 Signal（使用完整的 MACD 序列）
print("\n[4] 方法3: 从索引 33 开始计算 Signal")
print("-" * 80)
macd_from_33 = macd_full[33:]
signal_from_33 = talib.EMA(macd_from_33, timeperiod=9)
print(f"Signal[33]: {signal_from_33[0]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_33[0] - signal_talib[33]):.2e}")

# 方法4: 检查 TA-Lib 是否使用了不同的 MACD 值来计算 Signal
print("\n[5] 检查 TA-Lib 使用的 MACD 值")
print("-" * 80)
# 假设 TA-Lib 使用了 MACD[25:33] 来计算 Signal[33]
# Signal[33] 应该是 MACD[25:33] 的 SMA（第一个 Signal 值）
macd_25_to_33 = macd_full[25:33]
print(f"MACD[25:33]: {macd_25_to_33}")
print(f"MACD[25:33] 的 SMA: {np.mean(macd_25_to_33):.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(np.mean(macd_25_to_33) - signal_talib[33]):.2e}")

# 方法5: 检查 TA-Lib 是否使用了 MACD[25:34] 来计算 Signal[33]
print("\n[6] 检查 TA-Lib 使用的 MACD 值（包括索引 33）")
print("-" * 80)
macd_25_to_34 = macd_full[25:34]
print(f"MACD[25:34]: {macd_25_to_34}")
print(f"MACD[25:34] 的 SMA: {np.mean(macd_25_to_34):.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(np.mean(macd_25_to_34) - signal_talib[33]):.2e}")

# 方法6: 检查 TA-Lib 是否使用了不同的 Signal 计算算法
print("\n[7] 检查 Signal 计算算法")
print("-" * 80)
# 尝试使用 Wilder's smoothing 计算 Signal
def wilder_rma(data, period):
    result = np.full_like(data, np.nan)
    if len(data) < period:
        return result
    
    # 初始值 = SMA
    result[period-1] = np.mean(data[:period])
    
    # RMA: RMA[i] = (RMA[i-1] * (period-1) + data[i]) / period
    for i in range(period, len(data)):
        result[i] = (result[i-1] * (period - 1) + data[i]) / period
    
    return result

# 从索引 25 开始使用 Wilder's smoothing
signal_rma_25 = wilder_rma(macd_full[25:], 9)
print(f"从索引 25 开始 RMA Signal[33]: {signal_rma_25[33-25]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_rma_25[33-25] - signal_talib[33]):.2e}")

# 从索引 32 开始使用 Wilder's smoothing
signal_rma_32 = wilder_rma(macd_full[32:], 9)
print(f"从索引 32 开始 RMA Signal[33]: {signal_rma_32[33-32]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_rma_32[33-32] - signal_talib[33]):.2e}")

print("\n" + "=" * 80)
print("测试完成")
print("=" * 80)
