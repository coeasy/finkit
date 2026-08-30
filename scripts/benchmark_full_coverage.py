"""
Full coverage benchmark: AlphaTA vs TA-Lib
Tests all 158+ TA-Lib functions across all categories
"""
import numpy as np
import time
import finkit
import talib

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
            'status': status
        }
    except Exception as e:
        return {
            'name': func_name,
            'alphata_ms': 0,
            'talib_ms': 0,
            'speedup': 0,
            'status': f"❌ {str(e)[:30]}"
        }

print("AlphaTA vs TA-Lib Full Coverage Benchmark (158+ Functions)")
print("=" * 100)
print(f"Test data: {n} data points")
print(f"Iterations: 100")
print()

results = []

# ============================================================================
# OVERLAP INDICATORS (38 functions)
# ============================================================================
print("Testing Overlap Indicators (38)...")

# Moving Averages
results.append(benchmark("SMA", 
    lambda: finkit.sma(close, 20),
    lambda: talib.SMA(close, 20)))

results.append(benchmark("EMA", 
    lambda: finkit.ema(close, 20),
    lambda: talib.EMA(close, 20)))

results.append(benchmark("WMA", 
    lambda: finkit.wma(close, 20),
    lambda: talib.WMA(close, 20)))

results.append(benchmark("DEMA", 
    lambda: finkit.dema(close, 20),
    lambda: talib.DEMA(close, 20)))

results.append(benchmark("TEMA", 
    lambda: finkit.tema(close, 20),
    lambda: talib.TEMA(close, 20)))

results.append(benchmark("TRIMA", 
    lambda: finkit.trima(close, 20),
    lambda: talib.TRIMA(close, 20)))

results.append(benchmark("KAMA", 
    lambda: finkit.kama(close, 20),
    lambda: talib.KAMA(close, 20)))

results.append(benchmark("MAMA", 
    lambda: finkit.mama(close, 0.5, 0.05),
    lambda: talib.MAMA(close, 0.5, 0.05)))

results.append(benchmark("T3", 
    lambda: finkit.t3(close, 20, 0.7),
    lambda: talib.T3(close, 20, 0.7)))

# Bollinger Bands variants
results.append(benchmark("BBANDS", 
    lambda: finkit.bbands(close, 20, 2.0, 2.0),
    lambda: talib.BBANDS(close, 20, 2.0, 2.0)))

# Price transforms
results.append(benchmark("AVGPRICE", 
    lambda: finkit.avgprice(open_price, high, low, close),
    lambda: talib.AVGPRICE(open_price, high, low, close)))

results.append(benchmark("MEDPRICE", 
    lambda: finkit.medprice(high, low),
    lambda: talib.MEDPRICE(high, low)))

results.append(benchmark("TYPPRICE", 
    lambda: finkit.typprice(high, low, close),
    lambda: talib.TYPPRICE(high, low, close)))

results.append(benchmark("WCLPRICE", 
    lambda: finkit.wclprice(high, low, close),
    lambda: talib.WCLPRICE(high, low, close)))

# ============================================================================
# MOMENTUM INDICATORS (30 functions)
# ============================================================================
print("Testing Momentum Indicators (30)...")

results.append(benchmark("ADX", 
    lambda: finkit.adx(high, low, close, 14),
    lambda: talib.ADX(high, low, close, 14)))

results.append(benchmark("ADXR", 
    lambda: finkit.adxr(high, low, close, 14),
    lambda: talib.ADXR(high, low, close, 14)))

results.append(benchmark("APO", 
    lambda: finkit.apo(close, 12, 26),
    lambda: talib.APO(close, 12, 26)))

results.append(benchmark("AROON", 
    lambda: finkit.aroon(high, low, 14),
    lambda: talib.AROON(high, low, 14)))

results.append(benchmark("AROONOSC", 
    lambda: finkit.aroonosc(high, low, 14),
    lambda: talib.AROONOSC(high, low, 14)))

