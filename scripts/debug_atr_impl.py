#!/usr/bin/env python3
"""
深入分析 TA-Lib ATR 的实现细节
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import talib
    import alpha_ta
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# 生成测试数据
np.random.seed(42)
n = 10000
close = np.cumsum(np.random.randn(n)) + 100
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))

print("=" * 80)
print("ATR 实现细节深度分析")
print("=" * 80)

# 计算 True Range
tr = np.zeros(n)
tr[0] = high[0] - low[0]
for i in range(1, n):
    tr[i] = max(high[i] - low[i], abs(high[i] - close[i-1]), abs(low[i] - close[i-1]))

print("\n[1] True Range 对比")
print("-" * 80)
print(f"TR[0]: {tr[0]:.10f}")
print(f"TR[13]: {tr[13]:.10f}")
print(f"TR[100]: {tr[100]:.10f}")

# 方法1: 使用 SMA 作为初始值，然后 Wilder's RMA
def atr_method1(tr, period):
    atr = np.full_like(tr, np.nan)
    # 第一个 ATR = SMA(TR, period)
    atr[period-1] = np.mean(tr[:period])
    # Wilder's RMA
    for i in range(period, len(tr)):
        atr[i] = (atr[i-1] * (period - 1) + tr[i]) / period
    return atr

# 方法2: 使用第一个 TR 作为初始值，然后 Wilder's RMA
def atr_method2(tr, period):
    atr = np.full_like(tr, np.nan)
    # 第一个 ATR = TR[0]
    atr[0] = tr[0]
    # Wilder's RMA
    for i in range(1, len(tr)):
        atr[i] = (atr[i-1] * (period - 1) + tr[i]) / period
    return atr

# 方法3: 使用 SMA 作为初始值，但从 period 开始输出
def atr_method3(tr, period):
    atr = np.full_like(tr, np.nan)
    # 第一个 ATR = SMA(TR, period)
    atr[period] = np.mean(tr[1:period+1])  # 从索引 1 开始
    # Wilder's RMA
    for i in range(period+1, len(tr)):
        atr[i] = (atr[i-1] * (period - 1) + tr[i]) / period
    return atr

# 获取 TA-Lib ATR
atr_talib = talib.ATR(high, low, close, timeperiod=14)

print("\n[2] 不同方法对比")
print("-" * 80)

atr_m1 = atr_method1(tr, 14)
atr_m2 = atr_method2(tr, 14)
atr_m3 = atr_method3(tr, 14)

print(f"TA-Lib ATR[14]:   {atr_talib[14]:.10f}")
print(f"方法1 ATR[14]:    {atr_m1[14]:.10f}")
print(f"方法2 ATR[14]:    {atr_m2[14]:.10f}")
print(f"方法3 ATR[14]:    {atr_m3[14]:.10f}")
print(f"AlphaTA ATR[14]:  {np.array(alpha_ta.atr(high, low, close, timeperiod=14))[14]:.10f}")

print(f"\n差异:")
print(f"方法1 vs TA-Lib:  {abs(atr_m1[14] - atr_talib[14]):.2e}")
print(f"方法2 vs TA-Lib:  {abs(atr_m2[14] - atr_talib[14]):.2e}")
print(f"方法3 vs TA-Lib:  {abs(atr_m3[14] - atr_talib[14]):.2e}")

print("\n[3] 检查 TA-Lib 的初始值")
print("-" * 80)
# 检查 TA-Lib 是否使用了不同的初始值
print(f"TR[0:14] 的 SMA: {np.mean(tr[:14]):.10f}")
print(f"TR[1:15] 的 SMA: {np.mean(tr[1:15]):.10f}")
print(f"TA-Lib ATR[13]:  {atr_talib[13]:.10f}")
print(f"TA-Lib ATR[14]:  {atr_talib[14]:.10f}")

# 方法4: 检查是否使用了 EMA 而不是 RMA
def atr_method4(tr, period):
    atr = np.full_like(tr, np.nan)
    # 第一个 ATR = SMA(TR, period)
    atr[period-1] = np.mean(tr[:period])
    # EMA: k = 2/(period+1)
    k = 2.0 / (period + 1)
    for i in range(period, len(tr)):
        atr[i] = tr[i] * k + atr[i-1] * (1 - k)
    return atr

atr_m4 = atr_method4(tr, 14)
print(f"\n方法4 (EMA) ATR[14]: {atr_m4[14]:.10f}")
print(f"方法4 vs TA-Lib:    {abs(atr_m4[14] - atr_talib[14]):.2e}")

print("\n[4] 检查 AlphaTA 的实现")
print("-" * 80)
alpha_atr = np.array(alpha_ta.atr(high, low, close, timeperiod=14))
print(f"AlphaTA ATR[13]: {alpha_atr[13]:.10f}")
print(f"AlphaTA ATR[14]: {alpha_atr[14]:.10f}")
print(f"TA-Lib ATR[13]:  {atr_talib[13]:.10f}")
print(f"TA-Lib ATR[14]:  {atr_talib[14]:.10f}")

print("\n[5] 详细对比（索引 100-110）")
print("-" * 80)
for i in range(100, 110):
    print(f"Index {i}:")
    print(f"  TA-Lib:  {atr_talib[i]:.10f}")
    print(f"  AlphaTA: {alpha_atr[i]:.10f}")
    print(f"  方法1:   {atr_m1[i]:.10f}")
    print(f"  差异:    {abs(atr_talib[i] - alpha_atr[i]):.2e}")

print("\n" + "=" * 80)
print("分析完成")
print("=" * 80)
