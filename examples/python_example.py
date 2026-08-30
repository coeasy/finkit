"""
alpha_ta Python 示例代码
展示如何使用 alpha_ta 进行技术分析
"""

import finkit as ta
import numpy as np
import pandas as pd

def basic_indicators():
    """基础指标计算示例"""
    print("=== 基础指标示例 ===")
    
    # 创建示例数据
    close = np.array([44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 
                      46.08, 46.32, 46.56, 46.80, 47.04, 47.28, 47.52])
    
    # 移动平均
    sma_5 = ta.sma(close, timeperiod=5)
    sma_10 = ta.sma(close, timeperiod=10)
    ema_5 = ta.ema(close, timeperiod=5)
    
    print(f"SMA(5): {sma_5[-3:]}")
    print(f"SMA(10): {sma_10[-3:]}")
    print(f"EMA(5): {ema_5[-3:]}")
    
    # RSI
    rsi = ta.rsi(close, timeperiod=14)
    print(f"RSI(14): {rsi[-1]:.2f}")
    
    # MACD
    macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
    print(f"MACD: {macd[-1]:.4f}, Signal: {signal[-1]:.4f}, Hist: {hist[-1]:.4f}")
    
    # 布林带
    upper, middle, lower = ta.bollinger_bands(close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0)
    print(f"布林带: Upper={upper[-1]:.2f}, Middle={middle[-1]:.2f}, Lower={lower[-1]:.2f}")


def ohlcv_analysis():
    """OHLCV 数据分析示例"""
    print("\n=== OHLCV 分析示例 ===")
    
    # 创建 OHLCV 数据
    n = 100
    np.random.seed(42)
    close = np.cumsum(np.random.randn(n)) + 100
    high = close + np.random.uniform(0.5, 2.0, n)
    low = close - np.random.uniform(0.5, 2.0, n)
    open_ = close + np.random.uniform(-1.0, 1.0, n)
    volume = np.random.uniform(1000, 5000, n)
    
    # ATR - 波动率
    atr = ta.atr(high, low, close, timeperiod=14)
    print(f"ATR(14): {atr[-1]:.4f}")
    
    # KDJ - 随机指标
    slowk, slowd = ta.stoch(high, low, close, fastk_period=9, slowk_period=3, slowd_period=3)
    print(f"KDJ: K={slowk[-1]:.2f}, D={slowd[-1]:.2f}")
    
    # ADX - 趋势强度
    adx = ta.adx(high, low, close, timeperiod=14)
    print(f"ADX(14): {adx[-1]:.2f}")
    
    # OBV - 成交量指标
    obv = ta.obv(close, volume)
    print(f"OBV: {obv[-1]:.2f}")
    
    # MFI - 资金流量指数
    mfi = ta.mfi(high, low, close, volume, timeperiod=14)
    print(f"MFI(14): {mfi[-1]:.2f}")


def candlestick_patterns():
    """K线形态识别示例"""
    print("\n=== K线形态识别示例 ===")
    
    # 创建 OHLC 数据
    n = 50
    np.random.seed(42)
    close = np.cumsum(np.random.randn(n)) + 100
    high = close + np.random.uniform(0.5, 2.0, n)
    low = close - np.random.uniform(0.5, 2.0, n)
    open_ = close + np.random.uniform(-1.0, 1.0, n)
    
    # 识别 K 线形态
    doji = ta.cdl_doji(open_, high, low, close)
    hammer = ta.cdl_hammer(open_, high, low, close)
    engulfing = ta.cdl_engulfing(open_, high, low, close)
    morning_star = ta.cdl_morningstar(open_, high, low, close)
    evening_star = ta.cdl_eveningstar(open_, high, low, close)
    
    # 统计形态数量
    print(f"十字星数量: {np.sum(doji != 0)}")
    print(f"锤子线数量: {np.sum(hammer != 0)}")
    print(f"吞没形态数量: {np.sum(engulfing != 0)}")
    print(f"晨星数量: {np.sum(morning_star != 0)}")
    print(f"晚星数量: {np.sum(evening_star != 0)}")
    
    # 显示最近的形态
    for i in range(-5, 0):
        patterns = []
        if doji[i] != 0:
            patterns.append(f"十字星({doji[i]})")
        if hammer[i] != 0:
            patterns.append(f"锤子线({hammer[i]})")
        if engulfing[i] != 0:
            patterns.append(f"吞没({engulfing[i]})")
        if patterns:
            print(f"第 {n+i} 根K线: {', '.join(patterns)}")


