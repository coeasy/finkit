#!/usr/bin/env python3
"""
MACD Signal line 精度诊断脚本
"""
import numpy as np
import talib
import alpha_ta

# 生成测试数据
np.random.seed(42)
n = 1000
close = np.cumsum(np.random.randn(n)) + 100

# 计算 TA-Lib MACD
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

# 计算 AlphaTA MACD
alpha_result = alpha_ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd = alpha_result[0]
alpha_signal = alpha_result[1]
alpha_hist = alpha_result[2]

print("=" * 80)
print("MACD Signal line 精度诊断")
print("=" * 80)

# 找到第一个有效 Signal 的索引
first_valid_idx = np.where(~np.isnan(talib_signal))[0][0]
print(f"\n第一个有效 Signal 索引: {first_valid_idx}")
print(f"TA-Lib Signal[{first_valid_idx}]: {talib_signal[first_valid_idx]:.10f}")
print(f"AlphaTA Signal[{first_valid_idx}]: {alpha_signal[first_valid_idx]:.10f}")
print(f"差异: {abs(talib_signal[first_valid_idx] - alpha_signal[first_valid_idx]):.2e}")

# 手动计算 MACD line
ema12 = talib.EMA(close, 12)
ema26 = talib.EMA(close, 26)
macd_line = ema12 - ema26

print(f"\n手动计算 MACD[{first_valid_idx}]: {macd_line[first_valid_idx]:.10f}")
print(f"TA-Lib MACD[{first_valid_idx}]: {talib_macd[first_valid_idx]:.10f}")
print(f"差异: {abs(macd_line[first_valid_idx] - talib_macd[first_valid_idx]):.2e}")

# 分析 Signal line 的计算方式
print("\n" + "=" * 80)
print("Signal line 计算方式分析")
print("=" * 80)

# 方法1: 从 slow_period-1 开始对 MACD line 应用 EMA
print("\n方法1: 从索引 25 开始对 MACD line 应用 EMA")
macd_from_25 = macd_line[25:]
signal_method1 = talib.EMA(macd_from_25, 9)
print(f"Signal[{first_valid_idx}]: {signal_method1[first_valid_idx-25]:.10f}")
print(f"TA-Lib Signal[{first_valid_idx}]: {talib_signal[first_valid_idx]:.10f}")
print(f"差异: {abs(signal_method1[first_valid_idx-25] - talib_signal[first_valid_idx]):.2e}")

# 方法2: 使用 MACD[25:33] 的 SMA 作为 Signal 的初始值
print("\n方法2: 使用 MACD[25:33] 的 SMA 作为 Signal 初始值")
macd_25_to_33 = macd_line[25:33]
sma_init = np.mean(macd_25_to_33)
print(f"MACD[25:33] 的 SMA: {sma_init:.10f}")
print(f"TA-Lib Signal[33]: {talib_signal[33]:.10f}")
print(f"差异: {abs(sma_init - talib_signal[33]):.2e}")

# 方法3: 检查 TA-Lib 是否使用了不同的 Signal 计算起点
print("\n方法3: 检查 Signal 计算起点")
for start_idx in range(25, 35):
    macd_from_start = macd_line[start_idx:]
    if len(macd_from_start) >= 9:
        signal_test = talib.EMA(macd_from_start, 9)
        if not np.isnan(signal_test[0]):
            print(f"从索引 {start_idx} 开始: Signal[{first_valid_idx}] = {signal_test[first_valid_idx-start_idx]:.10f}")

# 详细对比
print("\n" + "=" * 80)
print("详细对比 (索引 30-40)")
print("=" * 80)
print(f"{'索引':<8} {'TA-Lib Signal':<20} {'AlphaTA Signal':<20} {'差异':<15}")
print("-" * 80)
for i in range(30, 41):
    talib_val = talib_signal[i]
    alpha_val = alpha_signal[i]
    if not np.isnan(talib_val) and not np.isnan(alpha_val):
        diff = abs(talib_val - alpha_val)
        print(f"{i:<8} {talib_val:<20.10f} {alpha_val:<20.10f} {diff:<15.2e}")
    else:
        print(f"{i:<8} {'NaN':<20} {'NaN':<20} {'N/A':<15}")

print("\n" + "=" * 80)
print("诊断完成")
print("=" * 80)
