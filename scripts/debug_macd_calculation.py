#!/usr/bin/env python3
"""
MACD 计算方式深度分析
"""
import numpy as np
import talib

# 生成测试数据
np.random.seed(42)
n = 100
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("MACD 计算方式深度分析")
print("=" * 80)

# 计算 TA-Lib 指标
talib_ema12 = talib.EMA(close, 12)
talib_ema26 = talib.EMA(close, 26)
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)

print("\n[1] 检查 MACD 的计算方式")
print("-" * 80)

# 方法1: 标准 MACD = EMA12 - EMA26
macd_method1 = talib_ema12 - talib_ema26

print(f"索引       EMA12             EMA26             EMA12-EMA26       TA-Lib MACD     差异")
print("-" * 100)
for i in [25, 26, 27, 30, 33, 40]:
    ema12_val = talib_ema12[i]
    ema26_val = talib_ema26[i]
    manual_macd = macd_method1[i]
    talib_val = talib_macd[i]
    
    if not np.isnan(manual_macd) and not np.isnan(talib_val):
        diff = abs(manual_macd - talib_val)
        print(f"{i:<8} {ema12_val:<18.10f} {ema26_val:<18.10f} {manual_macd:<18.10f} {talib_val:<18.10f} {diff:<18.2e}")
    else:
        print(f"{i:<8} {ema12_val:<18.10f} {ema26_val:<18.10f} {'NaN':<18} {'NaN':<18} {'N/A':<18}")

# 检查 TA-Lib MACD 的第一个有效值位置
print("\n[2] 检查 TA-Lib MACD 的第一个有效值位置")
print("-" * 80)
first_valid_macd = np.where(~np.isnan(talib_macd))[0][0]
print(f"TA-Lib MACD 第一个有效值索引: {first_valid_macd}")
print(f"预期索引 (slow_period-1): 25")
print(f"差异: {first_valid_macd - 25}")

# 检查 TA-Lib 是否使用了不同的 MACD 计算起点
print("\n[3] 分析 TA-Lib MACD 的可能计算方式")
print("-" * 80)

# 假设1: TA-Lib 可能从索引 33 开始计算 MACD（而不是 25）
# 这意味着 TA-Lib 可能在计算 MACD 之前等待 Signal line 准备好
print("假设1: TA-Lib 可能使用了延迟计算")
print(f"  如果 MACD 从索引 33 开始计算，那么需要 33 个数据点")
print(f"  33 = slow_period (26) + signal_period (9) - 2")

# 假设2: 检查 TA-Lib 是否使用了不同的 EMA 计算方式
# 可能 TA-Lib 在计算 MACD 时使用了不同的 EMA 参数
print("\n假设2: 检查 TA-Lib 是否使用了不同的 EMA 组合")

# 测试不同的 EMA 组合
for fast in [12, 13, 14]:
    for slow in [26, 27, 28]:
        ema_fast = talib.EMA(close, fast)
        ema_slow = talib.EMA(close, slow)
        macd_test = ema_fast - ema_slow
        
        if not np.isnan(macd_test[33]):
            diff = abs(macd_test[33] - talib_macd[33])
            if diff < 1e-6:
                print(f"  匹配! EMA({fast}) - EMA({slow}) = MACD[33]")
                print(f"    计算值: {macd_test[33]:.10f}")
                print(f"    TA-Lib: {talib_macd[33]:.10f}")
                print(f"    差异: {diff:.2e}")

# 假设3: 检查 TA-Lib 是否使用了 MACDFIX 或其他变体
print("\n[4] 检查 TA-Lib 的其他 MACD 函数")
print("-" * 80)

# MACDFIX: 使用固定的 12 周期 EMA
macdfix, signal_fix, hist_fix = talib.MACDFIX(close, signalperiod=9)
print(f"MACDFIX[33]: {macdfix[33]:.10f}")
print(f"MACD[33]:    {talib_macd[33]:.10f}")
print(f"差异: {abs(macdfix[33] - talib_macd[33]):.2e}")

# MACDEXT: 可扩展的 MACD
macdext, signal_ext, hist_ext = talib.MACDEXT(
    close, 
    fastperiod=12, fastmatype=1,  # EMA
    slowperiod=26, slowmatype=1,  # EMA
    signalperiod=9, signalmatype=1  # EMA
)
print(f"\nMACDEXT[33]: {macdext[33]:.10f}")
print(f"MACD[33]:    {talib_macd[33]:.10f}")
print(f"差异: {abs(macdext[33] - talib_macd[33]):.2e}")

# 假设4: 检查 TA-Lib 是否在计算 MACD 时使用了不同的数据范围
print("\n[5] 检查 TA-Lib 是否使用了不同的数据范围")
print("-" * 80)

# 可能 TA-Lib 在计算 EMA 时使用了不同的起始位置
for offset in range(0, 5):
    # 从 offset 开始计算 EMA
    close_offset = close[offset:]
    ema12_offset = talib.EMA(close_offset, 12)
    ema26_offset = talib.EMA(close_offset, 26)
    
    if len(ema12_offset) > 33 and len(ema26_offset) > 33:
        macd_offset = ema12_offset - ema26_offset
        idx = 33 - offset
        
        if idx >= 0 and idx < len(macd_offset) and not np.isnan(macd_offset[idx]):
            diff = abs(macd_offset[idx] - talib_macd[33])
            print(f"从索引 {offset} 开始: MACD[{idx}] = {macd_offset[idx]:.10f}, 差异 = {diff:.2e}")

# 假设5: 检查 TA-Lib 是否使用了不同的平滑因子
print("\n[6] 检查 TA-Lib 是否使用了不同的平滑因子")
print("-" * 80)

# 标准 EMA 的平滑因子: k = 2/(period+1)
# 可能 TA-Lib 使用了不同的 k 值

def custom_ema(data, period, k):
    ema = np.full_like(data, np.nan)
    ema[period-1] = np.mean(data[:period])
    for i in range(period, len(data)):
        ema[i] = data[i] * k + ema[i-1] * (1 - k)
    return ema

# 测试不同的 k 值
k_standard_12 = 2.0 / (12 + 1)
k_standard_26 = 2.0 / (26 + 1)

print(f"标准 k12 = {k_standard_12:.10f}")
print(f"标准 k26 = {k_standard_26:.10f}")

# 尝试微调 k 值
for delta in [-0.01, -0.005, -0.001, 0, 0.001, 0.005, 0.01]:
    k12_test = k_standard_12 + delta
    k26_test = k_standard_26 + delta
    
    ema12_test = custom_ema(close, 12, k12_test)
    ema26_test = custom_ema(close, 26, k26_test)
    macd_test = ema12_test - ema26_test
    
    if not np.isnan(macd_test[33]):
        diff = abs(macd_test[33] - talib_macd[33])
        print(f"k 偏移 {delta:+.4f}: MACD[33] = {macd_test[33]:.10f}, 差异 = {diff:.2e}")

print("\n" + "=" * 80)
print("分析完成")
print("=" * 80)
