#!/usr/bin/env python3
"""
测试完整的 MACD 序列计算方法
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
print("完整 MACD 序列测试")
print("=" * 80)

# 获取 TA-Lib MACD
macd_talib, signal_talib, hist_talib = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# 计算 EMA
ema12 = talib.EMA(close, timeperiod=12)
ema26 = talib.EMA(close, timeperiod=26)

# 方法1: 计算完整的 MACD 序列（包括 NaN 位置）
print("\n[1] 完整 MACD 序列计算")
print("-" * 80)

# 将 NaN 替换为 0 进行计算
ema12_filled = np.nan_to_num(ema12, nan=0.0)
ema26_filled = np.nan_to_num(ema26, nan=0.0)
macd_full = ema12_filled - ema26_filled

print(f"EMA12[20]: {ema12[20]:.10f}, EMA12_filled[20]: {ema12_filled[20]:.10f}")
print(f"EMA26[20]: {ema26[20]:.10f}, EMA26_filled[20]: {ema26_filled[20]:.10f}")
print(f"MACD_full[20]: {macd_full[20]:.10f}")

# 对完整序列计算 Signal
signal_from_full = talib.EMA(macd_full, timeperiod=9)

print(f"\n从完整序列计算的 Signal[33]: {signal_from_full[33]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_full[33] - signal_talib[33]):.2e}")

# 方法2: 只使用有效值计算
print("\n[2] 只使用有效值计算")
print("-" * 80)

# 找到第一个有效 MACD 的索引
first_valid = np.where(~np.isnan(macd_talib))[0][0]
print(f"第一个有效 MACD 索引: {first_valid}")

# 提取有效值
macd_valid = macd_talib[first_valid:]
signal_from_valid = talib.EMA(macd_valid, timeperiod=9)

print(f"从有效值计算的 Signal[33]: {signal_from_valid[33-first_valid]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_valid[33-first_valid] - signal_talib[33]):.2e}")

# 方法3: 检查 TA-Lib 是否使用了不同的 Signal 计算起点
print("\n[3] Signal 计算起点分析")
print("-" * 80)

# 假设 Signal 从索引 25 开始计算（slow_period - 1）
# 但使用完整的 MACD 序列
macd_from_25 = macd_full[25:]
signal_from_25 = talib.EMA(macd_from_25, timeperiod=9)

print(f"从索引 25 开始计算的 Signal[33]: {signal_from_25[33-25]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_25[33-25] - signal_talib[33]):.2e}")

# 方法4: 检查是否 Signal 使用了 Wilder's smoothing
print("\n[4] Signal 使用 Wilder's smoothing 测试")
print("-" * 80)

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

# 方法5: 检查 MACD line 本身的计算
print("\n[5] MACD line 计算分析")
print("-" * 80)

print(f"EMA12[33]: {ema12[33]:.10f}")
print(f"EMA26[33]: {ema26[33]:.10f}")
print(f"EMA12 - EMA26: {ema12[33] - ema26[33]:.10f}")
print(f"TA-Lib MACD[33]: {macd_talib[33]:.10f}")
print(f"差异: {abs(ema12[33] - ema26[33] - macd_talib[33]):.2e}")

# 检查是否使用了不同的 EMA 算法
print("\n检查 EMA 算法：")
print(f"EMA12[11] (第一个有效值): {ema12[11]:.10f}")
print(f"前12个值的 SMA: {np.mean(close[:12]):.10f}")
print(f"差异: {abs(ema12[11] - np.mean(close[:12])):.2e}")

# 方法6: 检查是否 MACD 使用了不同的初始值
print("\n[6] MACD 初始值分析")
print("-" * 80)

# 尝试使用不同的初始值计算 EMA
def ema_with_custom_init(data, period, init_value):
    result = np.full_like(data, np.nan)
    if len(data) < period:
        return result
    
    result[period-1] = init_value
    
    k = 2.0 / (period + 1)
    for i in range(period, len(data)):
        result[i] = data[i] * k + result[i-1] * (1 - k)
    
    return result

# 尝试不同的初始值
for init_offset in [-0.5, -0.25, 0.0, 0.25, 0.5]:
    ema12_custom = ema_with_custom_init(close, 12, np.mean(close[:12]) + init_offset)
    ema26_custom = ema_with_custom_init(close, 26, np.mean(close[:26]) + init_offset)
    macd_custom = ema12_custom - ema26_custom
    
    diff = abs(macd_custom[33] - macd_talib[33])
    print(f"初始值偏移 {init_offset:+.2f}: MACD[33] 差异 = {diff:.2e}")

print("\n" + "=" * 80)
print("测试完成")
print("=" * 80)