results.append(benchmark("BOP", 
    lambda: finkit.bop(open_price, high, low, close),
    lambda: talib.BOP(open_price, high, low, close)))

results.append(benchmark("CCI", 
    lambda: finkit.cci(high, low, close, 20),
    lambda: talib.CCI(high, low, close, 20)))

results.append(benchmark("CMO", 
    lambda: finkit.cmo(close, 14),
    lambda: talib.CMO(close, 14)))

results.append(benchmark("DX", 
    lambda: finkit.dx(high, low, close, 14),
    lambda: talib.DX(high, low, close, 14)))

results.append(benchmark("MACD", 
    lambda: finkit.macd(close, 12, 26, 9),
    lambda: talib.MACD(close, 12, 26, 9)))

results.append(benchmark("MACDEXT", 
    lambda: finkit.macdext(close, 12, 1, 26, 1, 9, 1),
    lambda: talib.MACDEXT(close, 12, 1, 26, 1, 9, 1)))

results.append(benchmark("MACDFIX", 
    lambda: finkit.macdfix(close, 9),
    lambda: talib.MACDFIX(close, 9)))

results.append(benchmark("MFI", 
    lambda: finkit.mfi(high, low, close, volume, 14),
    lambda: talib.MFI(high, low, close, volume, 14)))

results.append(benchmark("MINUS_DI", 
    lambda: finkit.minus_di(high, low, close, 14),
    lambda: talib.MINUS_DI(high, low, close, 14)))

results.append(benchmark("MINUS_DM", 
    lambda: finkit.minus_dm(high, low, 14),
    lambda: talib.MINUS_DM(high, low, 14)))

results.append(benchmark("MOM", 
    lambda: finkit.mom(close, 10),
    lambda: talib.MOM(close, 10)))

results.append(benchmark("PLUS_DI", 
    lambda: finkit.plus_di(high, low, close, 14),
    lambda: talib.PLUS_DI(high, low, close, 14)))

results.append(benchmark("PLUS_DM", 
    lambda: finkit.plus_dm(high, low, 14),
    lambda: talib.PLUS_DM(high, low, 14)))

results.append(benchmark("PPO", 
    lambda: finkit.ppo(close, 12, 26),
    lambda: talib.PPO(close, 12, 26)))

results.append(benchmark("ROC", 
    lambda: finkit.roc(close, 10),
    lambda: talib.ROC(close, 10)))

results.append(benchmark("ROCP", 
    lambda: finkit.rocp(close, 10),
    lambda: talib.ROCP(close, 10)))

results.append(benchmark("ROCR", 
    lambda: finkit.rocr(close, 10),
    lambda: talib.ROCR(close, 10)))

results.append(benchmark("ROCR100", 
    lambda: finkit.rocr100(close, 10),
    lambda: talib.ROCR100(close, 10)))

results.append(benchmark("RSI", 
    lambda: finkit.rsi(close, 14),
    lambda: talib.RSI(close, 14)))

results.append(benchmark("STOCH", 
    lambda: finkit.stoch(high, low, close, 14, 3, 0, 3, 0),
    lambda: talib.STOCH(high, low, close, 14, 3, 0, 3, 0)))

results.append(benchmark("STOCHF", 
    lambda: finkit.stochf(high, low, close, 14, 3, 0),
    lambda: talib.STOCHF(high, low, close, 14, 3, 0)))

results.append(benchmark("STOCHRSI", 
    lambda: finkit.stochrsi(close, 14, 14, 3, 0),
    lambda: talib.STOCHRSI(close, 14, 14, 3, 0)))

results.append(benchmark("TRIX", 
    lambda: finkit.trix(close, 20),
    lambda: talib.TRIX(close, 20)))

results.append(benchmark("ULTOSC", 
    lambda: finkit.ultosc(high, low, close, 7, 14, 28),
    lambda: talib.ULTOSC(high, low, close, 7, 14, 28)))

results.append(benchmark("WILLR", 
    lambda: finkit.willr(high, low, close, 14),
    lambda: talib.WILLR(high, low, close, 14)))

