#!/usr/bin/env python3
"""
EMA 实现方式深度分析
"""
import numpy as np
import talib

# 生成测试数据
np.random.seed(42)
n = 100
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("EMA 实现方式深度分析")
print("=" * 80)

# 计算 TA-Lib EMA
talib_ema12 = talib.EMA(close, 12)
talib_ema26 = talib.EMA(close, 26)

print("\n[1] TA-Lib EMA 初始值分析")
print("-" * 80)
print(f"EMA12[11] (第一个有效值): {talib_ema12[11]:.10f}")
print(f"close[0:12] 的 SMA: {np.mean(close[0:12]):.10f}")
print(f"差异: {abs(talib_ema12[11] - np.mean(close[0:12])):.2e}")

print(f"\nEMA26[25] (第一个有效值): {talib_ema26[25]:.10f}")
print(f"close[0:26] 的 SMA: {np.mean(close[0:26]):.10f}")
print(f"差异: {abs(talib_ema26[25] - np.mean(close[0:26])):.2e}")

# 方法1: 标准 EMA (SMA 作为初始值)
def ema_method1(data, period):
    ema = np.full_like(data, np.nan)
    # 初始值 = SMA
    ema[period-1] = np.mean(data[:period])
    # EMA: EMA[i] = data[i] * k + EMA[i-1] * (1-k)
    k = 2.0 / (period + 1)
    for i in range(period, len(data)):
        ema[i] = data[i] * k + ema[i-1] * (1 - k)
    return ema

# 方法2: Wilder's RMA (SMA 作为初始值)
def ema_method2(data, period):
    ema = np.full_like(data, np.nan)
    # 初始值 = SMA
    ema[period-1] = np.mean(data[:period])
    # RMA: RMA[i] = (RMA[i-1] * (period-1) + data[i]) / period
    for i in range(period, len(data)):
        ema[i] = (ema[i-1] * (period - 1) + data[i]) / period
    return ema

# 方法3: 使用第一个值作为初始值
def ema_method3(data, period):
    ema = np.full_like(data, np.nan)
    # 初始值 = 第一个值
    ema[0] = data[0]
    # EMA: EMA[i] = data[i] * k + EMA[i-1] * (1-k)
    k = 2.0 / (period + 1)
    for i in range(1, len(data)):
        ema[i] = data[i] * k + ema[i-1] * (1 - k)
    return ema

ema_m1 = ema_method1(close, 12)
ema_m2 = ema_method2(close, 12)
ema_m3 = ema_method3(close, 12)

print("\n[2] 不同 EMA 方法对比 (period=12)")
print("-" * 80)
print(f"索引       TA-Lib EMA12      方法1 (标准EMA)   方法2 (RMA)       方法3 (首值)")
print("-" * 80)
for i in [11, 12, 13, 20, 30]:
    print(f"{i:<8} {talib_ema12[i]:<18.10f} {ema_m1[i]:<18.10f} {ema_m2[i]:<18.10f} {ema_m3[i]:<18.10f}")

print("\n[3] 差异分析")
print("-" * 80)
print(f"索引       TA-Lib vs 方法1   TA-Lib vs 方法2   TA-Lib vs 方法3")
print("-" * 80)
for i in [11, 12, 13, 20, 30]:
    diff1 = abs(talib_ema12[i] - ema_m1[i])
    diff2 = abs(talib_ema12[i] - ema_m2[i])
    diff3 = abs(talib_ema12[i] - ema_m3[i])
    print(f"{i:<8} {diff1:<18.2e} {diff2:<18.2e} {diff3:<18.2e}")

# 分析 MACD line 的计算
print("\n[4] MACD line 计算分析")
print("-" * 80)
macd_line = talib_ema12 - talib_ema26
talib_macd, _, _ = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

print(f"索引       手动计算 MACD     TA-Lib MACD       差异")
print("-" * 80)
for i in [25, 26, 27, 30, 33]:
    manual_macd = macd_line[i]
    talib_val = talib_macd[i]
    if not np.isnan(manual_macd) and not np.isnan(talib_val):
        diff = abs(manual_macd - talib_val)
        print(f"{i:<8} {manual_macd:<18.10f} {talib_val:<18.10f} {diff:<18.2e}")
    else:
        print(f"{i:<8} {'NaN':<18} {'NaN':<18} {'N/A':<18}")

# 检查 TA-Lib 是否使用了不同的 MACD 计算方式
print("\n[5] TA-Lib MACD 可能的实现方式")
print("-" * 80)

# 假设1: TA-Lib 使用了 RMA 而不是 EMA
macd_rma = ema_m2 - ema_method2(close, 26)
print(f"假设1 (RMA): MACD[33] = {macd_rma[33]:.10f}")
print(f"TA-Lib:      MACD[33] = {talib_macd[33]:.10f}")
print(f"差异: {abs(macd_rma[33] - talib_macd[33]):.2e}")

# 假设2: TA-Lib 使用了不同的初始值
# 检查 EMA 的初始值是否使用了不同的 SMA 范围
print("\n[6] 检查 EMA 初始值的 SMA 范围")
print("-" * 80)
for start in range(0, 3):
    for end_offset in range(0, 3):
        end = 12 + end_offset
        if end <= len(close):
            sma_val = np.mean(close[start:end])
            diff = abs(sma_val - talib_ema12[11])
            if diff < 1e-6:
                print(f"匹配! close[{start}:{end}] 的 SMA = {sma_val:.10f}")
                print(f"  TA-Lib EMA12[11] = {talib_ema12[11]:.10f}")
                print(f"  差异 = {diff:.2e}")

print("\n" + "=" * 80)
print("分析完成")
print("=" * 80)
