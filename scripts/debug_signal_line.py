#!/usr/bin/env python3
"""
深入分析 TA-Lib Signal line 的实现细节
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import talib
    import finkit
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# 生成随机测试数据
np.random.seed(42)
n = 100
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("Signal Line 实现细节分析")
print("=" * 80)

# 计算 EMA
ema12 = talib.EMA(close, timeperiod=12)
ema26 = talib.EMA(close, timeperiod=26)

# 计算 MACD line
macd_line_manual = ema12 - ema26

# 获取 TA-Lib MACD
macd_talib, signal_talib, hist_talib = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

print("\n[1] 检查 MACD line 的第一个有效值")
print("-" * 80)
first_valid_idx = 25 + 9 - 2  # slow_period + signal_period - 2 = 32
print(f"TA-Lib MACD 第一个有效值索引: {np.where(~np.isnan(macd_talib))[0][0]}")
print(f"手动计算 MACD 第一个有效值索引: {np.where(~np.isnan(macd_line_manual))[0][0]}")

print("\n[2] 对比 MACD line 值")
print("-" * 80)
for i in range(25, 35):
    print(f"Index {i}:")
    print(f"  手动 MACD:  {macd_line_manual[i]:.10f}")
    print(f"  TA-Lib MACD: {macd_talib[i]:.10f}")
    print(f"  差异: {abs(macd_line_manual[i] - macd_talib[i]):.2e}")

print("\n[3] 分析 Signal line 的计算方式")
print("-" * 80)

# 方法1: 对所有 MACD line 值（包括 NaN）应用 EMA
print("\n方法1: 对所有 MACD line 值应用 EMA")
# 提取有效的 MACD 值
valid_macd = macd_line_manual[25:]  # 从 slow_period - 1 开始
print(f"有效 MACD 值数量: {len(valid_macd)}")

# 对有效 MACD 值应用 EMA
signal_method1 = talib.EMA(valid_macd, timeperiod=9)
print(f"方法1 Signal[33]: {signal_method1[33-25]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_method1[33-25] - signal_talib[33]):.2e}")

# 方法2: 只对有效 MACD 值应用 EMA，然后对齐索引
print("\n方法2: 只对有效 MACD 值应用 EMA，然后对齐索引")
valid_macd_clean = macd_line_manual[25:35]  # 取前10个有效值
signal_method2 = talib.EMA(valid_macd_clean, timeperiod=9)
print(f"有效 MACD 值: {valid_macd_clean}")
print(f"方法2 Signal: {signal_method2}")

# 方法3: 检查 TA-Lib 是否使用了不同的初始值
print("\n方法3: 检查 Signal line 的初始值")
# Signal line 的第一个有效值应该在索引 32
# 检查索引 32 的值
print(f"TA-Lib Signal[32]: {signal_talib[32]:.10f}")
print(f"TA-Lib MACD[25:33]: {macd_talib[25:33]}")

# 计算索引 25-32 的 MACD 值的 SMA
macd_25_to_32 = macd_talib[25:33]
print(f"MACD[25:33] 的 SMA: {np.mean(macd_25_to_32):.10f}")
print(f"差异: {abs(np.mean(macd_25_to_32) - signal_talib[32]):.2e}")

# 方法4: 检查是否使用了 Wilder's smoothing
print("\n方法4: 检查是否使用了 Wilder's smoothing (RMA)")
# RMA: signal[i] = (signal[i-1] * (period-1) + macd[i]) / period
signal_rma = np.full_like(close, np.nan)
signal_rma[32] = np.mean(macd_talib[25:33])  # 初始值 = SMA
for i in range(33, len(close)):
    signal_rma[i] = (signal_rma[i-1] * 8 + macd_talib[i]) / 9

print(f"RMA Signal[33]: {signal_rma[33]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"差异: {abs(signal_rma[33] - signal_talib[33]):.2e}")

print("\n[4] 对比 AlphaTA Signal line")
print("-" * 80)
alpha_result = finkit.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd = np.array(alpha_result[0])
alpha_signal = np.array(alpha_result[1])

print(f"AlphaTA MACD[33]: {alpha_macd[33]:.10f}")
print(f"TA-Lib MACD[33]: {macd_talib[33]:.10f}")
print(f"MACD 差异: {abs(alpha_macd[33] - macd_talib[33]):.2e}")

print(f"\nAlphaTA Signal[33]: {alpha_signal[33]:.10f}")
print(f"TA-Lib Signal[33]: {signal_talib[33]:.10f}")
print(f"Signal 差异: {abs(alpha_signal[33] - signal_talib[33]):.2e}")

print("\n" + "=" * 80)
print("分析完成")
print("=" * 80)