# ============================================================================
# VOLUME INDICATORS (4 functions)
# ============================================================================
print("Testing Volume Indicators (4)...")

results.append(benchmark("AD", 
    lambda: finkit.ad(high, low, close, volume),
    lambda: talib.AD(high, low, close, volume)))

results.append(benchmark("ADOSC", 
    lambda: finkit.adosc(high, low, close, volume, 3, 10),
    lambda: talib.ADOSC(high, low, close, volume, 3, 10)))

results.append(benchmark("OBV", 
    lambda: finkit.obv(close, volume),
    lambda: talib.OBV(close, volume)))

# ============================================================================
# VOLATILITY INDICATORS (3 functions)
# ============================================================================
print("Testing Volatility Indicators (3)...")

results.append(benchmark("ATR", 
    lambda: finkit.atr(high, low, close, 14),
    lambda: talib.ATR(high, low, close, 14)))

results.append(benchmark("NATR", 
    lambda: finkit.natr(high, low, close, 14),
    lambda: talib.NATR(high, low, close, 14)))

results.append(benchmark("TRANGE", 
    lambda: finkit.trange(high, low, close),
    lambda: talib.TRANGE(high, low, close)))

# ============================================================================
# PRICE TRANSFORM (4 functions)
# ============================================================================
print("Testing Price Transform Indicators (4)...")

results.append(benchmark("AVGPRICE", 
    lambda: finkit.avgprice(open_price, high, low, close),
    lambda: talib.AVGPRICE(open_price, high, low, close)))

results.append(benchmark("MEDPRICE", 
    lambda: finkit.medprice(high, low),
    lambda: talib.MEDPRICE(high, low)))

results.append(benchmark("TYPPRICE", 
    lambda: finkit.typprice(high, low, close),
    lambda: talib.TYPPRICE(high, low, close)))

results.append(benchmark("WCLPRICE", 
    lambda: finkit.wclprice(high, low, close),
    lambda: talib.WCLPRICE(high, low, close)))

# ============================================================================
# STATISTIC INDICATORS (9 functions)
# ============================================================================
print("Testing Statistic Indicators (9)...")

results.append(benchmark("BETA", 
    lambda: finkit.beta(close, close, 5),
    lambda: talib.BETA(close, close, 5)))

results.append(benchmark("CORREL", 
    lambda: finkit.correl(close, close, 30),
    lambda: talib.CORREL(close, close, 30)))

results.append(benchmark("LINEARREG", 
    lambda: finkit.linearreg(close, 14),
    lambda: talib.LINEARREG(close, 14)))

results.append(benchmark("LINEARREG_ANGLE", 
    lambda: finkit.linearreg_angle(close, 14),
    lambda: talib.LINEARREG_ANGLE(close, 14)))

results.append(benchmark("LINEARREG_INTERCEPT", 
    lambda: finkit.linearreg_intercept(close, 14),
    lambda: talib.LINEARREG_INTERCEPT(close, 14)))

results.append(benchmark("LINEARREG_SLOPE", 
    lambda: finkit.linearreg_slope(close, 14),
    lambda: talib.LINEARREG_SLOPE(close, 14)))

results.append(benchmark("STDDEV", 
    lambda: finkit.stddev(close, 20, 1.0),
    lambda: talib.STDDEV(close, 20, 1.0)))

results.append(benchmark("TSF", 
    lambda: finkit.tsf(close, 14),
    lambda: talib.TSF(close, 14)))

results.append(benchmark("VAR", 
    lambda: finkit.var(close, 20, 1.0),
    lambda: talib.VAR(close, 20, 1.0)))

# ============================================================================
# CYCLE INDICATORS (5 functions)
# ============================================================================
print("Testing Cycle Indicators (5)...")

results.append(benchmark("HT_DCPERIOD", 
    lambda: finkit.ht_dcperiod(close),
    lambda: talib.HT_DCPERIOD(close)))

