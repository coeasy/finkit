#!/usr/bin/env python3
"""
分析 AlphaTA 与 TA-Lib 精度差异的根因
"""
import sys
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
volume = np.abs(np.random.randn(n)) * 1000 + 500

print("=" * 80)
print("精度差异根因分析")
print("=" * 80)

# 1. AD (Accumulation/Distribution) - 差异 1.20e+08
print("\n[1] AD (Accumulation/Distribution) - 差异: 1.20e+08")
print("-" * 80)
talib_ad = talib.AD(high, low, close, volume)
alpha_ad = np.array(finkit.ad(high, low, close, volume))
print(f"TA-Lib AD[0:10]:    {talib_ad[0:10]}")
print(f"AlphaTA AD[0:10]:   {alpha_ad[0:10]}")
print(f"TA-Lib AD[-10:]:    {talib_ad[-10:]}")
print(f"AlphaTA AD[-10:]:   {alpha_ad[-10:]}")
print(f"Max diff: {np.nanmax(np.abs(talib_ad - alpha_ad)):.2e}")
print(f"Mean diff: {np.nanmean(np.abs(talib_ad - alpha_ad)):.2e}")

# 2. AROON - 差异 100.0
print("\n[2] AROON - 差异: 100.0")
print("-" * 80)
talib_aroon_down, talib_aroon_up = talib.AROON(high, low, timeperiod=14)
alpha_result = finkit.aroon(high, low, timeperiod=14)
alpha_aroon_down, alpha_aroon_up = np.array(alpha_result[0]), np.array(alpha_result[1])
print(f"TA-Lib AROON_down[100:110]:  {talib_aroon_down[100:110]}")
print(f"AlphaTA AROON_down[100:110]: {alpha_aroon_down[100:110]}")
print(f"TA-Lib AROON_up[100:110]:    {talib_aroon_up[100:110]}")
print(f"AlphaTA AROON_up[100:110]:   {alpha_aroon_up[100:110]}")
print(f"Max diff down: {np.nanmax(np.abs(talib_aroon_down - alpha_aroon_down)):.2f}")
print(f"Max diff up: {np.nanmax(np.abs(talib_aroon_up - alpha_aroon_up)):.2f}")

# 3. CMO (Chande Momentum Oscillator) - 差异 99.13
print("\n[3] CMO - 差异: 99.13")
print("-" * 80)
talib_cmo = talib.CMO(close, timeperiod=14)
alpha_cmo = np.array(finkit.cmo(close, timeperiod=14))
print(f"TA-Lib CMO[100:110]:  {talib_cmo[100:110]}")
print(f"AlphaTA CMO[100:110]: {alpha_cmo[100:110]}")
print(f"Max diff: {np.nanmax(np.abs(talib_cmo[100:110] - alpha_cmo[100:110])):.2f}")
print(f"TA-Lib range: [{np.nanmin(talib_cmo):.2f}, {np.nanmax(talib_cmo):.2f}]")
print(f"AlphaTA range: [{np.nanmin(alpha_cmo):.2f}, {np.nanmax(alpha_cmo):.2f}]")

# 4. BBANDS (Bollinger Bands) - 差异 0.257
print("\n[4] BBANDS - 差异: 0.257")
print("-" * 80)
talib_upper, talib_mid, talib_lower = talib.BBANDS(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
alpha_result = finkit.bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
alpha_upper, alpha_mid, alpha_lower = np.array(alpha_result[0]), np.array(alpha_result[1]), np.array(alpha_result[2])
print(f"TA-Lib upper[100:105]:  {talib_upper[100:105]}")
print(f"AlphaTA upper[100:105]: {alpha_upper[100:105]}")
print(f"TA-Lib mid[100:105]:    {talib_mid[100:105]}")
print(f"AlphaTA mid[100:105]:   {alpha_mid[100:105]}")
print(f"Max diff upper: {np.nanmax(np.abs(talib_upper - alpha_upper)):.4f}")
print(f"Max diff mid: {np.nanmax(np.abs(talib_mid - alpha_mid)):.4f}")
print(f"Max diff lower: {np.nanmax(np.abs(talib_lower - alpha_lower)):.4f}")

# 5. MACD - 差异 0.032
print("\n[5] MACD - 差异: 0.032")
print("-" * 80)
talib_macd, talib_signal, talib_hist = talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_result = finkit.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
alpha_macd, alpha_signal, alpha_hist = np.array(alpha_result[0]), np.array(alpha_result[1]), np.array(alpha_result[2])
print(f"TA-Lib MACD[100:105]:    {talib_macd[100:105]}")
print(f"AlphaTA MACD[100:105]:   {alpha_macd[100:105]}")
print(f"TA-Lib Signal[100:105]:  {talib_signal[100:105]}")
print(f"AlphaTA Signal[100:105]: {alpha_signal[100:105]}")
print(f"Max diff MACD: {np.nanmax(np.abs(talib_macd - alpha_macd)):.4f}")
print(f"Max diff Signal: {np.nanmax(np.abs(talib_signal - alpha_signal)):.4f}")
print(f"Max diff Hist: {np.nanmax(np.abs(talib_hist - alpha_hist)):.4f}")

# 6. ATR (Average True Range) - 差异 0.288
print("\n[6] ATR - 差异: 0.288")
print("-" * 80)
talib_atr = talib.ATR(high, low, close, timeperiod=14)
alpha_atr = np.array(finkit.atr(high, low, close, timeperiod=14))
print(f"TA-Lib ATR[100:110]:  {talib_atr[100:110]}")
print(f"AlphaTA ATR[100:110]: {alpha_atr[100:110]}")
print(f"Max diff: {np.nanmax(np.abs(talib_atr - alpha_atr)):.4f}")
print(f"Relative diff: {np.nanmax(np.abs((talib_atr - alpha_atr) / (talib_atr + 1e-10))) * 100:.2f}%")

print("\n" + "=" * 80)
print("分析完成")
print("=" * 80)
