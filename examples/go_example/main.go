package main

import (
	"fmt"
	"math"
	"math/rand"

	"github.com/alpha_ta-rs/alpha_ta"
)

func main() {
	fmt.Println("alpha_ta Go 示例代码")
	fmt.Println("==================================================")

	basicIndicators()
	ohlcvAnalysis()
	candlestickPatterns()
	tradingSignals()
	completeAnalysis()

	fmt.Println("\n==================================================")
	fmt.Println("示例完成！")
}

// ============================================
// 基础指标计算示例
// ============================================
func basicIndicators() {
	fmt.Println("\n=== 基础指标示例 ===")

	close := []float64{44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84,
		46.08, 46.32, 46.56, 46.80, 47.04, 47.28, 47.52}

	// 移动平均
	sma5, _ := alpha_ta.SMA(close, 5)
	sma10, _ := alpha_ta.SMA(close, 10)
	ema5, _ := alpha_ta.EMA(close, 5)

	fmt.Printf("SMA(5): %.2f, %.2f, %.2f\n", sma5[len(sma5)-3], sma5[len(sma5)-2], sma5[len(sma5)-1])
	fmt.Printf("SMA(10): %.2f, %.2f, %.2f\n", sma10[len(sma10)-3], sma10[len(sma10)-2], sma10[len(sma10)-1])
	fmt.Printf("EMA(5): %.2f, %.2f, %.2f\n", ema5[len(ema5)-3], ema5[len(ema5)-2], ema5[len(ema5)-1])

	// RSI
	rsi, _ := alpha_ta.RSI(close, 14)
	fmt.Printf("RSI(14): %.2f\n", rsi[len(rsi)-1])

	// MACD
	macd, signal, hist, _ := alpha_ta.MACD(close, 12, 26, 9)
	fmt.Printf("MACD: %.4f, Signal: %.4f, Hist: %.4f\n",
		macd[len(macd)-1], signal[len(signal)-1], hist[len(hist)-1])

	// 布林带
	upper, middle, lower, _ := alpha_ta.BBands(close, 5, 2.0, 2.0)
	fmt.Printf("布林带: Upper=%.2f, Middle=%.2f, Lower=%.2f\n",
		upper[len(upper)-1], middle[len(middle)-1], lower[len(lower)-1])
}

// ============================================
// OHLCV 数据分析示例
// ============================================
func ohlcvAnalysis() {
	fmt.Println("\n=== OHLCV 分析示例 ===")

	n := 100
	close := make([]float64, n)
	high := make([]float64, n)
	low := make([]float64, n)
	open := make([]float64, n)
	volume := make([]float64, n)

	// 生成模拟数据
	for i := 0; i < n; i++ {
		close[i] = 100 + float64(i)*0.5 + rand.Float64()*2 - 1
		high[i] = close[i] + rand.Float64()*2 + 0.5
		low[i] = close[i] - rand.Float64()*2 - 0.5
		open[i] = close[i] + rand.Float64()*2 - 1
		volume[i] = rand.Float64()*4000 + 1000
	}

	// ATR - 波动率
	atr, _ := alpha_ta.ATR(high, low, close, 14)
	fmt.Printf("ATR(14): %.4f\n", atr[len(atr)-1])

	// KDJ - 随机指标
	slowk, slowd, _ := alpha_ta.Stoch(high, low, close, 9, 3, 3)
	fmt.Printf("KDJ: K=%.2f, D=%.2f\n", slowk[len(slowk)-1], slowd[len(slowd)-1])

	// ADX - 趋势强度
	adx, _ := alpha_ta.ADX(high, low, close, 14)
	fmt.Printf("ADX(14): %.2f\n", adx[len(adx)-1])

	// OBV - 成交量指标
	obv, _ := alpha_ta.OBV(close, volume)
	fmt.Printf("OBV: %.2f\n", obv[len(obv)-1])

	// MFI - 资金流量指数
	mfi, _ := alpha_ta.MFI(high, low, close, volume, 14)
	fmt.Printf("MFI(14): %.2f\n", mfi[len(mfi)-1])
}