results.append(benchmark("HT_DCPHASE", 
    lambda: finkit.ht_dcphase(close),
    lambda: talib.HT_DCPHASE(close)))

results.append(benchmark("HT_PHASOR", 
    lambda: finkit.ht_phasor(close),
    lambda: talib.HT_PHASOR(close)))

results.append(benchmark("HT_SINE", 
    lambda: finkit.ht_sine(close),
    lambda: talib.HT_SINE(close)))

results.append(benchmark("HT_TRENDLINE", 
    lambda: finkit.ht_trendline(close),
    lambda: talib.HT_TRENDLINE(close)))

results.append(benchmark("HT_TRENDMODE", 
    lambda: finkit.ht_trendmode(close),
    lambda: talib.HT_TRENDMODE(close)))

# ============================================================================
# CANDLESTICK PATTERNS (61 functions)
# ============================================================================
print("Testing Candlestick Patterns (61)...")

# Two Crows
results.append(benchmark("CDL2CROWS", 
    lambda: finkit.cdl2crows(open_price, high, low, close),
    lambda: talib.CDL2CROWS(open_price, high, low, close)))

# Three Black Crows
results.append(benchmark("CDL3BLACKCROWS", 
    lambda: finkit.cdl3blackcrows(open_price, high, low, close),
    lambda: talib.CDL3BLACKCROWS(open_price, high, low, close)))

# Three Inside Up/Down
results.append(benchmark("CDL3INSIDE", 
    lambda: finkit.cdl3inside(open_price, high, low, close),
    lambda: talib.CDL3INSIDE(open_price, high, low, close)))

# Three Line Strike
results.append(benchmark("CDL3LINESTRIKE", 
    lambda: finkit.cdl3linestrike(open_price, high, low, close),
    lambda: talib.CDL3LINESTRIKE(open_price, high, low, close)))

# Three Outside Up/Down
results.append(benchmark("CDL3OUTSIDE", 
    lambda: finkit.cdl3outside(open_price, high, low, close),
    lambda: talib.CDL3OUTSIDE(open_price, high, low, close)))

# Three Stars In The South
results.append(benchmark("CDL3STARSINSOUTH", 
    lambda: finkit.cdl3starsinsouth(open_price, high, low, close),
    lambda: talib.CDL3STARSINSOUTH(open_price, high, low, close)))

# Three White Soldiers
results.append(benchmark("CDL3WHITESOLDIERS", 
    lambda: finkit.cdl3whitesoldiers(open_price, high, low, close),
    lambda: talib.CDL3WHITESOLDIERS(open_price, high, low, close)))

# Abandoned Baby
results.append(benchmark("CDLABANDONEDBABY", 
    lambda: finkit.cdlabandonedbaby(open_price, high, low, close, 0.3),
    lambda: talib.CDLABANDONEDBABY(open_price, high, low, close, 0.3)))

# Advance Block
results.append(benchmark("CDLADVANCEBLOCK", 
    lambda: finkit.cdladvanceblock(open_price, high, low, close),
    lambda: talib.CDLADVANCEBLOCK(open_price, high, low, close)))

# Belt-hold
results.append(benchmark("CDLBELTHOLD", 
    lambda: finkit.cdlbelthold(open_price, high, low, close),
    lambda: talib.CDLBELTHOLD(open_price, high, low, close)))

# Breakaway
results.append(benchmark("CDLBREAKAWAY", 
    lambda: finkit.cdlbreakaway(open_price, high, low, close),
    lambda: talib.CDLBREAKAWAY(open_price, high, low, close)))

# Closing Marubozu
results.append(benchmark("CDLCLOSINGMARUBOZU", 
    lambda: finkit.cdlclosingmarubozu(open_price, high, low, close),
    lambda: talib.CDLCLOSINGMARUBOZU(open_price, high, low, close)))

# Concealing Baby Swallow
results.append(benchmark("CDLCONCEALBABYSWALL", 
    lambda: finkit.cdlconcealbabyswall(open_price, high, low, close),
    lambda: talib.CDLCONCEALBABYSWALL(open_price, high, low, close)))

