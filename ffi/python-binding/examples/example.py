"""
Example: Using finkit in Python
"""
import numpy as np
import finkit as ta

def generate_sample_data(n=100):
    """Generate sample OHLCV data"""
    close = np.linspace(1.0, 10.0, n) + np.random.normal(0, 0.1, n)
    high = close * (1 + np.random.uniform(0.01, 0.03, n))
    low = close * (1 - np.random.uniform(0.01, 0.03, n))
    open_price = close + np.random.normal(0, 0.05, n)
    volume = np.random.uniform(1000, 2000, n)
    return open_price, high, low, close, volume

def main():
    # Generate sample data
    open_price, high, low, close, volume = generate_sample_data(200)
    
    print("=" * 60)
    print("finkit Python Example")
    print("=" * 60)
    
    # Overlap Studies
    print("\n--- Overlap Studies ---")
    sma_14 = ta.sma(close, timeperiod=14)
    ema_14 = ta.ema(close, timeperiod=14)
    bbands = ta.bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
    
    print(f"SMA(14): {sma_14[-1]:.4f}")
    print(f"EMA(14): {ema_14[-1]:.4f}")
    print(f"BBANDS Upper: {bbands[0][-1]:.4f}")
    print(f"BBANDS Middle: {bbands[1][-1]:.4f}")
    print(f"BBANDS Lower: {bbands[2][-1]:.4f}")
    
    # Momentum Indicators
    print("\n--- Momentum Indicators ---")
    rsi = ta.rsi(close, timeperiod=14)
    macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
    slowk, slowd = ta.stoch(high, low, close, fastk_period=5, slowk_period=3, slowd_period=3)
    
    print(f"RSI(14): {rsi[-1]:.4f}")
    print(f"MACD: {macd[-1]:.4f}")
    print(f"Signal: {signal[-1]:.4f}")
    print(f"Histogram: {hist[-1]:.4f}")
    print(f"Stochastic K: {slowk[-1]:.4f}")
    print(f"Stochastic D: {slowd[-1]:.4f}")
    
    # Volatility Indicators
    print("\n--- Volatility Indicators ---")
    atr = ta.atr(high, low, close, timeperiod=14)
    print(f"ATR(14): {atr[-1]:.4f}")
    
    # Volume Indicators
    print("\n--- Volume Indicators ---")
    obv = ta.obv(close, volume)
    print(f"OBV: {obv[-1]:.2f}")
    
    # Candlestick Patterns
    print("\n--- Candlestick Patterns ---")
    doji = ta.cdl_doji(open_price, high, low, close, doji_pct=0.1)
    hammer = ta.cdl_hammer(open_price, high, low, close)
    engulfing = ta.cdl_engulfing(open_price, high, low, close)
    
    doji_count = sum(1 for x in doji if x != 0)
    hammer_count = sum(1 for x in hammer if x != 0)
    engulfing_count = sum(1 for x in engulfing if x != 0)
    
    print(f"Doji patterns detected: {doji_count}")
    print(f"Hammer patterns detected: {hammer_count}")
    print(f"Engulfing patterns detected: {engulfing_count}")
    
    # Chart Patterns
    print("\n--- Chart Patterns ---")
    double_tops = ta.detect_double_top(high, lookback=20, tolerance=0.03)
    double_bottoms = ta.detect_double_bottom(low, lookback=20, tolerance=0.03)
    
    print(f"Double tops detected: {len(double_tops)}")
    print(f"Double bottoms detected: {len(double_bottoms)}")
    
    # Trading Signal Example
    print("\n--- Trading Signal Example ---")
    # Simple RSI + MACD strategy
    if rsi[-1] < 30 and macd[-1] > signal[-1]:
        print("Signal: BUY (Oversold RSI + MACD crossover)")
    elif rsi[-1] > 70 and macd[-1] < signal[-1]:
        print("Signal: SELL (Overbought RSI + MACD crossunder)")
    else:
        print("Signal: HOLD")
    
    print("\n" + "=" * 60)
    print("Example completed successfully!")
    print("=" * 60)

if __name__ == "__main__":
    main()
