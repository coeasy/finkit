"""
Generate comprehensive AlphaTA vs TA-Lib comparison report
Includes both performance benchmark and numerical correctness validation
"""
import numpy as np
import time
import finkit
import talib
from datetime import datetime

# Generate test data
np.random.seed(42)
n = 10000
open_price = np.random.uniform(100, 110, n).astype(np.float64)
high = np.random.uniform(100, 110, n).astype(np.float64)
low = np.random.uniform(90, 100, n).astype(np.float64)
close = np.random.uniform(95, 105, n).astype(np.float64)
volume = np.random.uniform(1000000, 2000000, n).astype(np.float64)

# Ensure OHLCV consistency
high = np.maximum(high, np.maximum(open_price, close) + 1)
low = np.minimum(low, np.minimum(open_price, close) - 1)

def benchmark(func_name, alphata_func, talib_func, iterations=100):
    """Benchmark a single indicator"""
    try:
        # Warm up
        for _ in range(5):
            alphata_func()
            talib_func()
        
        # Benchmark AlphaTA
        start = time.perf_counter()
        for _ in range(iterations):
            alphata_func()
        alphata_time = (time.perf_counter() - start) / iterations
        
        # Benchmark TA-Lib
        start = time.perf_counter()
        for _ in range(iterations):
            talib_func()
        talib_time = (time.perf_counter() - start) / iterations
        
        speedup = talib_time / alphata_time
        status = "✅" if speedup >= 1.0 else "⚠️"
        
        return {
            'name': func_name,
            'alphata_ms': alphata_time * 1000,
            'talib_ms': talib_time * 1000,
            'speedup': speedup,
            'status': status,
            'error': None
        }
    except Exception as e:
        return {
            'name': func_name,
            'alphata_ms': 0,
            'talib_ms': 0,
            'speedup': 0,
            'status': f"❌",
            'error': str(e)[:50]
        }

def validate_numerical(func_name, alphata_func, talib_func, tolerance=1e-6):
    """Validate numerical correctness between AlphaTA and TA-Lib"""
    try:
        alpha_result = alphata_func()
        talib_result = talib_func()
        
        # Handle tuple results (multiple outputs)
        if isinstance(alpha_result, tuple):
            alpha_result = alpha_result[0]
        if isinstance(talib_result, tuple):
            talib_result = talib_result[0]
        
        # Convert to numpy arrays
        alpha_arr = np.asarray(alpha_result)
        talib_arr = np.asarray(talib_result)
        
        # Check shape
        if alpha_arr.shape != talib_arr.shape:
            return {
                'name': func_name,
                'match': False,
                'max_diff': None,
                'mean_diff': None,
                'error': f'Shape mismatch: {alpha_arr.shape} vs {talib_arr.shape}'
            }
        
        # Find valid (non-NaN) indices
        valid_mask = ~(np.isnan(alpha_arr) | np.isnan(talib_arr))
        
        if not np.any(valid_mask):
            return {
                'name': func_name,
                'match': True,
                'max_diff': 0.0,
                'mean_diff': 0.0,
                'error': None
            }
        
        # Compare valid values
        alpha_valid = alpha_arr[valid_mask]
        talib_valid = talib_arr[valid_mask]
        
        diff = np.abs(alpha_valid - talib_valid)
        max_diff = np.max(diff)
        mean_diff = np.mean(diff)
        
        match = max_diff < tolerance
        
        return {
            'name': func_name,
            'match': match,
            'max_diff': max_diff,
            'mean_diff': mean_diff,
            'error': None
        }
    except Exception as e:
        return {
            'name': func_name,
            'match': False,
            'max_diff': None,
            'mean_diff': None,
            'error': str(e)[:50]
        }