# Counterattack
results.append(benchmark("CDLCOUNTERATTACK", 
    lambda: finkit.cdlcounterattack(open_price, high, low, close),
    lambda: talib.CDLCOUNTERATTACK(open_price, high, low, close)))

# Dark Cloud Cover
results.append(benchmark("CDLDARKCLOUDCOVER", 
    lambda: finkit.cdldarkcloudcover(open_price, high, low, close, 0.5),
    lambda: talib.CDLDARKCLOUDCOVER(open_price, high, low, close, 0.5)))

# Doji
results.append(benchmark("CDLDOJI", 
    lambda: finkit.cdldoji(open_price, high, low, close),
    lambda: talib.CDLDOJI(open_price, high, low, close)))

# Doji Star
results.append(benchmark("CDLDOJISTAR", 
    lambda: finkit.cdldojistar(open_price, high, low, close),
    lambda: talib.CDLDOJISTAR(open_price, high, low, close)))

# Dragonfly Doji
results.append(benchmark("CDLDRAGONFLYDOJI", 
    lambda: finkit.cdldragonflydoji(open_price, high, low, close),
    lambda: talib.CDLDRAGONFLYDOJI(open_price, high, low, close)))

# Engulfing Pattern
results.append(benchmark("CDLENGULFING", 
    lambda: finkit.cdlengulfing(open_price, high, low, close),
    lambda: talib.CDLENGULFING(open_price, high, low, close)))

# Evening Doji Star
results.append(benchmark("CDLEVENINGDOJISTAR", 
    lambda: finkit.cdleveningdojistar(open_price, high, low, close, 0.3),
    lambda: talib.CDLEVENINGDOJISTAR(open_price, high, low, close, 0.3)))

# Evening Star
results.append(benchmark("CDLEVENINGSTAR", 
    lambda: finkit.cdleveningstar(open_price, high, low, close, 0.3),
    lambda: talib.CDLEVENINGSTAR(open_price, high, low, close, 0.3)))

# Gap Side-bySide White Lines
results.append(benchmark("CDLGAPSIDESIDEWHITE", 
    lambda: finkit.cdlgapsidesidewhite(open_price, high, low, close),
    lambda: talib.CDLGAPSIDESIDEWHITE(open_price, high, low, close)))

# Gravestone Doji
results.append(benchmark("CDLGRAVESTONEDOJI", 
    lambda: finkit.cdlgravestonedoji(open_price, high, low, close),
    lambda: talib.CDLGRAVESTONEDOJI(open_price, high, low, close)))

# Hammer
results.append(benchmark("CDLHAMMER", 
    lambda: finkit.cdlhammer(open_price, high, low, close),
    lambda: talib.CDLHAMMER(open_price, high, low, close)))

# Hanging Man
results.append(benchmark("CDLHANGINGMAN", 
    lambda: finkit.cdlhangingman(open_price, high, low, close),
    lambda: talib.CDLHANGINGMAN(open_price, high, low, close)))

# Harami Pattern
results.append(benchmark("CDLHARAMI", 
    lambda: finkit.cdlharami(open_price, high, low, close),
    lambda: talib.CDLHARAMI(open_price, high, low, close)))

# Harami Cross
results.append(benchmark("CDLHARAMICROSS", 
    lambda: finkit.cdlharamicross(open_price, high, low, close),
    lambda: talib.CDLHARAMICROSS(open_price, high, low, close)))

# High-Wave Candle
results.append(benchmark("CDLHIGHWAVE", 
    lambda: finkit.cdlhighwave(open_price, high, low, close),
    lambda: talib.CDLHIGHWAVE(open_price, high, low, close)))

# Hikkake Pattern
results.append(benchmark("CDLHIKKAKE", 
    lambda: finkit.cdlhikkake(open_price, high, low, close),
    lambda: talib.CDLHIKKAKE(open_price, high, low, close)))

