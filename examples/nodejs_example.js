/**
 * alpha_ta Node.js 示例代码
 * 展示如何使用 alpha_ta 进行技术分析
 */

const ta = require('alpha_ta-node');

// ============================================
// 基础指标计算示例
// ============================================
function basicIndicators() {
    console.log('=== 基础指标示例 ===');
    
    const close = [44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 
                   46.08, 46.32, 46.56, 46.80, 47.04, 47.28, 47.52];
    
    // 移动平均
    const sma5 = ta.sma(close, 5);
    const sma10 = ta.sma(close, 10);
    const ema5 = ta.ema(close, 5);
    
    console.log(`SMA(5): ${sma5.slice(-3).map(v => v?.toFixed(2) || 'NaN').join(', ')}`);
    console.log(`SMA(10): ${sma10.slice(-3).map(v => v?.toFixed(2) || 'NaN').join(', ')}`);
    console.log(`EMA(5): ${ema5.slice(-3).map(v => v?.toFixed(2) || 'NaN').join(', ')}`);
    
    // RSI
    const rsi = ta.rsi(close, 14);
    console.log(`RSI(14): ${rsi[rsi.length - 1]?.toFixed(2) || 'NaN'}`);
    
    // MACD
    const [macd, signal, hist] = ta.macd(close, 12, 26, 9);
    console.log(`MACD: ${macd[macd.length - 1]?.toFixed(4) || 'NaN'}, Signal: ${signal[signal.length - 1]?.toFixed(4) || 'NaN'}, Hist: ${hist[hist.length - 1]?.toFixed(4) || 'NaN'}`);
    
    // 布林带
    const [upper, middle, lower] = ta.bollinger_bands(close, 5, 2.0, 2.0);
    console.log(`布林带: Upper=${upper[upper.length - 1]?.toFixed(2) || 'NaN'}, Middle=${middle[middle.length - 1]?.toFixed(2) || 'NaN'}, Lower=${lower[lower.length - 1]?.toFixed(2) || 'NaN'}`);
}

// ============================================
// OHLCV 数据分析示例
// ============================================
function ohlcvAnalysis() {
    console.log('\n=== OHLCV 分析示例 ===');
    
    // 创建 OHLCV 数据
    const n = 100;
    const close = Array.from({ length: n }, (_, i) => 100 + i * 0.5 + Math.random() * 2 - 1);
    const high = close.map(x => x + Math.random() * 2 + 0.5);
    const low = close.map(x => x - Math.random() * 2 - 0.5);
    const open = close.map(x => x + Math.random() * 2 - 1);
    const volume = Array.from({ length: n }, () => Math.random() * 4000 + 1000);
    
    // ATR - 波动率
    const atr = ta.atr(high, low, close, 14);
    console.log(`ATR(14): ${atr[atr.length - 1]?.toFixed(4) || 'NaN'}`);
    
    // KDJ - 随机指标
    const [slowk, slowd] = ta.stoch(high, low, close, 9, 3, 3);
    console.log(`KDJ: K=${slowk[slowk.length - 1]?.toFixed(2) || 'NaN'}, D=${slowd[slowd.length - 1]?.toFixed(2) || 'NaN'}`);
    
    // ADX - 趋势强度
    const adx = ta.adx(high, low, close, 14);
    console.log(`ADX(14): ${adx[adx.length - 1]?.toFixed(2) || 'NaN'}`);
    
    // OBV - 成交量指标
    const obv = ta.obv(close, volume);
    console.log(`OBV: ${obv[obv.length - 1]?.toFixed(2) || 'NaN'}`);
    
    // MFI - 资金流量指数
    const mfi = ta.mfi(high, low, close, volume, 14);
    console.log(`MFI(14): ${mfi[mfi.length - 1]?.toFixed(2) || 'NaN'}`);
}

// ============================================
// K线形态识别示例
// ============================================
function candlestickPatterns() {
    console.log('\n=== K线形态识别示例 ===');
    
    // 创建 OHLC 数据
    const n = 50;
    const close = Array.from({ length: n }, (_, i) => 100 + i * 0.3 + Math.random() * 2 - 1);
    const high = close.map(x => x + Math.random() * 2 + 0.5);
    const low = close.map(x => x - Math.random() * 2 - 0.5);
    const open = close.map(x => x + Math.random() * 2 - 1);
    
    // 识别 K 线形态
    const doji = ta.cdl_doji(open, high, low, close);
    const hammer = ta.cdl_hammer(open, high, low, close);
    const engulfing = ta.cdl_engulfing(open, high, low, close);
    const morningStar = ta.cdl_morningstar(open, high, low, close);
    const eveningStar = ta.cdl_eveningstar(open, high, low, close);
    
    // 统计形态数量
    console.log(`十字星数量: ${doji.filter(x => x !== 0).length}`);
    console.log(`锤子线数量: ${hammer.filter(x => x !== 0).length}`);
    console.log(`吞没形态数量: ${engulfing.filter(x => x !== 0).length}`);
    console.log(`晨星数量: ${morningStar.filter(x => x !== 0).length}`);
    console.log(`晚星数量: ${eveningStar.filter(x => x !== 0).length}`);
    
    // 显示最近的形态
    for (let i = n - 5; i < n; i++) {
        const patterns = [];
        if (doji[i] !== 0) patterns.push(`十字星(${doji[i]})`);
        if (hammer[i] !== 0) patterns.push(`锤子线(${hammer[i]})`);
        if (engulfing[i] !== 0) patterns.push(`吞没(${engulfing[i]})`);
        if (patterns.length > 0) {
            console.log(`第 ${i} 根K线: ${patterns.join(', ')}`);
        }
    }
}