// ============================================
// K线形态识别示例
// ============================================
func candlestickPatterns() {
	fmt.Println("\n=== K线形态识别示例 ===")

	n := 50
	close := make([]float64, n)
	high := make([]float64, n)
	low := make([]float64, n)
	open := make([]float64, n)

	// 生成模拟数据
	for i := 0; i < n; i++ {
		close[i] = 100 + float64(i)*0.3 + rand.Float64()*2 - 1
		high[i] = close[i] + rand.Float64()*2 + 0.5
		low[i] = close[i] - rand.Float64()*2 - 0.5
		open[i] = close[i] + rand.Float64()*2 - 1
	}

	// 识别 K 线形态
	doji, _ := alpha_ta.CDLDoji(open, high, low, close)
	hammer, _ := alpha_ta.CDLHammer(open, high, low, close)
	engulfing, _ := alpha_ta.CDLEngulfing(open, high, low, close)
	morningStar, _ := alpha_ta.CDLMorningstar(open, high, low, close)
	eveningStar, _ := alpha_ta.CDLEveningstar(open, high, low, close)

	// 统计形态数量
	fmt.Printf("十字星数量: %d\n", countNonZero(doji))
	fmt.Printf("锤子线数量: %d\n", countNonZero(hammer))
	fmt.Printf("吞没形态数量: %d\n", countNonZero(engulfing))
	fmt.Printf("晨星数量: %d\n", countNonZero(morningStar))
	fmt.Printf("晚星数量: %d\n", countNonZero(eveningStar))

	// 显示最近的形态
	for i := n - 5; i < n; i++ {
		var patterns []string
		if doji[i] != 0 {
			patterns = append(patterns, fmt.Sprintf("十字星(%d)", doji[i]))
		}
		if hammer[i] != 0 {
			patterns = append(patterns, fmt.Sprintf("锤子线(%d)", hammer[i]))
		}
		if engulfing[i] != 0 {
			patterns = append(patterns, fmt.Sprintf("吞没(%d)", engulfing[i]))
		}
		if len(patterns) > 0 {
			fmt.Printf("第 %d 根K线: %s\n", i, patterns)
		}
	}
}

// ============================================
// 交易信号生成示例
// ============================================
func tradingSignals() {
	fmt.Println("\n=== 交易信号示例 ===")

	n := 100
	close := make([]float64, n)
	high := make([]float64, n)
	low := make([]float64, n)

	// 生成模拟数据
	for i := 0; i < n; i++ {
		close[i] = 100 + float64(i)*0.5 + rand.Float64()*2 - 1
		high[i] = close[i] + rand.Float64()*2 + 0.5
		low[i] = close[i] - rand.Float64()*2 - 0.5
	}

	// 计算多个指标
	sma20, _ := alpha_ta.SMA(close, 20)
	sma50, _ := alpha_ta.SMA(close, 50)
	rsi, _ := alpha_ta.RSI(close, 14)
	macd, signal, hist, _ := alpha_ta.MACD(close, 12, 26, 9)

	// 生成交易信号
	signalCount := 0
	for i := 50; i < n; i++ {
		buySignal := sma20[i] > sma50[i] && // 趋势向上
			rsi[i] < 30 && // RSI 超卖
			hist[i] > 0 // MACD 金叉

		sellSignal := sma20[i] < sma50[i] && // 趋势向下
			rsi[i] > 70 && // RSI 超买
			hist[i] < 0 // MACD 死叉

		if buySignal && signalCount < 5 {
			fmt.Printf("  BUY @ 第%d根K线, 价格=%.2f\n", i, close[i])
			signalCount++
		} else if sellSignal && signalCount < 5 {
			fmt.Printf("  SELL @ 第%d根K线, 价格=%.2f\n", i, close[i])
			signalCount++
		}
	}

	fmt.Printf("生成信号数量: %d\n", signalCount)
}

