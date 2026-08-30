package com.alphata.examples;

import com.alphata.Indicators;
import com.alphata.MacdResult;
import com.alphata.BbandsResult;
import com.alphata.StochResult;
import java.util.Arrays;

/**
 * alpha_ta Java 示例代码
 * 展示如何使用 alpha_ta 进行技术分析
 */
public class AlphaTAExample {

    public static void main(String[] args) {
        System.out.println("alpha_ta Java 示例代码");
        System.out.println("==================================================");
        
        basicIndicators();
        ohlcvAnalysis();
        candlestickPatterns();
        tradingSignals();
        completeAnalysis();
        
        System.out.println("\n==================================================");
        System.out.println("示例完成！");
    }
    
    /**
     * 基础指标计算示例
     */
    public static void basicIndicators() {
        System.out.println("\n=== 基础指标示例 ===");
        
        double[] close = {44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 
                          46.08, 46.32, 46.56, 46.80, 47.04, 47.28, 47.52};
        
        // 移动平均
        double[] sma5 = Indicators.sma(close, 5);
        double[] sma10 = Indicators.sma(close, 10);
        double[] ema5 = Indicators.ema(close, 5);
        
        System.out.println("SMA(5): " + Arrays.toString(Arrays.copyOfRange(sma5, sma5.length - 3, sma5.length)));
        System.out.println("SMA(10): " + Arrays.toString(Arrays.copyOfRange(sma10, sma10.length - 3, sma10.length)));
        System.out.println("EMA(5): " + Arrays.toString(Arrays.copyOfRange(ema5, ema5.length - 3, ema5.length)));
        
        // RSI
        double[] rsi = Indicators.rsi(close, 14);
        System.out.println("RSI(14): " + rsi[rsi.length - 1]);
        
        // MACD
        MacdResult macd = Indicators.macd(close, 12, 26, 9);
        System.out.println("MACD: " + macd.macd[macd.macd.length - 1] + 
                          ", Signal: " + macd.signal[macd.signal.length - 1] + 
                          ", Hist: " + macd.histogram[macd.histogram.length - 1]);
        
        // 布林带
        BbandsResult bbands = Indicators.bbands(close, 5, 2.0, 2.0);
        System.out.println("布林带: Upper=" + bbands.upperBand[bbands.upperBand.length - 1] + 
                          ", Middle=" + bbands.middleBand[bbands.middleBand.length - 1] + 
                          ", Lower=" + bbands.lowerBand[bbands.lowerBand.length - 1]);
    }
    
    /**
     * OHLCV 数据分析示例
     */
    public static void ohlcvAnalysis() {
        System.out.println("\n=== OHLCV 分析示例 ===");
        
        int n = 100;
        double[] close = new double[n];
        double[] high = new double[n];
        double[] low = new double[n];
        double[] open = new double[n];
        double[] volume = new double[n];
        
        // 生成模拟数据
        for (int i = 0; i < n; i++) {
            close[i] = 100 + i * 0.5 + Math.random() * 2 - 1;
            high[i] = close[i] + Math.random() * 2 + 0.5;
            low[i] = close[i] - Math.random() * 2 - 0.5;
            open[i] = close[i] + Math.random() * 2 - 1;
            volume[i] = Math.random() * 4000 + 1000;
        }
        
        // ATR - 波动率
        double[] atr = Indicators.atr(high, low, close, 14);
        System.out.println("ATR(14): " + atr[atr.length - 1]);
        
        // KDJ - 随机指标
        StochResult stoch = Indicators.stoch(high, low, close, 9, 3, 3);
        System.out.println("KDJ: K=" + stoch.slowK[stoch.slowK.length - 1] + 
                          ", D=" + stoch.slowD[stoch.slowD.length - 1]);
        
        // ADX - 趋势强度
        double[] adx = Indicators.adx(high, low, close, 14);
        System.out.println("ADX(14): " + adx[adx.length - 1]);
        
        // OBV - 成交量指标
        double[] obv = Indicators.obv(close, volume);
        System.out.println("OBV: " + obv[obv.length - 1]);
        
        // MFI - 资金流量指数
        double[] mfi = Indicators.mfi(high, low, close, volume, 14);
        System.out.println("MFI(14): " + mfi[mfi.length - 1]);
    }
    
    /**
     * K线形态识别示例
     */
    public static void candlestickPatterns() {
        System.out.println("\n=== K线形态识别示例 ===");
        
        int n = 50;
        double[] close = new double[n];
        double[] high = new double[n];
        double[] low = new double[n];
        double[] open = new double[n];
        
        // 生成模拟数据
        for (int i = 0; i < n; i++) {
            close[i] = 100 + i * 0.3 + Math.random() * 2 - 1;
            high[i] = close[i] + Math.random() * 2 + 0.5;
            low[i] = close[i] - Math.random() * 2 - 0.5;
            open[i] = close[i] + Math.random() * 2 - 1;
        }
        
        // 识别 K 线形态
        int[] doji = Indicators.cdlDoji(open, high, low, close);
        int[] hammer = Indicators.cdlHammer(open, high, low, close);
        int[] engulfing = Indicators.cdlEngulfing(open, high, low, close);
        int[] morningStar = Indicators.cdlMorningstar(open, high, low, close);
        int[] eveningStar = Indicators.cdlEveningstar(open, high, low, close);
        
        // 统计形态数量
        System.out.println("十字星数量: " + countNonZero(doji));
        System.out.println("锤子线数量: " + countNonZero(hammer));
        System.out.println("吞没形态数量: " + countNonZero(engulfing));
        System.out.println("晨星数量: " + countNonZero(morningStar));
        System.out.println("晚星数量: " + countNonZero(eveningStar));
        
        // 显示最近的形态
        for (int i = n - 5; i < n; i++) {
            StringBuilder patterns = new StringBuilder();
            if (doji[i] != 0) patterns.append("十字星(" + doji[i] + ") ");
            if (hammer[i] != 0) patterns.append("锤子线(" + hammer[i] + ") ");
            if (engulfing[i] != 0) patterns.append("吞没(" + engulfing[i] + ") ");
            if (patterns.length() > 0) {
                System.out.println("第 " + i + " 根K线: " + patterns.toString());
            }
        }
    }
    