// ============================================
// 交易信号生成示例
// ============================================
function tradingSignals() {
    console.log('\n=== 交易信号示例 ===');
    
    // 创建模拟数据
    const n = 100;
    const close = Array.from({ length: n }, (_, i) => 100 + i * 0.5 + Math.random() * 2 - 1);
    const high = close.map(x => x + Math.random() * 2 + 0.5);
    const low = close.map(x => x - Math.random() * 2 - 0.5);
    
    // 计算多个指标
    const sma20 = ta.sma(close, 20);
    const sma50 = ta.sma(close, 50);
    const rsi = ta.rsi(close, 14);
    const [macd, signal, hist] = ta.macd(close);
    
    // 生成交易信号
    const signals = [];
    for (let i = 50; i < n; i++) {
        const buySignal = 
            sma20[i] > sma50[i] &&  // 趋势向上
            rsi[i] < 30 &&          // RSI 超卖
            hist[i] > 0;            // MACD 金叉
        
        const sellSignal = 
            sma20[i] < sma50[i] &&  // 趋势向下
            rsi[i] > 70 &&          // RSI 超买
            hist[i] < 0;            // MACD 死叉
        
        if (buySignal) {
            signals.push(['BUY', i, close[i]]);
        } else if (sellSignal) {
            signals.push(['SELL', i, close[i]]);
        }
    }
    
    console.log(`生成信号数量: ${signals.length}`);
    signals.slice(0, 5).forEach(([type, idx, price]) => {
        console.log(`  ${type} @ 第${idx}根K线, 价格=${price.toFixed(2)}`);
    });
}

// ============================================
// 完整交易分析示例
// ============================================
function completeAnalysis() {
    console.log('\n=== 完整交易分析示例 ===');
    
    // 创建完整的 OHLCV 数据
    const n = 200;
    const dates = Array.from({ length: n }, (_, i) => {
        const d = new Date('2024-01-01');
        d.setDate(d.getDate() + i);
        return d.toISOString().split('T')[0];
    });
    
    const close = Array.from({ length: n }, (_, i) => 100 + i * 0.3 + Math.sin(i / 20) * 5 + Math.random() * 2);
    const high = close.map(x => x + Math.random() * 2 + 0.5);
    const low = close.map(x => x - Math.random() * 2 - 0.5);
    const open = close.map(x => x + Math.random() * 2 - 1);
    const volume = Array.from({ length: n }, () => Math.random() * 5000 + 1000);
    
    // 计算所有指标
    const sma20 = ta.sma(close, 20);
    const sma50 = ta.sma(close, 50);
    const ema12 = ta.ema(close, 12);
    const ema26 = ta.ema(close, 26);
    const rsi = ta.rsi(close, 14);
    const [macd, macdSignal, macdHist] = ta.macd(close);
    const atr = ta.atr(high, low, close, 14);
    const obv = ta.obv(close, volume);
    
    // 分析最近的数据
    const lastIdx = n - 1;
    console.log(`\n最后一天 (${dates[lastIdx]}) 分析:`);
    console.log(`  收盘价: ${close[lastIdx].toFixed(2)}`);
    console.log(`  SMA20: ${sma20[lastIdx]?.toFixed(2) || 'NaN'}, SMA50: ${sma50[lastIdx]?.toFixed(2) || 'NaN'}`);
    console.log(`  趋势: ${sma20[lastIdx] > sma50[lastIdx] ? '向上' : '向下'}`);
    console.log(`  RSI: ${rsi[lastIdx]?.toFixed(2) || 'NaN'} (${rsi[lastIdx] > 70 ? '超买' : rsi[lastIdx] < 30 ? '超卖' : '中性'})`);
    console.log(`  MACD: ${macd[lastIdx]?.toFixed(4) || 'NaN'}, Signal: ${macdSignal[lastIdx]?.toFixed(4) || 'NaN'}`);
    console.log(`  MACD信号: ${macdHist[lastIdx] > 0 ? '金叉' : '死叉'}`);
    console.log(`  ATR: ${atr[lastIdx]?.toFixed(4) || 'NaN'} (波动率)`);
    console.log(`  OBV: ${obv[lastIdx]?.toFixed(2) || 'NaN'} (资金流向)`);
    
    // 综合建议
    const trend = sma20[lastIdx] > sma50[lastIdx] ? 'UP' : 'DOWN';
    const momentum = rsi[lastIdx] < 30 ? 'OVERSOLD' : rsi[lastIdx] > 70 ? 'OVERBOUGHT' : 'NEUTRAL';
    const macdSignalType = macdHist[lastIdx] > 0 ? 'BULLISH' : 'BEARISH';
    
    console.log(`\n综合分析:`);
    console.log(`  趋势: ${trend}`);
    console.log(`  动量: ${momentum}`);
    console.log(`  MACD: ${macdSignalType}`);
    
    if (trend === 'UP' && momentum === 'OVERSOLD' && macdSignalType === 'BULLISH') {
        console.log(`  建议: 买入机会`);
    } else if (trend === 'DOWN' && momentum === 'OVERBOUGHT' && macdSignalType === 'BEARISH') {
        console.log(`  建议: 卖出机会`);
    } else {
        console.log(`  建议: 观望`);
    }
}

// ============================================
// 主函数
// ============================================
function main() {
    console.log('alpha_ta Node.js 示例代码');
    console.log('=' .repeat(50));
    
    basicIndicators();
    ohlcvAnalysis();
    candlestickPatterns();
    tradingSignals();
    completeAnalysis();
    
    console.log('\n' + '='.repeat(50));
    console.log('示例完成！');
}

// 运行示例
main();