// ============================================
// 完整交易分析示例
// ============================================
func completeAnalysis() {
	fmt.Println("\n=== 完整交易分析示例 ===")

	n := 200
	close := make([]float64, n)
	high := make([]float64, n)
	low := make([]float64, n)
	open := make([]float64, n)
	volume := make([]float64, n)

	// 生成模拟数据
	for i := 0; i < n; i++ {
		close[i] = 100 + float64(i)*0.3 + math.Sin(float64(i)/20.0)*5 + rand.Float64()*2
		high[i] = close[i] + rand.Float64()*2 + 0.5
		low[i] = close[i] - rand.Float64()*2 - 0.5
		open[i] = close[i] + rand.Float64()*2 - 1
		volume[i] = rand.Float64()*5000 + 1000
	}

	// 计算所有指标
	sma20, _ := alpha_ta.SMA(close, 20)
	sma50, _ := alpha_ta.SMA(close, 50)
	rsi, _ := alpha_ta.RSI(close, 14)
	macd, signal, hist, _ := alpha_ta.MACD(close, 12, 26, 9)
	atr, _ := alpha_ta.ATR(high, low, close, 14)
	obv, _ := alpha_ta.OBV(close, volume)

	// 分析最近的数据
	lastIdx := n - 1
	fmt.Println("\n最后一天分析:")
	fmt.Printf("  收盘价: %.2f\n", close[lastIdx])
	fmt.Printf("  SMA20: %.2f, SMA50: %.2f\n", sma20[lastIdx], sma50[lastIdx])
	fmt.Printf("  趋势: %s\n", trendDirection(sma20[lastIdx], sma50[lastIdx]))
	fmt.Printf("  RSI: %.2f (%s)\n", rsi[lastIdx], rsiStatus(rsi[lastIdx]))
	fmt.Printf("  MACD: %.4f, Signal: %.4f\n", macd[lastIdx], signal[lastIdx])
	fmt.Printf("  MACD信号: %s\n", macdSignal(hist[lastIdx]))
	fmt.Printf("  ATR: %.4f (波动率)\n", atr[lastIdx])
	fmt.Printf("  OBV: %.2f (资金流向)\n", obv[lastIdx])

	// 综合建议
	trend := trendDirection(sma20[lastIdx], sma50[lastIdx])
	momentum := rsiStatus(rsi[lastIdx])
	macdSig := macdSignal(hist[lastIdx])

	fmt.Println("\n综合分析:")
	fmt.Printf("  趋势: %s\n", trend)
	fmt.Printf("  动量: %s\n", momentum)
	fmt.Printf("  MACD: %s\n", macdSig)

	if trend == "UP" && momentum == "OVERSOLD" && macdSig == "BULLISH" {
		fmt.Println("  建议: 买入机会")
	} else if trend == "DOWN" && momentum == "OVERBOUGHT" && macdSig == "BEARISH" {
		fmt.Println("  建议: 卖出机会")
	} else {
		fmt.Println("  建议: 观望")
	}
}

// ============================================
// 辅助函数
// ============================================
func countNonZero(arr []int) int {
	count := 0
	for _, val := range arr {
		if val != 0 {
			count++
		}
	}
	return count
}

func trendDirection(sma20, sma50 float64) string {
	if sma20 > sma50 {
		return "UP"
	}
	return "DOWN"
}

func rsiStatus(rsi float64) string {
	if rsi > 70 {
		return "OVERBOUGHT"
	} else if rsi < 30 {
		return "OVERSOLD"
	}
	return "NEUTRAL"
}

func macdSignal(hist float64) string {
	if hist > 0 {
		return "BULLISH"
	}
	return "BEARISH"
}