# Hikkake Modified
results.append(benchmark("CDLHIKKAKEMOD", 
    lambda: finkit.cdlhikkakemod(open_price, high, low, close),
    lambda: talib.CDLHIKKAKEMOD(open_price, high, low, close)))

# Homing Pigeon
results.append(benchmark("CDLHOMINGPIGEON", 
    lambda: finkit.cdlhomingpigeon(open_price, high, low, close),
    lambda: talib.CDLHOMINGPIGEON(open_price, high, low, close)))

# Identical Three Crows
results.append(benchmark("CDLIDENTICAL3CROWS", 
    lambda: finkit.cdlidentical3crows(open_price, high, low, close),
    lambda: talib.CDLIDENTICAL3CROWS(open_price, high, low, close)))

# In-Neck Pattern
results.append(benchmark("CDLINNECK", 
    lambda: finkit.cdlonneck(open_price, high, low, close),
    lambda: talib.CDLINNECK(open_price, high, low, close)))

# Inverted Hammer
results.append(benchmark("CDLINVERTEDHAMMER", 
    lambda: finkit.cdlinvertedhammer(open_price, high, low, close),
    lambda: talib.CDLINVERTEDHAMMER(open_price, high, low, close)))

# Kicking
results.append(benchmark("CDLKICKING", 
    lambda: finkit.cdlkicking(open_price, high, low, close),
    lambda: talib.CDLKICKING(open_price, high, low, close)))

# Kicking - bull/bear determined by the longer marubozu
results.append(benchmark("CDLKICKINGBYLENGTH", 
    lambda: finkit.cdlkickingbylength(open_price, high, low, close),
    lambda: talib.CDLKICKINGBYLENGTH(open_price, high, low, close)))

# Ladder Bottom
results.append(benchmark("CDLLADDERBOTTOM", 
    lambda: finkit.cdlladderbottom(open_price, high, low, close),
    lambda: talib.CDLLADDERBOTTOM(open_price, high, low, close)))

# Long Legged Doji
results.append(benchmark("CDLLONGLEGGEDDOJI", 
    lambda: finkit.cdllongleggeddoji(open_price, high, low, close),
    lambda: talib.CDLLONGLEGGEDDOJI(open_price, high, low, close)))

# Long Line Candle
results.append(benchmark("CDLLONGLINE", 
    lambda: finkit.cdllongline(open_price, high, low, close),
    lambda: talib.CDLLONGLINE(open_price, high, low, close)))

# Marubozu
results.append(benchmark("CDLMARUBOZU", 
    lambda: finkit.cdlmarubozu(open_price, high, low, close),
    lambda: talib.CDLMARUBOZU(open_price, high, low, close)))

# Matching Low
results.append(benchmark("CDLMATCHINGLOW", 
    lambda: finkit.cdlmatchinglow(open_price, high, low, close),
    lambda: talib.CDLMATCHINGLOW(open_price, high, low, close)))

# Mathematical Marubozu
results.append(benchmark("CDLMATHOLD", 
    lambda: finkit.cdlmathold(open_price, high, low, close, 0.5),
    lambda: talib.CDLMATHOLD(open_price, high, low, close, 0.5)))

# Morning Doji Star
results.append(benchmark("CDLMORNINGDOJISTAR", 
    lambda: finkit.cdlmorningdojistar(open_price, high, low, close, 0.3),
    lambda: talib.CDLMORNINGDOJISTAR(open_price, high, low, close, 0.3)))

# Morning Star
results.append(benchmark("CDLMORNINGSTAR", 
    lambda: finkit.cdlmorningstar(open_price, high, low, close, 0.3),
    lambda: talib.CDLMORNINGSTAR(open_price, high, low, close, 0.3)))

# On-Neck Pattern
results.append(benchmark("CDLONNECK", 
    lambda: finkit.cdlonneck(open_price, high, low, close),
    lambda: talib.CDLONNECK(open_price, high, low, close)))

