"""
全面数值准确度诊断脚本
对比 Finkit 和 TA-Lib 的所有指标
"""
import numpy as np
import finkit
import talib
import json

alpha_ta = finkit  # backward-compat alias (module renamed to `finkit`)

# 生成测试数据
np.random.seed(42)
n = 200
close = np.random.uniform(95, 105, n).astype(np.float64)
high = np.random.uniform(100, 110, n).astype(np.float64)
low = np.random.uniform(90, 100, n).astype(np.float64)
open_price = np.random.uniform(100, 110, n).astype(np.float64)
volume = np.random.uniform(1000000, 2000000, n).astype(np.float64)

high = np.maximum(high, np.maximum(open_price, close) + 1)
low = np.minimum(low, np.minimum(open_price, close) - 1)

def compare_indicator(name, alpha_func, talib_func, tolerance=1e-6):
    """对比单个指标"""
    try:
        alpha_result = alpha_func()
        talib_result = talib_func()
        
        # 处理多输出
        if isinstance(alpha_result, tuple):
            results = []
            for i, (a, t) in enumerate(zip(alpha_result, talib_result)):
                a_arr = np.asarray(a)
                t_arr = np.asarray(t)
                
                valid_mask = ~(np.isnan(a_arr) | np.isnan(t_arr))
                valid_count = np.sum(valid_mask)
                
                if valid_count == 0:
                    continue
                
                a_valid = a_arr[valid_mask]
                t_valid = t_arr[valid_mask]
                
                diff = np.abs(a_valid - t_valid)
                max_diff = np.max(diff)
                mean_diff = np.mean(diff)
                
                passed = max_diff < tolerance
                results.append({
                    'index': i,
                    'max_diff': float(max_diff),
                    'mean_diff': float(mean_diff),
                    'passed': passed
                })
            
            if not results:
                return {'name': name, 'status': 'no_data', 'message': '无有效值'}
            
            all_passed = all(r['passed'] for r in results)
            max_diff_overall = max(r['max_diff'] for r in results)
            
            return {
                'name': name,
                'status': 'pass' if all_passed else 'fail',
                'max_diff': max_diff_overall,
                'details': results
            }
        else:
            alpha_arr = np.asarray(alpha_result)
            talib_arr = np.asarray(talib_result)
            
            if alpha_arr.shape != talib_arr.shape:
                return {'name': name, 'status': 'shape_mismatch', 
                        'message': f'形状不匹配: {alpha_arr.shape} vs {talib_arr.shape}'}
            
            valid_mask = ~(np.isnan(alpha_arr) | np.isnan(talib_arr))
            valid_count = np.sum(valid_mask)
            
            if valid_count == 0:
                return {'name': name, 'status': 'no_data', 'message': '无有效值'}
            
            alpha_valid = alpha_arr[valid_mask]
            talib_valid = talib_arr[valid_mask]
            
            diff = np.abs(alpha_valid - talib_valid)
            max_diff = np.max(diff)
            mean_diff = np.mean(diff)
            
            passed = max_diff < tolerance
            
            return {
                'name': name,
                'status': 'pass' if passed else 'fail',
                'max_diff': float(max_diff),
                'mean_diff': float(mean_diff),
                'valid_count': int(valid_count)
            }
            
    except Exception as e:
        return {'name': name, 'status': 'error', 'message': str(e)}

# 测试所有指标
print("开始全面数值准确度诊断...")
print("="*80)

