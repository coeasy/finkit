# finkit: 高性能Python技术分析库

> **Tier1** · 成熟度: `stable` · [绑定分级说明](../../docs/BINDING_TIERS.md)

| 能力 | 指标计算 | 流式 | 公式引擎 | ML 特征 | 可视化 |
|------|----------|------|----------|---------|--------|
| 状态 | ✅ 完整 | ✅ | ✅ | ✅ | ✅ |

<p align="center">
  <strong>由Rust驱动，为量化交易而生</strong><br>
  比纯Python实现快10-100倍
</p>

---

## 特性

- **极致性能**: 底层Rust实现，零开销抽象
- **80+技术指标**: 涵盖重叠研究、动量、周期、波动率、成交量、价格变换、统计等
- **60+K线形态识别**: 完整的蜡烛图形态检测
- **15+图表形态识别**: 头肩顶/底、双顶/底、三角形等
- **零GIL竞争**: 完全释放Python GIL，支持真正并行计算
- **跨平台**: Windows/macOS/Linux
- **Python 3.8+**: 广泛兼容

## 安装

```bash
pip install finkit
```

### 从源码编译

```bash
# 前置条件: Rust (rustup.rs)
git clone https://github.com/coeasy/finkit.git
cd finkit/ffi/python-binding
pip install maturin
maturin develop --release
```

## 快速入门

```python
import finkit as ta
import numpy as np

# 示例数据
close = [100.0, 101.5, 99.8, 102.3, 103.1, 101.0, 100.5, 102.8, 104.2, 103.5,
         105.0, 106.2, 104.8, 107.3, 108.1, 106.5, 105.8, 107.9, 109.2, 108.5]

# RSI
rsi = ta.rsi(close, timeperiod=14)
print(f"RSI: {rsi[-1]:.2f}")

# MACD
macd, signal, hist = ta.macd(close)
print(f"MACD: {macd[-1]:.4f}, Signal: {signal[-1]:.4f}")

# 布林带
upper, middle, lower = ta.bollinger_bands(close, timeperiod=5)
print(f"BB: {upper[-1]:.2f} / {middle[-1]:.2f} / {lower[-1]:.2f}")
```

## 完整指标列表

### 重叠研究 (Overlap Studies)

| 函数 | 描述 | 参数 |
|------|------|------|
| `sma(close, timeperiod=14)` | 简单移动平均 | close: array |
| `ema(close, timeperiod=14)` | 指数移动平均 | close: array |
| `wma(close, timeperiod=14)` | 加权移动平均 | close: array |
| `dema(close, timeperiod=14)` | 双重指数移动平均 | close: array |
| `tema(close, timeperiod=14)` | 三重指数移动平均 | close: array |
| `kama(close, timeperiod=10, fastperiod=2, slowperiod=30)` | 考夫曼自适应移动平均 | close: array |
| `mama(close, fastlimit=0.5, slowlimit=0.05)` | MESA自适应移动平均 | close: array → (mama, fama) |
| `t3(close, timeperiod=5, vfactor=0.7)` | T3移动平均 | close: array |
| `bollinger_bands(close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0)` | 布林带 | close: array → (upper, middle, lower) |
| `midpoint(close, timeperiod=14)` | 中点 | close: array |
| `midprice(high, low, timeperiod=14)` | 中间价 | high, low: arrays |
| `sar(high, low, close, acceleration=0.02, maximum=0.2)` | 抛物线转向指标 | high, low, close → (sar, af) |

### 动量指标 (Momentum Indicators)

| 函数 | 描述 | 参数 |
|------|------|------|
| `rsi(close, timeperiod=14)` | 相对强弱指数 | close: array |
| `macd(close, fastperiod=12, slowperiod=26, signalperiod=9)` | MACD | close → (macd, signal, hist) |
| `stoch(high, low, close, fastk_period=5, slowk_period=3, slowd_period=3)` | 随机指标 | high, low, close → (k, d) |
| `adx(high, low, close, timeperiod=14)` | 平均趋向指数 | high, low, close: arrays |
| `aroon(high, low, timeperiod=14)` | 阿隆指标 | high, low → (aroon_up, aroon_down) |
| `cci(high, low, close, timeperiod=14)` | 商品通道指数 | high, low, close: arrays |
| `mom(close, timeperiod=10)` | 动量 | close: array |
| `roc(close, timeperiod=10)` | 变化率 | close: array |
| `willr(high, low, close, timeperiod=14)` | 威廉指标 | high, low, close: arrays |
| `apo(close, fastperiod=12, slowperiod=26)` | 绝对价格振荡器 | close: array |
| `bop(open, high, low, close)` | 均势 | open, high, low, close: arrays |
| `cmo(close, timeperiod=14)` | 钱德动量振荡器 | close: array |
| `dx(high, low, close, timeperiod=14)` | 动向指数 | high, low, close: arrays |
| `mfi(high, low, close, volume, timeperiod=14)` | 资金流量指数 | high, low, close, volume: arrays |
| `minus_di(high, low, close, timeperiod=14)` | 负方向指标 | high, low, close: arrays |
| `minus_dm(high, low)` | 负方向运动 | high, low: arrays |
| `plus_di(high, low, close, timeperiod=14)` | 正方向指标 | high, low, close: arrays |
| `plus_dm(high, low)` | 正方向运动 | high, low: arrays |
| `trix(close, timeperiod=14)` | 三重指数平滑平均 | close: array |

### 周期指标 (Cycle Indicators - Hilbert Transform)