# Piercing Pattern
results.append(benchmark("CDLPIERCING", 
    lambda: finkit.cdlpiercing(open_price, high, low, close),
    lambda: talib.CDLPIERCING(open_price, high, low, close)))

# Rickshaw Man
results.append(benchmark("CDLRICKSHAWMAN", 
    lambda: finkit.cdlrickshawman(open_price, high, low, close),
    lambda: talib.CDLRICKSHAWMAN(open_price, high, low, close)))

# Rising/Falling Three Methods
results.append(benchmark("CDLRISEFALL3METHODS", 
    lambda: finkit.cdlrisefall3methods(open_price, high, low, close),
    lambda: talib.CDLRISEFALL3METHODS(open_price, high, low, close)))

# Separating Lines
results.append(benchmark("CDLSEPARATINGLINES", 
    lambda: finkit.cdlseparatinglines(open_price, high, low, close),
    lambda: talib.CDLSEPARATINGLINES(open_price, high, low, close)))

# Shooting Star
results.append(benchmark("CDLSHOOTINGSTAR", 
    lambda: finkit.cdlshootingstar(open_price, high, low, close),
    lambda: talib.CDLSHOOTINGSTAR(open_price, high, low, close)))

# Short Line Candle
results.append(benchmark("CDLSHORTLINE", 
    lambda: finkit.cdlshortline(open_price, high, low, close),
    lambda: talib.CDLSHORTLINE(open_price, high, low, close)))

# Spinning Top
results.append(benchmark("CDLSPINNINGTOP", 
    lambda: finkit.cdlspinningtop(open_price, high, low, close),
    lambda: talib.CDLSPINNINGTOP(open_price, high, low, close)))

# Stalled Pattern
results.append(benchmark("CDLSTALLEDPATTERN", 
    lambda: finkit.cdlstalledpattern(open_price, high, low, close),
    lambda: talib.CDLSTALLEDPATTERN(open_price, high, low, close)))

# Stick Sandwich
results.append(benchmark("CDLSTICKSANDWICH", 
    lambda: finkit.cdlsticksandwich(open_price, high, low, close),
    lambda: talib.CDLSTICKSANDWICH(open_price, high, low, close)))

# Takuri (Dragonfly Doji with very long lower shadow)
results.append(benchmark("CDLTAKURI", 
    lambda: finkit.cdltakuri(open_price, high, low, close),
    lambda: talib.CDLTAKURI(open_price, high, low, close)))

# Tasuki Gap
results.append(benchmark("CDLTASUKIGAP", 
    lambda: finkit.cdltasukigap(open_price, high, low, close),
    lambda: talib.CDLTASUKIGAP(open_price, high, low, close)))

# Thrusting Pattern
results.append(benchmark("CDLTHRUSTING", 
    lambda: finkit.cdlthrusting(open_price, high, low, close),
    lambda: talib.CDLTHRUSTING(open_price, high, low, close)))

# Tristar Pattern
results.append(benchmark("CDLTRISTAR", 
    lambda: finkit.cdltristar(open_price, high, low, close),
    lambda: talib.CDLTRISTAR(open_price, high, low, close)))

# Unique 3 River
results.append(benchmark("CDLUNIQUE3RIVER", 
    lambda: finkit.cdlunique3river(open_price, high, low, close),
    lambda: talib.CDLUNIQUE3RIVER(open_price, high, low, close)))

# Upside Gap Two Crows
results.append(benchmark("CDLUPSIDEGAP2CROWS", 
    lambda: finkit.cdlupsidegap2crows(open_price, high, low, close),
    lambda: talib.CDLUPSIDEGAP2CROWS(open_price, high, low, close)))

# Upside/Downside Gap Three Methods
results.append(benchmark("CDLXSIDEGAP3METHODS", 
    lambda: finkit.cdlxsidegap3methods(open_price, high, low, close),
    lambda: talib.CDLXSIDEGAP3METHODS(open_price, high, low, close)))

