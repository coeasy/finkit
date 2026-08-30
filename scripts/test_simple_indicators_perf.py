#!/usr/bin/env python3
"""
测试简单指标的 Python FFI 性能优化效果
"""
import sys
import time
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
high = close + np.abs(np.random.randn(n))
low = close - np.abs(np.random.randn(n))
open_price = close + np.random.randn(n) * 0.5

print("=" * 80)
print("简单指标 Python FFI 性能优化测试")
print("=" * 80)
print(f"数据规模: {n} 个数据点")
print(f"测试次数: 100 次")
print()

# 测试简单指标
test_cases = [
    ('MOM', lambda: finkit.mom(close, 10), lambda: talib.MOM(close, 10)),
    ('ROC', lambda: finkit.roc(close, 10), lambda: talib.ROC(close, 10)),
    ('BOP', lambda: finkit.bop(open_price, high, low, close), lambda: talib.BOP(open_price, high, low, close)),
    ('TRANGE', lambda: finkit.trange(high, low, close), lambda: talib.TRANGE(high, low, close)),
    ('AVGPRICE', lambda: finkit.avgprice(open_price, high, low, close), lambda: talib.AVGPRICE(open_price, high, low, close)),
    ('MEDPRICE', lambda: finkit.medprice(high, low), lambda: talib.MEDPRICE(high, low)),
    ('TYPPRICE', lambda: finkit.typprice(high, low, close), lambda: talib.TYPPRICE(high, low, close)),
    ('WCLPRICE', lambda: finkit.wclprice(high, low, close), lambda: talib.WCLPRICE(high, low, close)),
]

results = []

for name, alpha_func, talib_func in test_cases:
    # 预热
    for _ in range(10):
        alpha_func()
        talib_func()
    
    # 测试 AlphaTA
    start = time.perf_counter()
    for _ in range(100):
        alpha_func()
    alpha_time = time.perf_counter() - start
    
    # 测试 TA-Lib
    start = time.perf_counter()
    for _ in range(100):
        talib_func()
    talib_time = time.perf_counter() - start
    
    speedup = talib_time / alpha_time
    results.append((name, alpha_time, talib_time, speedup))
    
    status = "✅" if speedup >= 0.8 else "⚠️" if speedup >= 0.5 else "❌"
    print(f"{name:12s} | AlphaTA: {alpha_time*10:.3f}ms | TA-Lib: {talib_time*10:.3f}ms | 加速比: {speedup:.2f}x {status}")

print()
print("=" * 80)
print("总结:")
avg_speedup = sum(r[3] for r in results) / len(results)
print(f"平均加速比: {avg_speedup:.2f}x")
print(f"优化指标数: {len(results)}")
print("=" * 80)