| 函数 | 描述 | 参数 |
|------|------|------|
| `ht_dcperiod(close)` | 希尔伯特变换 - 主周期 | close: array |
| `ht_dcphase(close)` | 希尔伯特变换 - 主相位 | close: array |
| `ht_phasor(close)` | 希尔伯特变换 - 相量 | close → (in_phase, quadrature) |
| `ht_sine(close)` | 希尔伯特变换 - 正弦波 | close → (sine, lead_sine) |
| `ht_trendmode(close)` | 希尔伯特变换 - 趋势/周期模式 | close: array |
| `ht_trendline(close)` | 希尔伯特变换 - 瞬时趋势线 | close: array |

### 成交量指标 (Volume Indicators)

| 函数 | 描述 | 参数 |
|------|------|------|
| `obv(close, volume)` | 能量潮 | close, volume: arrays |
| `ad(high, low, close, volume)` | 累积/分布线 | high, low, close, volume: arrays |
| `adosc(high, low, close, volume, fastperiod=3, slowperiod=10)` | 蔡金A/D振荡器 | high, low, close, volume: arrays |

### 波动率指标 (Volatility Indicators)

| 函数 | 描述 | 参数 |
|------|------|------|
| `atr(high, low, close, timeperiod=14)` | 平均真实波幅 | high, low, close: arrays |
| `natr(high, low, close, timeperiod=14)` | 标准化平均真实波幅 | high, low, close: arrays |
| `trange(high, low, close)` | 真实波幅 | high, low, close: arrays |

### 价格变换 (Price Transforms)

| 函数 | 描述 | 参数 |
|------|------|------|
| `avgprice(open, high, low, close)` | 平均价格 | open, high, low, close: arrays |
| `medprice(high, low)` | 中间价格 | high, low: arrays |
| `typprice(high, low, close)` | 典型价格 | high, low, close: arrays |
| `wclprice(high, low, close)` | 加权收盘价 | high, low, close: arrays |

### 统计函数 (Statistics Functions)

| 函数 | 描述 | 参数 |
|------|------|------|
| `zscore(close, timeperiod=14)` | Z分数/标准化 | close: array |
| `percent_rank(close, timeperiod=10)` | 百分比排名 | close: array |
| `beta(asset, benchmark, timeperiod=5)` | Beta系数 | asset, benchmark: arrays |
| `correlation(input_a, input_b, timeperiod=14)` | 皮尔逊相关系数 | input_a, input_b: arrays |
| `std_dev(close, timeperiod=5, nbdev=1.0)` | 标准差 | close: array |
| `linear_reg(close, timeperiod=14)` | 线性回归 | close: array |
| `tsf(close, timeperiod=14)` | 时间序列预测 | close: array |

### K线形态识别 (Candlestick Patterns)

所有K线形态函数返回 `array[i]` 值为：
- `100`: 看涨形态
- `-100`: 看跌形态
- `0`: 无形态

```python
# 基础用法 (需要 OHLC 数据)
ta.cdl_doji(open, high, low, close, doji_pct=0.1)  # 十字星
ta.cdl_hammer(open, high, low, close)               # 锤子线
ta.cdl_engulfing(open, high, low, close)            # 吞没形态
ta.cdl_morning_star(open, high, low, close)         # 晨星
# ... 60+ 种形态
```

支持的形态：Doji, Dragonfly Doji, Gravestone Doji, Long-Legged Doji, 4 Price Doji, Hammer, Inverted Hammer, Hanging Man, Shooting Star, Engulfing, Harami, Harami Cross, Morning Star, Evening Star, Morning Doji Star, Evening Doji Star, Marubozu, Three White Soldiers, Three Black Crows, Three Inside Up, Three Outside Up, Three Inside Down, Three Outside Down, Piercing, Dark Cloud Cover, Belt Hold, Spinning Top, High Wave, Rickshaw Man, Short Line, Long Line, Kicking, 等。

### 图表形态识别 (Chart Patterns)

返回检测到的形态位置索引。

```python
# 示例：检测头肩顶
indices = ta.detect_head_shoulders(high, min_bars=5, head_ratio=1.1)
print(f"Head & Shoulders detected at: {indices}")

# 其他形态
ta.detect_double_top(high, lookback=20, tolerance=0.03)
ta.detect_double_bottom(low, lookback=20, tolerance=0.03)
ta.detect_ascending_triangle(high, low, lookback=20, tolerance=0.05)
ta.detect_falling_wedge(high, low, lookback=20)
ta.detect_flag(high, low, close, flagpole_period=10, flag_period=5)
# ... 更多形态
```

## NumPy/Pandas集成

```python
import numpy as np
import pandas as pd
import finkit as ta

# NumPy数组输入
close_np = np.array([100.0, 101.5, 99.8, ...])
rsi = ta.rsi(close_np.tolist())
rsi_np = np.array(rsi)

# Pandas DataFrame
df = pd.DataFrame({
    'open': [...],
    'high': [...],
    'low': [...],
    'close': [...],
    'volume': [...]
})

df['RSI'] = ta.rsi(df['close'].tolist())
df['MACD'], df['Signal'], df['Hist'] = ta.macd(df['close'].tolist())
df['Upper'], df['Middle'], df['Lower'] = ta.bollinger_bands(df['close'].tolist())
```

## 性能对比

```
指标计算 (100,000条数据, RSI):
- pandas-ta:     45.2 ms
- ta-lib (C):    12.8 ms
- AlphaTA:    3.2 ms  ✅ 比pandas-ta快14倍
```

## 开发

```bash
# 安装开发依赖
pip install maturin pytest numpy pandas

# 本地构建
maturin develop

# 运行测试
python -m pytest tests/

# 构建wheel
maturin build --release

# 发布到PyPI
maturin publish
```

## 许可证

Apache-2.0

## 贡献

欢迎提交Issue和PR！