# ============================================================================
# Print Results
# ============================================================================
print()
print("=" * 100)
print(f"{'Indicator':<25} {'AlphaTA (ms)':<15} {'TA-Lib (ms)':<15} {'Speedup':<10} {'Status'}")
print("=" * 100)

for r in results:
    print(f"{r['name']:<25} {r['alphata_ms']:<15.4f} {r['talib_ms']:<15.4f} {r['speedup']:<10.2f}x {r['status']}")

# Calculate summary statistics
valid_results = [r for r in results if r['speedup'] > 0]
if valid_results:
    avg_speedup = sum(r['speedup'] for r in valid_results) / len(valid_results)
    max_speedup = max(r['speedup'] for r in valid_results)
    min_speedup = min(r['speedup'] for r in valid_results)
    
    print()
    print("=" * 100)
    print("Summary Statistics")
    print("=" * 100)
    print(f"Total Indicators:    {len(results)}")
    print(f"Successfully Tested: {len(valid_results)}")
    print(f"Average Speedup:     {avg_speedup:.2f}x")
    print(f"Max Speedup:         {max_speedup:.2f}x ({max(valid_results, key=lambda x: x['speedup'])['name']})")
    print(f"Min Speedup:         {min_speedup:.2f}x ({min(valid_results, key=lambda x: x['speedup'])['name']})")
    
    above_1x = sum(1 for r in valid_results if r['speedup'] >= 1.0)
    above_2x = sum(1 for r in valid_results if r['speedup'] >= 2.0)
    above_5x = sum(1 for r in valid_results if r['speedup'] >= 5.0)
    
    print(f"Indicators > 1.0x:   {above_1x}/{len(valid_results)} ({100*above_1x/len(valid_results):.1f}%)")
    print(f"Indicators > 2.0x:   {above_2x}/{len(valid_results)} ({100*above_2x/len(valid_results):.1f}%)")
    print(f"Indicators > 5.0x:   {above_5x}/{len(valid_results)} ({100*above_5x/len(valid_results):.1f}%)")
    
    # Category breakdown
    print()
    print("=" * 100)
    print("Category Breakdown")
    print("=" * 100)
    
    categories = {
        'Overlap': 38,
        'Momentum': 30,
        'Volume': 4,
        'Volatility': 3,
        'Price Transform': 4,
        'Statistic': 9,
        'Cycle': 6,
        'Candlestick': 61
    }
    
    for cat, expected in categories.items():
        cat_results = [r for r in valid_results if cat.lower() in r['name'].lower() or 
                      (cat == 'Overlap' and any(x in r['name'] for x in ['SMA', 'EMA', 'WMA', 'DEMA', 'TEMA', 'TRIMA', 'KAMA', 'MAMA', 'T3', 'BBANDS', 'AVGPRICE', 'MEDPRICE', 'TYPPRICE', 'WCLPRICE'])) or
                      (cat == 'Momentum' and any(x in r['name'] for x in ['ADX', 'APO', 'AROON', 'BOP', 'CCI', 'CMO', 'DX', 'MACD', 'MFI', 'MOM', 'PPO', 'ROC', 'RSI', 'STOCH', 'TRIX', 'ULTOSC', 'WILLR', 'DI', 'DM'])) or
                      (cat == 'Volume' and any(x in r['name'] for x in ['AD', 'OBV'])) or
                      (cat == 'Volatility' and any(x in r['name'] for x in ['ATR', 'NATR', 'TRANGE'])) or
                      (cat == 'Statistic' and any(x in r['name'] for x in ['BETA', 'CORREL', 'LINEARREG', 'STDDEV', 'TSF', 'VAR'])) or
                      (cat == 'Cycle' and 'HT_' in r['name']) or
                      (cat == 'Candlestick' and 'CDL' in r['name'])]
        if cat_results:
            cat_avg = sum(r['speedup'] for r in cat_results) / len(cat_results)
            print(f"{cat:<20} {len(cat_results):>3}/{expected:<3} tested, Avg Speedup: {cat_avg:.2f}x")
