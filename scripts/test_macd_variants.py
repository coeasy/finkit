#!/usr/bin/env python3
"""
测试 TA-Lib MACD 的不同实现方式
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
print("TA-Lib MACD 实现方式测试")
print("=" * 80)

# 获取 TA-Lib MACD
macd_talib, signal_talib, hist_talib = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# 方法1: 标准 EMA 差值
ema12 = talib.EMA(close, timeperiod=12)
ema26 = talib.EMA(close, timeperiod=26)
macd_method1 = ema12 - ema26

print("\n[1] 方法1: 标准 EMA 差值 (EMA12 - EMA26)")
print("-" * 80)
print(f"MACD[33] 差异: {abs(macd_method1[33] - macd_talib[33]):.2e}")

# 方法2: 使用 SMA 作为初始值的 EMA
def custom_ema_sma(data, period):
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

ema12_custom = custom_ema_sma(close, 12)
ema26_custom = custom_ema_sma(close, 26)
macd_method2 = ema12_custom - ema26_custom

print("\n[2] 方法2: 自定义 EMA (SMA 初始值)")
print("-" * 80)
print(f"EMA12[33] 差异: {abs(ema12_custom[33] - ema12[33]):.2e}")
print(f"EMA26[33] 差异: {abs(ema26_custom[33] - ema26[33]):.2e}")
print(f"MACD[33] 差异: {abs(macd_method2[33] - macd_talib[33]):.2e}")

# 方法3: 使用 Wilder's smoothing (RMA)
def custom_rma(data, period):
    result = np.full_like(data, np.nan)
    if len(data) < period:
        return result
    
    # 初始值 = SMA
    result[period-1] = np.mean(data[:period])
    
    # RMA: RMA[i] = (RMA[i-1] * (period-1) + data[i]) / period
    for i in range(period, len(data)):
        result[i] = (result[i-1] * (period - 1) + data[i]) / period
    
    return result

ema12_rma = custom_rma(close, 12)
ema26_rma = custom_rma(close, 26)
macd_method3 = ema12_rma - ema26_rma

print("\n[3] 方法3: Wilder's RMA")
print("-" * 80)
print(f"EMA12[33] 差异: {abs(ema12_rma[33] - ema12[33]):.2e}")
print(f"EMA26[33] 差异: {abs(ema26_rma[33] - ema26[33]):.2e}")
print(f"MACD[33] 差异: {abs(macd_method3[33] - macd_talib[33]):.2e}")

# 方法4: 检查 TA-Lib 是否使用了不同的 MACD 计算
# TA-Lib 可能使用了 MACDFIX 或其他变体
print("\n[4] 检查 TA-Lib 的其他 MACD 函数")
print("-" * 80)

# MACDFIX: 使用固定的 EMA 周期
macdfix, signal_fix, hist_fix = talib.MACDFIX(close, signalperiod=9)
print(f"MACDFIX[33]: {macdfix[33]:.10f}")
print(f"MACD[33]: {macd_talib[33]:.10f}")
print(f"差异: {abs(macdfix[33] - macd_talib[33]):.2e}")

# MACDEXT: 使用不同的 MA 类型
macdext, signal_ext, hist_ext = talib.MACDEXT(close, fastperiod=12, fastmatype=1,  # EMA
                                               slowperiod=26, slowmatype=1,  # EMA
                                               signalperiod=9, signalmatype=1)  # EMA
print(f"\nMACDEXT[33]: {macdext[33]:.10f}")
print(f"MACD[33]: {macd_talib[33]:.10f}")
print(f"差异: {abs(macdext[33] - macd_talib[33]):.2e}")

# 方法5: 检查 Signal line 的计算方式
print("\n[5] Signal line 计算方式分析")
print("-" * 80)

# 假设 MACD line 是正确的，检查 Signal line
# Signal line 应该是对 MACD line 的 EMA
macd_correct = macd_talib[25:]  # 从索引 25 开始的有效值
signal_from_correct = talib.EMA(macd_correct, timeperiod=9)

print(f"从 TA-Lib MACD 计算的 Signal[33]: {signal_from_correct[33-25]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_from_correct[33-25] - signal_talib[33]):.2e}")

# 方法6: 检查是否 Signal line 使用了不同的初始值
print("\n[6] Signal line 初始值分析")
print("-" * 80)

# Signal line 的第一个有效值应该在索引 32
# 但 TA-Lib 在索引 33 之前都是 NaN
# 这说明 Signal line 可能需要等待 MACD line 有足够的值

# 尝试：Signal line 的第一个值 = MACD[25:33] 的 SMA
macd_25_to_33 = macd_talib[25:33]
print(f"MACD[25:33]: {macd_25_to_33}")
print(f"MACD[25:33] 的 SMA: {np.mean(macd_25_to_33):.10f}")

# 但 TA-Lib 在索引 32 是 NaN，说明可能不是这样计算的

print("\n" + "=" * 80)
print("测试完成")
print("=" * 80)