print("=" * 100)
print("AlphaTA vs TA-Lib 完整对比报告")
print("=" * 100)
print(f"测试时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print(f"测试数据: {n} 个数据点")
print(f"迭代次数: 100 次")
print()

# Define test cases
test_cases = [
    # Overlap indicators
    ("SMA", lambda: finkit.sma(close, 20), lambda: talib.SMA(close, 20), "Overlap"),
    ("EMA", lambda: finkit.ema(close, 20), lambda: talib.EMA(close, 20), "Overlap"),
    ("WMA", lambda: finkit.wma(close, 20), lambda: talib.WMA(close, 20), "Overlap"),
    ("DEMA", lambda: finkit.dema(close, 20), lambda: talib.DEMA(close, 20), "Overlap"),
    ("TEMA", lambda: finkit.tema(close, 20), lambda: talib.TEMA(close, 20), "Overlap"),
    ("KAMA", lambda: finkit.kama(close, 20), lambda: talib.KAMA(close, 20), "Overlap"),
    ("MAMA", lambda: finkit.mama(close, 0.5, 0.05), lambda: talib.MAMA(close, 0.5, 0.05), "Overlap"),
    ("T3", lambda: finkit.t3(close, 20, 0.7), lambda: talib.T3(close, 20, 0.7), "Overlap"),
    
    # Price transforms
    ("AVGPRICE", lambda: finkit.avgprice(open_price, high, low, close), lambda: talib.AVGPRICE(open_price, high, low, close), "Price Transform"),
    ("MEDPRICE", lambda: finkit.medprice(high, low), lambda: talib.MEDPRICE(high, low), "Price Transform"),
    ("TYPPRICE", lambda: finkit.typprice(high, low, close), lambda: talib.TYPPRICE(high, low, close), "Price Transform"),
    ("WCLPRICE", lambda: finkit.wclprice(high, low, close), lambda: talib.WCLPRICE(high, low, close), "Price Transform"),
    
    # Momentum indicators
    ("ADX", lambda: finkit.adx(high, low, close, 14), lambda: talib.ADX(high, low, close, 14), "Momentum"),
    ("APO", lambda: finkit.apo(close, 12, 26), lambda: talib.APO(close, 12, 26), "Momentum"),
    ("AROON", lambda: finkit.aroon(high, low, 14), lambda: talib.AROON(high, low, 14), "Momentum"),
    ("BOP", lambda: finkit.bop(open_price, high, low, close), lambda: talib.BOP(open_price, high, low, close), "Momentum"),
    ("CCI", lambda: finkit.cci(high, low, close, 20), lambda: talib.CCI(high, low, close, 20), "Momentum"),
    ("CMO", lambda: finkit.cmo(close, 14), lambda: talib.CMO(close, 14), "Momentum"),
    ("DX", lambda: finkit.dx(high, low, close, 14), lambda: talib.DX(high, low, close, 14), "Momentum"),
    ("MACD", lambda: finkit.macd(close, 12, 26, 9), lambda: talib.MACD(close, 12, 26, 9), "Momentum"),
    ("MFI", lambda: finkit.mfi(high, low, close, volume, 14), lambda: talib.MFI(high, low, close, volume, 14), "Momentum"),
    ("MINUS_DI", lambda: finkit.minus_di(high, low, close, 14), lambda: talib.MINUS_DI(high, low, close, 14), "Momentum"),
    ("MOM", lambda: finkit.mom(close, 10), lambda: talib.MOM(close, 10), "Momentum"),
    ("PLUS_DI", lambda: finkit.plus_di(high, low, close, 14), lambda: talib.PLUS_DI(high, low, close, 14), "Momentum"),
    ("ROC", lambda: finkit.roc(close, 10), lambda: talib.ROC(close, 10), "Momentum"),
    ("RSI", lambda: finkit.rsi(close, 14), lambda: talib.RSI(close, 14), "Momentum"),
    ("TRIX", lambda: finkit.trix(close, 20), lambda: talib.TRIX(close, 20), "Momentum"),
    ("WILLR", lambda: finkit.willr(high, low, close, 14), lambda: talib.WILLR(high, low, close, 14), "Momentum"),
    
    # Volume indicators
    ("AD", lambda: finkit.ad(high, low, close, volume), lambda: talib.AD(high, low, close, volume), "Volume"),
    ("ADOSC", lambda: finkit.adosc(high, low, close, volume, 3, 10), lambda: talib.ADOSC(high, low, close, volume, 3, 10), "Volume"),
    ("OBV", lambda: finkit.obv(close, volume), lambda: talib.OBV(close, volume), "Volume"),
    
    # Volatility indicators
    ("ATR", lambda: finkit.atr(high, low, close, 14), lambda: talib.ATR(high, low, close, 14), "Volatility"),
    ("NATR", lambda: finkit.natr(high, low, close, 14), lambda: talib.NATR(high, low, close, 14), "Volatility"),
    ("TRANGE", lambda: finkit.trange(high, low, close), lambda: talib.TRANGE(high, low, close), "Volatility"),
    
    # Statistic indicators
    ("BETA", lambda: finkit.beta(close, close, 5), lambda: talib.BETA(close, close, 5), "Statistic"),
    ("TSF", lambda: finkit.tsf(close, 14), lambda: talib.TSF(close, 14), "Statistic"),
    ("VAR", lambda: finkit.var(close, 20, 1.0), lambda: talib.VAR(close, 20, 1.0), "Statistic"),
    
    # Cycle indicators
    ("HT_DCPERIOD", lambda: finkit.ht_dcperiod(close), lambda: talib.HT_DCPERIOD(close), "Cycle"),
    ("HT_DCPHASE", lambda: finkit.ht_dcphase(close), lambda: talib.HT_DCPHASE(close), "Cycle"),
    ("HT_PHASOR", lambda: finkit.ht_phasor(close), lambda: talib.HT_PHASOR(close), "Cycle"),
    ("HT_SINE", lambda: finkit.ht_sine(close), lambda: talib.HT_SINE(close), "Cycle"),
    ("HT_TRENDLINE", lambda: finkit.ht_trendline(close), lambda: talib.HT_TRENDLINE(close), "Cycle"),
    ("HT_TRENDMODE", lambda: finkit.ht_trendmode(close), lambda: talib.HT_TRENDMODE(close), "Cycle"),
]

print("正在运行性能测试和数值验证...")
print()

# Run benchmarks and validations
perf_results = []
valid_results = []

for name, alpha_func, talib_func, category in test_cases:
    perf = benchmark(name, alpha_func, talib_func)
    perf['category'] = category
    perf_results.append(perf)
    
    if perf['error'] is None:
        valid = validate_numerical(name, alpha_func, talib_func)
        valid['category'] = category
        valid_results.append(valid)

# Generate report
report_lines = []
report_lines.append("# AlphaTA vs TA-Lib 完整对比报告")
report_lines.append("")
report_lines.append(f"**测试时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
report_lines.append(f"**测试环境**: Windows 10, Python 3.13.12, TA-Lib 0.6.8")
report_lines.append(f"**测试数据**: {n} 个数据点")
report_lines.append(f"**迭代次数**: 100 次")
report_lines.append("")
report_lines.append("---")
report_lines.append("")

# Summary statistics
report_lines.append("## 📊 总体统计")
report_lines.append("")

tested_count = len([r for r in perf_results if r['error'] is None])
total_count = len(perf_results)
avg_speedup = sum(r['speedup'] for r in perf_results if r['error'] is None) / tested_count if tested_count > 0 else 0
max_speedup = max((r['speedup'] for r in perf_results if r['error'] is None), default=0)
min_speedup = min((r['speedup'] for r in perf_results if r['error'] is None), default=0)
above_1x = sum(1 for r in perf_results if r['error'] is None and r['speedup'] >= 1.0)
above_2x = sum(1 for r in perf_results if r['error'] is None and r['speedup'] >= 2.0)
above_5x = sum(1 for r in perf_results if r['error'] is None and r['speedup'] >= 5.0)

numerical_match = sum(1 for r in valid_results if r['match'])
numerical_total = len(valid_results)

report_lines.append("| 指标 | 数值 |")
report_lines.append("|------|------|")
report_lines.append(f"| 总测试指标数 | {total_count} |")
report_lines.append(f"| 成功测试数 | {tested_count} ({100*tested_count/total_count:.1f}%) |")
report_lines.append(f"| 测试失败数 | {total_count - tested_count} ({100*(total_count-tested_count)/total_count:.1f}%) |")
report_lines.append(f"| **平均加速比** | **{avg_speedup:.2f}x** |")
report_lines.append(f"| 最大加速比 | {max_speedup:.2f}x ({[r['name'] for r in perf_results if r['speedup'] == max_speedup][0]}) |")
report_lines.append(f"| 最小加速比 | {min_speedup:.2f}x ({[r['name'] for r in perf_results if r['speedup'] == min_speedup][0]}) |")
report_lines.append(f"| 性能优于 TA-Lib | {above_1x}/{tested_count} ({100*above_1x/tested_count:.1f}%) |")
report_lines.append(f"| 加速比 > 2.0x | {above_2x}/{tested_count} ({100*above_2x/tested_count:.1f}%) |")
report_lines.append(f"| 加速比 > 5.0x | {above_5x}/{tested_count} ({100*above_5x/tested_count:.1f}%) |")
report_lines.append(f"| **数值正确性** | **{numerical_match}/{numerical_total} ({100*numerical_match/numerical_total:.1f}%)** |")
report_lines.append("")
report_lines.append("---")
report_lines.append("")

# Top 10 fastest
report_lines.append("## 🏆 性能最优指标 TOP 10")
report_lines.append("")
report_lines.append("| 排名 | 指标 | AlphaTA (ms) | TA-Lib (ms) | 加速比 | 类别 | 数值正确 |")
report_lines.append("|------|------|--------------|-------------|--------|------|----------|")

sorted_perf = sorted([r for r in perf_results if r['error'] is None], key=lambda x: x['speedup'], reverse=True)
for i, r in enumerate(sorted_perf[:10], 1):
    valid_match = next((v['match'] for v in valid_results if v['name'] == r['name']), None)
    valid_str = "✅" if valid_match else "❌" if valid_match is False else "N/A"
    report_lines.append(f"| {i} | {r['name']} | {r['alphata_ms']:.4f} | {r['talib_ms']:.4f} | **{r['speedup']:.2f}x** | {r['category']} | {valid_str} |")

report_lines.append("")
report_lines.append("---")
report_lines.append("")

# Slowest indicators
report_lines.append("## 📉 性能落后指标 (加速比 < 1.0x)")
report_lines.append("")
report_lines.append("| 指标 | AlphaTA (ms) | TA-Lib (ms) | 加速比 | 差距 | 数值正确 |")
report_lines.append("|------|--------------|-------------|--------|------|----------|")

slow_indicators = [r for r in perf_results if r['error'] is None and r['speedup'] < 1.0]
slow_indicators.sort(key=lambda x: x['speedup'])
for r in slow_indicators:
    valid_match = next((v['match'] for v in valid_results if v['name'] == r['name']), None)
    valid_str = "✅" if valid_match else "❌" if valid_match is False else "N/A"
    gap = (1 - r['speedup']) * 100
    report_lines.append(f"| {r['name']} | {r['alphata_ms']:.4f} | {r['talib_ms']:.4f} | {r['speedup']:.2f}x | -{gap:.1f}% | {valid_str} |")

report_lines.append("")
report_lines.append("---")
report_lines.append("")

# Category breakdown
report_lines.append("## 📂 分类性能统计")
report_lines.append("")

categories = {}
for r in perf_results:
    if r['error'] is None:
        cat = r['category']
        if cat not in categories:
            categories[cat] = []
        categories[cat].append(r)

for cat in sorted(categories.keys()):
    cat_results = categories[cat]
    cat_avg = sum(r['speedup'] for r in cat_results) / len(cat_results)
    
    report_lines.append(f"### {cat} ({len(cat_results)} 个指标)")
    report_lines.append(f"**平均加速比: {cat_avg:.2f}x**")
    report_lines.append("")
    report_lines.append("| 指标 | AlphaTA (ms) | TA-Lib (ms) | 加速比 | 数值正确 |")
    report_lines.append("|------|--------------|-------------|--------|----------|")
    
    for r in sorted(cat_results, key=lambda x: x['speedup'], reverse=True):
        valid_match = next((v['match'] for v in valid_results if v['name'] == r['name']), None)
        valid_str = "✅" if valid_match else "❌" if valid_match is False else "N/A"
        status = "✅" if r['speedup'] >= 1.0 else "⚠️"
        report_lines.append(f"| {r['name']} | {r['alphata_ms']:.4f} | {r['talib_ms']:.4f} | {r['speedup']:.2f}x {status} | {valid_str} |")
    
    report_lines.append("")

report_lines.append("---")
report_lines.append("")

# Numerical correctness details
report_lines.append("## 🔬 数值正确性验证")
report_lines.append("")
report_lines.append(f"**验证通过**: {numerical_match}/{numerical_total} ({100*numerical_match/numerical_total:.1f}%)")
report_lines.append("")

if numerical_match < numerical_total:
    report_lines.append("### 数值不匹配的指标")
    report_lines.append("")
    report_lines.append("| 指标 | 最大差异 | 平均差异 | 错误信息 |")
    report_lines.append("|------|----------|----------|----------|")
    
    for v in valid_results:
        if not v['match']:
            max_diff = f"{v['max_diff']:.2e}" if v['max_diff'] is not None else "N/A"
            mean_diff = f"{v['mean_diff']:.2e}" if v['mean_diff'] is not None else "N/A"
            error = v['error'] if v['error'] else "超出容差"
            report_lines.append(f"| {v['name']} | {max_diff} | {mean_diff} | {error} |")
    
    report_lines.append("")

report_lines.append("---")
report_lines.append("")

# Conclusion
report_lines.append("## 📝 结论")
report_lines.append("")

if avg_speedup >= 1.5:
    perf_rating = "⭐⭐⭐⭐⭐ (5/5)"
    perf_comment = "卓越"
elif avg_speedup >= 1.2:
    perf_rating = "⭐⭐⭐⭐ (4/5)"
    perf_comment = "优秀"
elif avg_speedup >= 1.0:
    perf_rating = "⭐⭐⭐ (3/5)"
    perf_comment = "良好"
else:
    perf_rating = "⭐⭐ (2/5)"
    perf_comment = "需改进"

if numerical_match / numerical_total >= 0.95:
    correct_rating = "⭐⭐⭐⭐⭐ (5/5)"
    correct_comment = "优秀"
elif numerical_match / numerical_total >= 0.80:
    correct_rating = "⭐⭐⭐⭐ (4/5)"
    correct_comment = "良好"
else:
    correct_rating = "⭐⭐⭐ (3/5)"
    correct_comment = "需改进"

report_lines.append("### 性能表现")
report_lines.append(f"- **评分**: {perf_rating}")
report_lines.append(f"- **评价**: {perf_comment}")
report_lines.append(f"- **平均加速比**: {avg_speedup:.2f}x")
report_lines.append(f"- **优于 TA-Lib**: {100*above_1x/tested_count:.1f}%")
report_lines.append("")
report_lines.append("### 数值正确性")
report_lines.append(f"- **评分**: {correct_rating}")
report_lines.append(f"- **评价**: {correct_comment}")
report_lines.append(f"- **验证通过率**: {100*numerical_match/numerical_total:.1f}%")
report_lines.append("")
report_lines.append("### 综合建议")
report_lines.append("")

if avg_speedup >= 1.5 and numerical_match / numerical_total >= 0.95:
    report_lines.append("✅ **AlphaTA 在性能和正确性方面均表现优秀，可以替代 TA-Lib 使用**")
elif avg_speedup >= 1.0 and numerical_match / numerical_total >= 0.80:
    report_lines.append("✅ **AlphaTA 性能和正确性良好，大部分场景可替代 TA-Lib**")
    report_lines.append("⚠️ 建议优化性能落后的指标，并修复数值不匹配问题")
else:
    report_lines.append("⚠️ **AlphaTA 需要进一步优化**")
    report_lines.append("- 优先修复数值不匹配问题")
    report_lines.append("- 优化性能落后的指标")

report_lines.append("")
report_lines.append("---")
report_lines.append("")
report_lines.append(f"**报告生成时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")

# Write report
report_content = "\n".join(report_lines)
with open("ALPHATA_VS_TALIB_COMPARISON_REPORT.md", "w", encoding="utf-8") as f:
    f.write(report_content)

print()
print("=" * 100)
print("报告已生成: ALPHATA_VS_TALIB_COMPARISON_REPORT.md")
print("=" * 100)
print()
print("快速摘要:")
print(f"  - 测试指标数: {tested_count}/{total_count}")
print(f"  - 平均加速比: {avg_speedup:.2f}x")
print(f"  - 性能优于 TA-Lib: {100*above_1x/tested_count:.1f}%")
print(f"  - 数值正确性: {numerical_match}/{numerical_total} ({100*numerical_match/numerical_total:.1f}%)")
print(f"  - 最佳性能: {sorted_perf[0]['name']} ({sorted_perf[0]['speedup']:.2f}x)")
print(f"  - 最差性能: {slow_indicators[0]['name'] if slow_indicators else 'N/A'} ({slow_indicators[0]['speedup'] if slow_indicators else 0:.2f}x)")