    /**
     * 交易信号生成示例
     */
    public static void tradingSignals() {
        System.out.println("\n=== 交易信号示例 ===");
        
        int n = 100;
        double[] close = new double[n];
        double[] high = new double[n];
        double[] low = new double[n];
        
        // 生成模拟数据
        for (int i = 0; i < n; i++) {
            close[i] = 100 + i * 0.5 + Math.random() * 2 - 1;
            high[i] = close[i] + Math.random() * 2 + 0.5;
            low[i] = close[i] - Math.random() * 2 - 0.5;
        }
        
        // 计算多个指标
        double[] sma20 = Indicators.sma(close, 20);
        double[] sma50 = Indicators.sma(close, 50);
        double[] rsi = Indicators.rsi(close, 14);
        MacdResult macd = Indicators.macd(close);
        
        // 生成交易信号
        int signalCount = 0;
        for (int i = 50; i < n; i++) {
            boolean buySignal = sma20[i] > sma50[i] &&  // 趋势向上
                               rsi[i] < 30 &&           // RSI 超卖
                               macd.histogram[i] > 0;   // MACD 金叉
            
            boolean sellSignal = sma20[i] < sma50[i] &&  // 趋势向下
                                 rsi[i] > 70 &&          // RSI 超买
                                 macd.histogram[i] < 0;  // MACD 死叉
            
            if (buySignal && signalCount < 5) {
                System.out.println("  BUY @ 第" + i + "根K线, 价格=" + close[i]);
                signalCount++;
            } else if (sellSignal && signalCount < 5) {
                System.out.println("  SELL @ 第" + i + "根K线, 价格=" + close[i]);
                signalCount++;
            }
        }
        
        System.out.println("生成信号数量: " + signalCount);
    }
    
    /**
     * 完整交易分析示例
     */
    public static void completeAnalysis() {
        System.out.println("\n=== 完整交易分析示例 ===");
        
        int n = 200;
        double[] close = new double[n];
        double[] high = new double[n];
        double[] low = new double[n];
        double[] open = new double[n];
        double[] volume = new double[n];
        
        // 生成模拟数据
        for (int i = 0; i < n; i++) {
            close[i] = 100 + i * 0.3 + Math.sin(i / 20.0) * 5 + Math.random() * 2;
            high[i] = close[i] + Math.random() * 2 + 0.5;
            low[i] = close[i] - Math.random() * 2 - 0.5;
            open[i] = close[i] + Math.random() * 2 - 1;
            volume[i] = Math.random() * 5000 + 1000;
        }
        
        // 计算所有指标
        double[] sma20 = Indicators.sma(close, 20);
        double[] sma50 = Indicators.sma(close, 50);
        double[] rsi = Indicators.rsi(close, 14);
        MacdResult macd = Indicators.macd(close);
        double[] atr = Indicators.atr(high, low, close, 14);
        double[] obv = Indicators.obv(close, volume);
        
        // 分析最近的数据
        int lastIdx = n - 1;
        System.out.println("\n最后一天分析:");
        System.out.println("  收盘价: " + close[lastIdx]);
        System.out.println("  SMA20: " + sma20[lastIdx] + ", SMA50: " + sma50[lastIdx]);
        System.out.println("  趋势: " + (sma20[lastIdx] > sma50[lastIdx] ? "向上" : "向下"));
        System.out.println("  RSI: " + rsi[lastIdx] + 
                          " (" + (rsi[lastIdx] > 70 ? "超买" : rsi[lastIdx] < 30 ? "超卖" : "中性") + ")");
        System.out.println("  MACD: " + macd.macd[lastIdx] + ", Signal: " + macd.signal[lastIdx]);
        System.out.println("  MACD信号: " + (macd.histogram[lastIdx] > 0 ? "金叉" : "死叉"));
        System.out.println("  ATR: " + atr[lastIdx] + " (波动率)");
        System.out.println("  OBV: " + obv[lastIdx] + " (资金流向)");
        
        // 综合建议
        String trend = sma20[lastIdx] > sma50[lastIdx] ? "UP" : "DOWN";
        String momentum = rsi[lastIdx] < 30 ? "OVERSOLD" : rsi[lastIdx] > 70 ? "OVERBOUGHT" : "NEUTRAL";
        String macdSignal = macd.histogram[lastIdx] > 0 ? "BULLISH" : "BEARISH";
        
        System.out.println("\n综合分析:");
        System.out.println("  趋势: " + trend);
        System.out.println("  动量: " + momentum);
        System.out.println("  MACD: " + macdSignal);
        
        if (trend.equals("UP") && momentum.equals("OVERSOLD") && macdSignal.equals("BULLISH")) {
            System.out.println("  建议: 买入机会");
        } else if (trend.equals("DOWN") && momentum.equals("OVERBOUGHT") && macdSignal.equals("BEARISH")) {
            System.out.println("  建议: 卖出机会");
        } else {
            System.out.println("  建议: 观望");
        }
    }
    
    /**
     * 统计非零元素数量
     */
    private static int countNonZero(int[] arr) {
        int count = 0;
        for (int val : arr) {
            if (val != 0) count++;
        }
        return count;
    }
}