def trading_signals():
    """交易信号生成示例"""
    print("\n=== 交易信号示例 ===")
    
    # 创建模拟数据
    n = 100
    np.random.seed(42)
    close = np.cumsum(np.random.randn(n)) + 100
    high = close + np.random.uniform(0.5, 2.0, n)
    low = close - np.random.uniform(0.5, 2.0, n)
    
    # 计算多个指标
    sma_20 = ta.sma(close, timeperiod=20)
    sma_50 = ta.sma(close, timeperiod=50)
    rsi = ta.rsi(close, timeperiod=14)
    macd, signal, hist = ta.macd(close)
    
    # 生成交易信号
    signals = []
    for i in range(50, n):
        buy_signal = (
            sma_20[i] > sma_50[i] and  # 趋势向上
            rsi[i] < 30 and             # RSI 超卖
            hist[i] > 0                 # MACD 金叉
        )
        sell_signal = (
            sma_20[i] < sma_50[i] and  # 趋势向下
            rsi[i] > 70 and            # RSI 超买
            hist[i] < 0                # MACD 死叉
        )
        
        if buy_signal:
            signals.append(('BUY', i, close[i]))
        elif sell_signal:
            signals.append(('SELL', i, close[i]))
    
    print(f"生成信号数量: {len(signals)}")
    for signal in signals[:5]:
        print(f"  {signal[0]} @ 第{signal[1]}根K线, 价格={signal[2]:.2f}")


def streaming_indicator():
    """流式指标示例"""
    print("\n=== 流式指标示例 ===")
    
    # 创建流式 RSI
    streaming_rsi = ta.StreamingRSI(period=14)
    
    # 逐根 K 线更新
    prices = [44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 46.32]
    rsi_values = []
    
    for price in prices:
        rsi_value = streaming_rsi.update(price)
        rsi_values.append(rsi_value)
        print(f"Price: {price:.2f}, RSI: {rsi_value:.2f}")
    
    # 保存状态
    state = streaming_rsi.save()
    print(f"状态已保存: {state}")
    
    # 恢复状态
    new_rsi = ta.StreamingRSI.from_state(state)
    new_value = new_rsi.update(46.56)
    print(f"恢复后新值: RSI={new_value:.2f}")


def formula_engine():
    """公式引擎示例"""
    print("\n=== 公式引擎示例 ===")
    
    # 创建公式引擎
    engine = ta.FormulaEngine()
    
    # 示例数据
    close = np.array([44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84])
    
    # 计算布林带上轨
    upper = engine.evaluate("MA(CLOSE, 5) + 2 * STDDEV(CLOSE, 5)", close=close)
    print(f"布林带上轨: {upper[-1]:.2f}")
    
    # 计算自定义指标
    zscore = engine.evaluate("(CLOSE - MA(CLOSE, 5)) / STDDEV(CLOSE, 5)", close=close)
    print(f"Z-Score: {zscore[-1]:.4f}")


def pandas_integration():
    """Pandas 集成示例"""
    print("\n=== Pandas 集成示例 ===")
    
    # 创建 DataFrame
    df = pd.DataFrame({
        'date': pd.date_range('2024-01-01', periods=100),
        'close': np.cumsum(np.random.randn(100)) + 100,
        'high': np.cumsum(np.random.randn(100)) + 102,
        'low': np.cumsum(np.random.randn(100)) + 98,
        'volume': np.random.uniform(1000, 5000, 100)
    })
    
    # 计算指标并添加到 DataFrame
    df['sma_20'] = ta.sma(df['close'].values, timeperiod=20)
    df['ema_12'] = ta.ema(df['close'].values, timeperiod=12)
    df['rsi'] = ta.rsi(df['close'].values, timeperiod=14)
    
    # MACD
    macd, signal, hist = ta.macd(df['close'].values)
    df['macd'] = macd
    df['macd_signal'] = signal
    df['macd_hist'] = hist
    
    # ATR
    df['atr'] = ta.atr(df['high'].values, df['low'].values, df['close'].values, timeperiod=14)
    
    # 显示结果
    print(df[['date', 'close', 'sma_20', 'ema_12', 'rsi', 'atr']].tail(10))


if __name__ == '__main__':
    print("alpha_ta Python 示例代码")
    print("=" * 50)
    
    basic_indicators()
    ohlcv_analysis()
    candlestick_patterns()
    trading_signals()
    streaming_indicator()
    formula_engine()
    pandas_integration()
    
    print("\n" + "=" * 50)
    print("示例完成！")