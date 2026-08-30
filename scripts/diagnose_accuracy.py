"""
诊断脚本：详细对比 AlphaTA 和 TA-Lib 的数值差异
找出每个指标的具体偏差位置和模式
"""
import numpy as np
import finkit
import talib

# 生成测试数据
np.random.seed(42)
n = 100
close = np.random.uniform(95, 105, n).astype(np.float64)
high = np.random.uniform(100, 110, n).astype(np.float64)
low = np.random.uniform(90, 100, n).astype(np.float64)
open_price = np.random.uniform(100, 110, n).astype(np.float64)
volume = np.random.uniform(1000000, 2000000, n).astype(np.float64)

high = np.maximum(high, np.maximum(open_price, close) + 1)
low = np.minimum(low, np.minimum(open_price, close) - 1)

def compare_indicator(name, alpha_func, talib_func, tolerance=1e-6):
    """详细对比单个指标"""
    print(f"\n{'='*80}")
    print(f"指标: {name}")
    print(f"{'='*80}")
    
    try:
        alpha_result = alpha_func()
        talib_result = talib_func()
        
        # 处理多输出
        if isinstance(alpha_result, tuple):
            alpha_result = alpha_result[0]
        if isinstance(talib_result, tuple):
            talib_result = talib_result[0]
        
        alpha_arr = np.asarray(alpha_result)
        talib_arr = np.asarray(talib_result)
        
        print(f"AlphaTA shape: {alpha_arr.shape}, TA-Lib shape: {talib_arr.shape}")
        
        if alpha_arr.shape != talib_arr.shape:
            print(f"❌ 形状不匹配！")
            return False
        
        # 找到有效值
        valid_mask = ~(np.isnan(alpha_arr) | np.isnan(talib_arr))
        valid_count = np.sum(valid_mask)
        
        print(f"有效值数量: {valid_count}/{len(alpha_arr)}")
        
        if valid_count == 0:
            print("⚠️ 没有有效值可比较")
            return True
        
        # 计算差异
        alpha_valid = alpha_arr[valid_mask]
        talib_valid = talib_arr[valid_mask]
        
        diff = np.abs(alpha_valid - talib_valid)
        max_diff = np.max(diff)
        mean_diff = np.mean(diff)
        median_diff = np.median(diff)
        
        print(f"\n差异统计:")
        print(f"  最大差异: {max_diff:.6e}")
        print(f"  平均差异: {mean_diff:.6e}")
        print(f"  中位差异: {median_diff:.6e}")
        
        # 找到差异最大的位置
        max_diff_idx = np.argmax(diff)
        original_idx = np.where(valid_mask)[0][max_diff_idx]
        
        print(f"\n最大差异位置: index={original_idx}")
        print(f"  AlphaTA: {alpha_arr[original_idx]:.10f}")
        print(f"  TA-Lib:  {talib_arr[original_idx]:.10f}")
        print(f"  差异:    {diff[max_diff_idx]:.10e}")
        
        # 显示前10个有效值的对比
        print(f"\n前10个有效值对比:")
        valid_indices = np.where(valid_mask)[0][:10]
        for idx in valid_indices:
            a_val = alpha_arr[idx]
            t_val = talib_arr[idx]
            d = abs(a_val - t_val)
            status = "✅" if d < tolerance else "❌"
            print(f"  [{idx:3d}] AlphaTA={a_val:12.6f}  TA-Lib={t_val:12.6f}  diff={d:.2e} {status}")
        
        # 判断是否通过
        passed = max_diff < tolerance
        print(f"\n{'✅ 通过' if passed else '❌ 失败'} (容差: {tolerance:.1e})")
        
        return passed
        
    except Exception as e:
        print(f"❌ 错误: {e}")
        return False

# 测试有问题的指标
print("开始诊断数值准确度问题...")

# HT_* 系列
compare_indicator("HT_DCPERIOD", 
    lambda: finkit.ht_dcperiod(close),
    lambda: talib.HT_DCPERIOD(close))

compare_indicator("HT_DCPHASE",
    lambda: finkit.ht_dcphase(close),
    lambda: talib.HT_DCPHASE(close))

compare_indicator("HT_PHASOR",
    lambda: finkit.ht_phasor(close),
    lambda: talib.HT_PHASOR(close))

compare_indicator("HT_SINE",
    lambda: finkit.ht_sine(close),
    lambda: talib.HT_SINE(close))

compare_indicator("HT_TRENDLINE",
    lambda: finkit.ht_trendline(close),
    lambda: talib.HT_TRENDLINE(close))

compare_indicator("HT_TRENDMODE",
    lambda: finkit.ht_trendmode(close),
    lambda: talib.HT_TRENDMODE(close))

# MAMA 和 T3
compare_indicator("MAMA",
    lambda: finkit.mama(close, 0.5, 0.05),
    lambda: talib.MAMA(close, 0.5, 0.05))

compare_indicator("T3",
    lambda: finkit.t3(close, 20, 0.7),
    lambda: talib.T3(close, 20, 0.7))

# ADX 系列
compare_indicator("ADX",
    lambda: finkit.adx(high, low, close, 14),
    lambda: talib.ADX(high, low, close, 14))

compare_indicator("DX",
    lambda: finkit.dx(high, low, close, 14),
    lambda: talib.DX(high, low, close, 14))

compare_indicator("PLUS_DI",
    lambda: finkit.plus_di(high, low, close, 14),
    lambda: talib.PLUS_DI(high, low, close, 14))

compare_indicator("MINUS_DI",
    lambda: finkit.minus_di(high, low, close, 14),
    lambda: talib.MINUS_DI(high, low, close, 14))

# MACD
compare_indicator("MACD",
    lambda: finkit.macd(close, 12, 26, 9),
    lambda: talib.MACD(close, 12, 26, 9))

# TRIX
compare_indicator("TRIX",
    lambda: finkit.trix(close, 20),
    lambda: talib.TRIX(close, 20))

# AD 和 ADOSC
compare_indicator("AD",
    lambda: finkit.ad(high, low, close, volume),
    lambda: talib.AD(high, low, close, volume))

compare_indicator("ADOSC",
    lambda: finkit.adosc(high, low, close, volume, 3, 10),
    lambda: talib.ADOSC(high, low, close, volume, 3, 10))

# ATR
compare_indicator("ATR",
    lambda: finkit.atr(high, low, close, 14),
    lambda: talib.ATR(high, low, close, 14))

# VAR
compare_indicator("VAR",
    lambda: finkit.var(close, 20, 1.0),
    lambda: talib.VAR(close, 20, 1.0))

print("\n" + "="*80)
print("诊断完成")
print("="*80)
