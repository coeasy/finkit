#!/usr/bin/env python3
"""
Comprehensive comparison test between AlphaTA and TA-Lib
Tests all indicators and generates detailed accuracy report
"""

import numpy as np
import talib
import alpha_ta
import time
import json
from typing import Dict, List, Tuple

def generate_test_data(size: int = 1000) -> Dict[str, np.ndarray]:
    """Generate realistic OHLCV test data"""
    np.random.seed(42)
    
    # Generate realistic price data with trend and volatility
    base_price = 100.0
    returns = np.random.normal(0.0005, 0.02, size)
    close = base_price * np.exp(np.cumsum(returns))
    
    # Generate OHLC from close
    high = close * (1 + np.abs(np.random.normal(0, 0.01, size)))
    low = close * (1 - np.abs(np.random.normal(0, 0.01, size)))
    open_price = np.roll(close, 1)
    open_price[0] = close[0] * 0.99
    
    # Ensure OHLC consistency
    high = np.maximum(high, np.maximum(open_price, close))
    low = np.minimum(low, np.minimum(open_price, close))
    
    # Generate volume
    volume = np.random.uniform(1e6, 1e7, size)
    
    return {
        'open': open_price,
        'high': high,
        'low': low,
        'close': close,
        'volume': volume
    }

def compare_arrays(alpha_arr: np.ndarray, talib_arr: np.ndarray, 
                   tolerance: float = 1e-6) -> Tuple[bool, float, float]:
    """
    Compare two arrays and return (match, max_diff, mean_diff)
    Handles NaN values properly
    """
    # Find valid indices (both not NaN)
    valid_mask = ~(np.isnan(alpha_arr) | np.isnan(talib_arr))
    
    if not np.any(valid_mask):
        return True, 0.0, 0.0
    
    alpha_valid = alpha_arr[valid_mask]
    talib_valid = talib_arr[valid_mask]
    
    # Calculate differences
    diff = np.abs(alpha_valid - talib_valid)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    match = max_diff < tolerance
    
    return match, max_diff, mean_diff

def test_indicator(name: str, alpha_func, talib_func, data: Dict, 
                   tolerance: float = 1e-6) -> Dict:
    """Test a single indicator and return results"""
    try:
        # Run AlphaTA
        start_time = time.time()
        alpha_result = alpha_func(data)
        alpha_time = time.time() - start_time
        
        # Run TA-Lib
        start_time = time.time()
        talib_result = talib_func(data)
        talib_time = time.time() - start_time
        
        # Handle multiple outputs
        if isinstance(alpha_result, tuple):
            results = []
            for i, (a_res, t_res) in enumerate(zip(alpha_result, talib_result)):
                match, max_diff, mean_diff = compare_arrays(a_res, t_res, tolerance)
                results.append({
                    'output_idx': i,
                    'match': match,
                    'max_diff': max_diff,
                    'mean_diff': mean_diff
                })
            
            all_match = all(r['match'] for r in results)
            max_diff = max(r['max_diff'] for r in results)
            mean_diff = max(r['mean_diff'] for r in results)
            
        else:
            match, max_diff, mean_diff = compare_arrays(alpha_result, talib_result, tolerance)
            results = [{'output_idx': 0, 'match': match, 'max_diff': max_diff, 'mean_diff': mean_diff}]
            all_match = match
        
        speedup = talib_time / alpha_time if alpha_time > 0 else 1.0
        
        return {
            'name': name,
            'status': 'PASS' if all_match else 'FAIL',
            'max_diff': max_diff,
            'mean_diff': mean_diff,
            'alpha_time': alpha_time,
            'talib_time': talib_time,
            'speedup': speedup,
            'details': results
        }
        
    except Exception as e:
        return {
            'name': name,
            'status': 'ERROR',
            'error': str(e)
        }