indicators_to_test = [
    # HT_* 系列
    ("HT_DCPERIOD", lambda: alpha_ta.ht_dcperiod(close), lambda: talib.HT_DCPERIOD(close)),
    ("HT_DCPHASE", lambda: alpha_ta.ht_dcphase(close), lambda: talib.HT_DCPHASE(close)),
    ("HT_PHASOR", lambda: alpha_ta.ht_phasor(close), lambda: talib.HT_PHASOR(close)),
    ("HT_SINE", lambda: alpha_ta.ht_sine(close), lambda: talib.HT_SINE(close)),
    ("HT_TRENDLINE", lambda: alpha_ta.ht_trendline(close), lambda: talib.HT_TRENDLINE(close)),
    ("HT_TRENDMODE", lambda: alpha_ta.ht_trendmode(close), lambda: talib.HT_TRENDMODE(close)),
    
    # MAMA 和 T3
    ("MAMA", lambda: alpha_ta.mama(close, 0.5, 0.05), lambda: talib.MAMA(close, 0.5, 0.05)),
    ("T3", lambda: alpha_ta.t3(close, 20, 0.7), lambda: talib.T3(close, 20, 0.7)),
    
    # ADX 系列
    ("ADX", lambda: alpha_ta.adx(high, low, close, 14), lambda: talib.ADX(high, low, close, 14)),
    ("DX", lambda: alpha_ta.dx(high, low, close, 14), lambda: talib.DX(high, low, close, 14)),
    ("PLUS_DI", lambda: alpha_ta.plus_di(high, low, close, 14), lambda: talib.PLUS_DI(high, low, close, 14)),
    ("MINUS_DI", lambda: alpha_ta.minus_di(high, low, close, 14), lambda: talib.MINUS_DI(high, low, close, 14)),
    
    # MACD
    ("MACD", lambda: alpha_ta.macd(close, 12, 26, 9), lambda: talib.MACD(close, 12, 26, 9)),
    
    # TRIX
    ("TRIX", lambda: alpha_ta.trix(close, 20), lambda: talib.TRIX(close, 20)),
    
    # AD 和 ADOSC
    ("AD", lambda: alpha_ta.ad(high, low, close, volume), lambda: talib.AD(high, low, close, volume)),
    ("ADOSC", lambda: alpha_ta.adosc(high, low, close, volume, 3, 10), lambda: talib.ADOSC(high, low, close, volume, 3, 10)),
    
    # ATR
    ("ATR", lambda: alpha_ta.atr(high, low, close, 14), lambda: talib.ATR(high, low, close, 14)),
    
    # VAR
    ("VAR", lambda: alpha_ta.var(close, 20, 1.0), lambda: talib.VAR(close, 20, 1.0)),
    
    # 其他常用指标
    ("RSI", lambda: alpha_ta.rsi(close, 14), lambda: talib.RSI(close, 14)),
    ("SMA", lambda: alpha_ta.sma(close, 20), lambda: talib.SMA(close, 20)),
    ("EMA", lambda: alpha_ta.ema(close, 20), lambda: talib.EMA(close, 20)),
    ("WMA", lambda: alpha_ta.wma(close, 20), lambda: talib.WMA(close, 20)),
    ("BBANDS", lambda: alpha_ta.bbands(close, 20, 2.0, 2.0), lambda: talib.BBANDS(close, 20, 2.0, 2.0)),
    ("STOCH", lambda: alpha_ta.stoch(high, low, close, 14, 3, 3), lambda: talib.STOCH(high, low, close, 14, 3, 3, 3, 3)),
    ("CCI", lambda: alpha_ta.cci(high, low, close, 20), lambda: talib.CCI(high, low, close, 20)),
    ("WILLR", lambda: alpha_ta.willr(high, low, close, 14), lambda: talib.WILLR(high, low, close, 14)),
    ("AROON", lambda: alpha_ta.aroon(high, low, 14), lambda: talib.AROON(high, low, 14)),
    ("APO", lambda: alpha_ta.apo(close, 12, 26), lambda: talib.APO(close, 12, 26)),
    ("KAMA", lambda: alpha_ta.kama(close, 30), lambda: talib.KAMA(close, 30)),
]

results = []
for name, alpha_func, talib_func in indicators_to_test:
    result = compare_indicator(name, alpha_func, talib_func)
    results.append(result)
    
    status_icon = "✅" if result['status'] == 'pass' else "❌" if result['status'] == 'fail' else "⚠️"
    print(f"{status_icon} {name:20s} - {result['status']:15s}", end="")
    
    if result['status'] == 'pass':
        print(f" (max_diff: {result.get('max_diff', 'N/A'):.2e})")
    elif result['status'] == 'fail':
        print(f" (max_diff: {result.get('max_diff', 'N/A'):.2e})")
    else:
        print(f" ({result.get('message', 'unknown')})")

print("="*80)

# 统计
passed_count = sum(1 for r in results if r['status'] == 'pass')
failed_count = sum(1 for r in results if r['status'] == 'fail')
error_count = sum(1 for r in results if r['status'] not in ['pass', 'fail'])

print(f"\n总计: {len(results)} 个指标")
print(f"通过: {passed_count}")
print(f"失败: {failed_count}")
print(f"错误/无数据: {error_count}")

# 保存详细结果
with open('accuracy_diagnose_results.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

print("\n详细结果已保存到 accuracy_diagnose_results.json")