def main():
    print("=" * 80)
    print("AlphaTA vs TA-Lib Comprehensive Comparison Test")
    print("=" * 80)
    
    # Generate test data
    print("\nGenerating test data (1000 bars)...")
    data = generate_test_data(1000)
    
    # Define all indicators to test
    indicators = [
        # Overlap studies
        ('BBANDS', 
         lambda d: alpha_ta.bbands(d['close'], timeperiod=20, nbdevup=2.0, nbdevdn=2.0),
         lambda d: talib.BBANDS(d['close'], timeperiod=20, nbdevup=2.0, nbdevdn=2.0)),
        
        ('DEMA',
         lambda d: alpha_ta.dema(d['close'], timeperiod=30),
         lambda d: talib.DEMA(d['close'], timeperiod=30)),
        
        ('EMA',
         lambda d: alpha_ta.ema(d['close'], timeperiod=30),
         lambda d: talib.EMA(d['close'], timeperiod=30)),
        
        ('HT_TRENDLINE',
         lambda d: alpha_ta.ht_trendline(d['close']),
         lambda d: talib.HT_TRENDLINE(d['close'])),
        
        ('KAMA',
         lambda d: alpha_ta.kama(d['close'], timeperiod=30),
         lambda d: talib.KAMA(d['close'], timeperiod=30)),
        
        ('MA',
         lambda d: alpha_ta.ma(d['close'], timeperiod=30, matype=0),
         lambda d: talib.MA(d['close'], timeperiod=30, matype=0)),
        
        ('MAMA',
         lambda d: alpha_ta.mama(d['close'], fastlimit=0.5, slowlimit=0.05),
         lambda d: talib.MAMA(d['close'], fastlimit=0.5, slowlimit=0.05)),
        
        ('MIDPOINT',
         lambda d: alpha_ta.midpoint(d['close'], timeperiod=14),
         lambda d: talib.MIDPOINT(d['close'], timeperiod=14)),
        
        ('MIDPRICE',
         lambda d: alpha_ta.midprice(d['high'], d['low'], timeperiod=14),
         lambda d: talib.MIDPRICE(d['high'], d['low'], timeperiod=14)),
        
        ('SAR',
         lambda d: alpha_ta.sar(d['high'], d['low'], acceleration=0.02, maximum=0.2),
         lambda d: talib.SAR(d['high'], d['low'], acceleration=0.02, maximum=0.2)),
        
        ('SMA',
         lambda d: alpha_ta.sma(d['close'], timeperiod=30),
         lambda d: talib.SMA(d['close'], timeperiod=30)),
        
        ('T3',
         lambda d: alpha_ta.t3(d['close'], timeperiod=5, vfactor=0.7),
         lambda d: talib.T3(d['close'], timeperiod=5, vfactor=0.7)),
        
        ('TEMA',
         lambda d: alpha_ta.tema(d['close'], timeperiod=30),
         lambda d: talib.TEMA(d['close'], timeperiod=30)),
        
        ('TRIMA',
         lambda d: alpha_ta.trima(d['close'], timeperiod=30),
         lambda d: talib.TRIMA(d['close'], timeperiod=30)),
        
        ('WMA',
         lambda d: alpha_ta.wma(d['close'], timeperiod=30),
         lambda d: talib.WMA(d['close'], timeperiod=30)),
        
        # Momentum indicators
        ('ADX',
         lambda d: alpha_ta.adx(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.ADX(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('ADXR',
         lambda d: alpha_ta.adxr(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.ADXR(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('APO',
         lambda d: alpha_ta.apo(d['close'], fastperiod=12, slowperiod=26, matype=0),
         lambda d: talib.APO(d['close'], fastperiod=12, slowperiod=26, matype=0)),
        
        ('AROON',
         lambda d: alpha_ta.aroon(d['high'], d['low'], timeperiod=14),
         lambda d: talib.AROON(d['high'], d['low'], timeperiod=14)),
        
        ('AROONOSC',
         lambda d: alpha_ta.aroonosc(d['high'], d['low'], timeperiod=14),
         lambda d: talib.AROONOSC(d['high'], d['low'], timeperiod=14)),
        
        ('BOP',
         lambda d: alpha_ta.bop(d['open'], d['high'], d['low'], d['close']),
         lambda d: talib.BOP(d['open'], d['high'], d['low'], d['close'])),
        
        ('CCI',
         lambda d: alpha_ta.cci(d['high'], d['low'], d['close'], timeperiod=20),
         lambda d: talib.CCI(d['high'], d['low'], d['close'], timeperiod=20)),
        
        ('CMO',
         lambda d: alpha_ta.cmo(d['close'], timeperiod=14),
         lambda d: talib.CMO(d['close'], timeperiod=14)),
        
        ('DX',
         lambda d: alpha_ta.dx(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.DX(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('MACD',
         lambda d: alpha_ta.macd(d['close'], fastperiod=12, slowperiod=26, signalperiod=9),
         lambda d: talib.MACD(d['close'], fastperiod=12, slowperiod=26, signalperiod=9)),
        
        ('MACDEXT',
         lambda d: alpha_ta.macdext(d['close'], fastperiod=12, fastmatype=0, slowperiod=26, slowmatype=0, signalperiod=9, signalmatype=0),
         lambda d: talib.MACDEXT(d['close'], fastperiod=12, fastmatype=0, slowperiod=26, slowmatype=0, signalperiod=9, signalmatype=0)),
        
        ('MACDFIX',
         lambda d: alpha_ta.macdfix(d['close'], signalperiod=9),
         lambda d: talib.MACDFIX(d['close'], signalperiod=9)),
        
        ('MFI',
         lambda d: alpha_ta.mfi(d['high'], d['low'], d['close'], d['volume'], timeperiod=14),
         lambda d: talib.MFI(d['high'], d['low'], d['close'], d['volume'], timeperiod=14)),
        
        ('MINUS_DI',
         lambda d: alpha_ta.minus_di(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.MINUS_DI(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('MINUS_DM',
         lambda d: alpha_ta.minus_dm(d['high'], d['low'], timeperiod=14),
         lambda d: talib.MINUS_DM(d['high'], d['low'], timeperiod=14)),
        
        ('MOM',
         lambda d: alpha_ta.mom(d['close'], timeperiod=10),
         lambda d: talib.MOM(d['close'], timeperiod=10)),
        
        ('PLUS_DI',
         lambda d: alpha_ta.plus_di(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.PLUS_DI(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('PLUS_DM',
         lambda d: alpha_ta.plus_dm(d['high'], d['low'], timeperiod=14),
         lambda d: talib.PLUS_DM(d['high'], d['low'], timeperiod=14)),
        
        ('PPO',
         lambda d: alpha_ta.ppo(d['close'], fastperiod=12, slowperiod=26, matype=0),
         lambda d: talib.PPO(d['close'], fastperiod=12, slowperiod=26, matype=0)),
        
        ('ROC',
         lambda d: alpha_ta.roc(d['close'], timeperiod=10),
         lambda d: talib.ROC(d['close'], timeperiod=10)),
        
        ('ROCP',
         lambda d: alpha_ta.rocp(d['close'], timeperiod=10),
         lambda d: talib.ROCP(d['close'], timeperiod=10)),
        
        ('ROCR',
         lambda d: alpha_ta.rocr(d['close'], timeperiod=10),
         lambda d: talib.ROCR(d['close'], timeperiod=10)),
        
        ('ROCR100',
         lambda d: alpha_ta.rocr100(d['close'], timeperiod=10),
         lambda d: talib.ROCR100(d['close'], timeperiod=10)),
        
        ('RSI',
         lambda d: alpha_ta.rsi(d['close'], timeperiod=14),
         lambda d: talib.RSI(d['close'], timeperiod=14)),
        
        ('STOCH',
         lambda d: alpha_ta.stoch(d['high'], d['low'], d['close'], fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0),
         lambda d: talib.STOCH(d['high'], d['low'], d['close'], fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0)),
        
        ('STOCHF',
         lambda d: alpha_ta.stochf(d['high'], d['low'], d['close'], fastk_period=5, fastd_period=3, fastd_matype=0),
         lambda d: talib.STOCHF(d['high'], d['low'], d['close'], fastk_period=5, fastd_period=3, fastd_matype=0)),
        
        ('STOCHRSI',
         lambda d: alpha_ta.stochrsi(d['close'], timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0),
         lambda d: talib.STOCHRSI(d['close'], timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0)),
        
        ('TRIX',
         lambda d: alpha_ta.trix(d['close'], timeperiod=30),
         lambda d: talib.TRIX(d['close'], timeperiod=30)),
        
        ('ULTOSC',
         lambda d: alpha_ta.ultosc(d['high'], d['low'], d['close'], timeperiod1=7, timeperiod2=14, timeperiod3=28),
         lambda d: talib.ULTOSC(d['high'], d['low'], d['close'], timeperiod1=7, timeperiod2=14, timeperiod3=28)),
        
        ('WILLR',
         lambda d: alpha_ta.willr(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.WILLR(d['high'], d['low'], d['close'], timeperiod=14)),
        
        # Volume indicators
        ('AD',
         lambda d: alpha_ta.ad(d['high'], d['low'], d['close'], d['volume']),
         lambda d: talib.AD(d['high'], d['low'], d['close'], d['volume'])),
        
        ('ADOSC',
         lambda d: alpha_ta.adosc(d['high'], d['low'], d['close'], d['volume'], fastperiod=3, slowperiod=10),
         lambda d: talib.ADOSC(d['high'], d['low'], d['close'], d['volume'], fastperiod=3, slowperiod=10)),
        
        ('OBV',
         lambda d: alpha_ta.obv(d['close'], d['volume']),
         lambda d: talib.OBV(d['close'], d['volume'])),
        
        # Volatility indicators
        ('ATR',
         lambda d: alpha_ta.atr(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.ATR(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('NATR',
         lambda d: alpha_ta.natr(d['high'], d['low'], d['close'], timeperiod=14),
         lambda d: talib.NATR(d['high'], d['low'], d['close'], timeperiod=14)),
        
        ('TRANGE',
         lambda d: alpha_ta.trange(d['high'], d['low'], d['close']),
         lambda d: talib.TRANGE(d['high'], d['low'], d['close'])),
        
        # Cycle indicators
        ('HT_DCPERIOD',
         lambda d: alpha_ta.ht_dcperiod(d['close']),
         lambda d: talib.HT_DCPERIOD(d['close'])),
        
        ('HT_DCPHASE',
         lambda d: alpha_ta.ht_dcphase(d['close']),
         lambda d: talib.HT_DCPHASE(d['close'])),
        
        ('HT_PHASOR',
         lambda d: alpha_ta.ht_phasor(d['close']),
         lambda d: talib.HT_PHASOR(d['close'])),
        
        ('HT_SINE',
         lambda d: alpha_ta.ht_sine(d['close']),
         lambda d: talib.HT_SINE(d['close'])),
        
        ('HT_TRENDMODE',
         lambda d: alpha_ta.ht_trendmode(d['close']),
         lambda d: talib.HT_TRENDMODE(d['close'])),
        
        # Statistics
        ('BETA',
         lambda d: alpha_ta.beta(d['high'], d['low'], timeperiod=5),
         lambda d: talib.BETA(d['high'], d['low'], timeperiod=5)),
        
        ('CORREL',
         lambda d: alpha_ta.correl(d['high'], d['low'], timeperiod=30),
         lambda d: talib.CORREL(d['high'], d['low'], timeperiod=30)),
        
        ('LINEARREG',
         lambda d: alpha_ta.linearreg(d['close'], timeperiod=14),
         lambda d: talib.LINEARREG(d['close'], timeperiod=14)),
        
        ('LINEARREG_ANGLE',
         lambda d: alpha_ta.linearreg_angle(d['close'], timeperiod=14),
         lambda d: talib.LINEARREG_ANGLE(d['close'], timeperiod=14)),
        
        ('LINEARREG_INTERCEPT',
         lambda d: alpha_ta.linearreg_intercept(d['close'], timeperiod=14),
         lambda d: talib.LINEARREG_INTERCEPT(d['close'], timeperiod=14)),
        
        ('LINEARREG_SLOPE',
         lambda d: alpha_ta.linearreg_slope(d['close'], timeperiod=14),
         lambda d: talib.LINEARREG_SLOPE(d['close'], timeperiod=14)),
        
        ('STDDEV',
         lambda d: alpha_ta.stddev(d['close'], timeperiod=5, nbdev=1.0),
         lambda d: talib.STDDEV(d['close'], timeperiod=5, nbdev=1.0)),
        
        ('TSF',
         lambda d: alpha_ta.tsf(d['close'], timeperiod=14),
         lambda d: talib.TSF(d['close'], timeperiod=14)),
        
        ('VAR',
         lambda d: alpha_ta.var(d['close'], timeperiod=5, nbdev=1.0),
         lambda d: talib.VAR(d['close'], timeperiod=5, nbdev=1.0)),
    ]
    
    # Run tests
    print(f"\nTesting {len(indicators)} indicators...")
    print("-" * 80)
    
    results = []
    pass_count = 0
    fail_count = 0
    error_count = 0
    
    for name, alpha_func, talib_func in indicators:
        result = test_indicator(name, alpha_func, talib_func, data)
        results.append(result)
        
        status_symbol = "✓" if result['status'] == 'PASS' else "✗" if result['status'] == 'FAIL' else "!"
        status_color = "\033[92m" if result['status'] == 'PASS' else "\033[91m" if result['status'] == 'FAIL' else "\033[93m"
        reset_color = "\033[0m"
        
        if result['status'] == 'PASS':
            pass_count += 1
            print(f"{status_color}{status_symbol} {name:20s} PASS  (max_diff: {result['max_diff']:.2e}, speedup: {result['speedup']:.2f}x){reset_color}")
        elif result['status'] == 'FAIL':
            fail_count += 1
            print(f"{status_color}{status_symbol} {name:20s} FAIL  (max_diff: {result['max_diff']:.2e}){reset_color}")
        else:
            error_count += 1
            print(f"{status_color}{status_symbol} {name:20s} ERROR ({result.get('error', 'Unknown')}){reset_color}")
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Total indicators: {len(indicators)}")
    print(f"Passed: {pass_count} ({pass_count/len(indicators)*100:.1f}%)")
    print(f"Failed: {fail_count} ({fail_count/len(indicators)*100:.1f}%)")
    print(f"Errors: {error_count} ({error_count/len(indicators)*100:.1f}%)")
    
    # Performance summary
    speedups = [r['speedup'] for r in results if r['status'] == 'PASS' and 'speedup' in r]
    if speedups:
        avg_speedup = sum(speedups) / len(speedups)
        max_speedup = max(speedups)
        min_speedup = min(speedups)
        faster_count = sum(1 for s in speedups if s > 1.0)
        
        print(f"\nPerformance (AlphaTA vs TA-Lib):")
        print(f"Average speedup: {avg_speedup:.2f}x")
        print(f"Max speedup: {max_speedup:.2f}x")
        print(f"Min speedup: {min_speedup:.2f}x")
        print(f"Indicators faster than TA-Lib: {faster_count}/{len(speedups)} ({faster_count/len(speedups)*100:.1f}%)")
    
    # Save detailed results
    output_file = 'comprehensive_test_results.json'
    
    # Convert numpy bools to Python bools for JSON serialization
    def convert_to_serializable(obj):
        if isinstance(obj, (np.bool_, np.int32, np.int64, np.float32, np.float64)):
            return obj.item()
        elif isinstance(obj, dict):
            return {k: convert_to_serializable(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [convert_to_serializable(item) for item in obj]
        return obj
    
    serializable_results = convert_to_serializable(results)
    
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump({
            'summary': {
                'total': len(indicators),
                'passed': pass_count,
                'failed': fail_count,
                'errors': error_count,
                'pass_rate': pass_count / len(indicators) * 100,
                'avg_speedup': avg_speedup if speedups else 0,
            },
            'results': serializable_results
        }, f, indent=2, ensure_ascii=False)
    
    print(f"\nDetailed results saved to: {output_file}")
    
    # List failed indicators
    if fail_count > 0:
        print("\n" + "=" * 80)
        print("FAILED INDICATORS (need attention)")
        print("=" * 80)
        for r in results:
            if r['status'] == 'FAIL':
                print(f"  - {r['name']}: max_diff = {r['max_diff']:.2e}")
    
    return results

if __name__ == '__main__':
    main()
