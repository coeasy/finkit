# Finkit API 参考文档

本文档详细列出所有可用的技术指标函数及其参数。

## 目录

1. [重叠研究指标](#重叠研究指标)
2. [动量指标](#动量指标)
3. [成交量指标](#成交量指标)
4. [波动率指标](#波动率指标)
5. [周期指标](#周期指标)
6. [价格变换](#价格变换)
7. [统计指标](#统计指标)
8. [K线形态识别](#k线形态识别)
9. [图表形态识别](#图表形态识别)
10. [经典形态指标](#经典形态指标)
11. [流式指标](#流式指标)
12. [公式引擎](#公式引擎)

---

## 重叠研究指标

### SMA - 简单移动平均

```python
sma(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

**返回:**
- 移动平均值数组 (前 `timeperiod-1` 个元素为 NaN)

**示例:**
```python
import finkit as ta
close = [44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84]
sma = ta.sma(close, 5)  # 5日均线
```

---

### EMA - 指数移动平均

```python
ema(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

**返回:**
- 指数移动平均值数组

---

### WMA - 加权移动平均

```python
wma(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

**返回:**
- 加权移动平均值数组

---

### DEMA - 双指数移动平均

```python
dema(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

---

### TEMA - 三指数移动平均

```python
tema(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

---

### KAMA - 自适应移动平均

```python
kama(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

---

### TRIMA - 三角移动平均

```python
trima(close, timeperiod=30)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 30)

---

### BBANDS - 布林带

```python
bbands(close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0, matype=0)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 5)
- `nbdevup`: 上轨偏差倍数 (默认 2.0)
- `nbdevdn`: 下轨偏差倍数 (默认 2.0)
- `matype`: 移动平均类型 (默认 0=SMA)

**返回:**
- `(upper, middle, lower)` 三个数组

**示例:**
```python
upper, middle, lower = ta.bbands(close, 20, 2.0, 2.0)
```

---

### MIDPOINT - 中间价

```python
midpoint(close, timeperiod=14)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 14)

**返回:**
- 周期内最高价和最低价的平均值

---

### MIDPRICE - 中间最高最低价

```python
midprice(high, low, timeperiod=14)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `timeperiod`: 周期 (默认 14)

---

### SAR - 抛物线 SAR

```python
sar(high, low, acceleration=0.02, maximum=0.2)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `acceleration`: 加速度因子 (默认 0.02)
- `maximum`: 最大加速度 (默认 0.2)

---

### MAVP - 移动平均可变周期

```python
mavp(close, periods, minperiod=2, maxperiod=30, matype=0)
```

**参数:**
- `close`: 收盘价数组
- `periods`: 周期数组
- `minperiod`: 最小周期 (默认 2)
- `maxperiod`: 最大周期 (默认 30)

---

## 动量指标

### RSI - 相对强弱指数

```python
rsi(close, timeperiod=14)
```

**参数:**
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 14)

**返回:**
- RSI 值数组 (范围 0-100)

**示例:**
```python
rsi = ta.rsi(close, 14)
# RSI > 70: 超买
# RSI < 30: 超卖
```

---

### MACD - 异同移动平均线

```python
macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
```

**参数:**
- `close`: 收盘价数组
- `fastperiod`: 快周期 (默认 12)
- `slowperiod`: 慢周期 (默认 26)
- `signalperiod`: 信号周期 (默认 9)

**返回:**
- `(macd, signal, histogram)` 三个数组

**示例:**
```python
macd, signal, hist = ta.macd(close, 12, 26, 9)
# MACD > Signal: 金叉 (看涨)
# MACD < Signal: 死叉 (看跌)
```

---

### STOCH - KDJ 指标

```python
stoch(high, low, close, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `fastk_period`: 快 K 周期 (默认 5)
- `slowk_period`: 慢 K 周期 (默认 3)
- `slowd_period`: 慢 D 周期 (默认 3)

**返回:**
- `(slowk, slowd)` 两个数组

---

### STOCHF - 快速 KDJ

```python
stochf(high, low, close, fastk_period=5, fastd_period=3, fastd_matype=0)
```

---

### STOCHRSI - RSI 的 KDJ

```python
stochrsi(close, timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0)
```

---

### ADX - 平均趋向指数

```python
adx(high, low, close, timeperiod=14)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 14)

**返回:**
- ADX 值数组 (范围 0-100)

**解读:**
- ADX > 25: 强趋势
- ADX < 20: 无趋势/震荡

---

### ADXR - 平均趋向指数评级

```python
adxr(high, low, close, timeperiod=14)
```

---

### APO - 绝对价格振荡器

```python
apo(close, fastperiod=12, slowperiod=26, matype=0)
```

---

### AROON - Aroon 指标

```python
aroon(high, low, timeperiod=14)
```

**返回:**
- `(aroon_down, aroon_up)` 两个数组

---

### AROONOSC - Aroon 振荡器

```python
aroonosc(high, low, timeperiod=14)
```

---

### BOP - 均衡点

```python
bop(open, high, low, close)
```

---

### CCI - 商品通道指数

```python
cci(high, low, close, timeperiod=14)
```

**解读:**
- CCI > 100: 超买
- CCI < -100: 超卖

---

### CMO - 钱德动量振荡器

```python
cmo(close, timeperiod=14)
```

---

### DX - 动向指数

```python
dx(high, low, close, timeperiod=14)
```

---

### MFI - 资金流量指数

```python
mfi(high, low, close, volume, timeperiod=14)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `volume`: 成交量数组
- `timeperiod`: 周期 (默认 14)

**解读:**
- MFI > 80: 超买
- MFI < 20: 超卖

---

### MINUS_DI - 负向动向指标

```python
minus_di(high, low, close, timeperiod=14)
```

---

### PLUS_DI - 正向动向指标

```python
plus_di(high, low, close, timeperiod=14)
```

---

### MINUS_DM - 负向动向

```python
minus_dm(high, low, timeperiod=14)
```

---

### PLUS_DM - 正向动向

```python
plus_dm(high, low, timeperiod=14)
```

---

### MOM - 动量

```python
mom(close, timeperiod=10)
```

---

### PPO - 价格振荡百分比

```python
ppo(close, fastperiod=12, slowperiod=26, matype=0)
```

---

### ROC - 变动率

```python
roc(close, timeperiod=10)
```

---

### ROCP - 变动率百分比

```python
rocp(close, timeperiod=10)
```

---

### ROCR - 变动率比率

```python
rocr(close, timeperiod=10)
```

---

### ROCR100 - 变动率比率 100

```python
rocr100(close, timeperiod=10)
```

---

### TRIX - 三重指数平滑平均线

```python
trix(close, timeperiod=30)
```

---

### ULTOSC - 终极振荡器

```python
ultosc(high, low, close, timeperiod1=7, timeperiod2=14, timeperiod3=28)
```

---

### WILLR - 威廉指标

```python
willr(high, low, close, timeperiod=14)
```

**解读:**
- Williams %R > -20: 超买
- Williams %R < -80: 超卖

---

## 成交量指标

### AD - 累积/派发线

```python
ad(high, low, close, volume)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `volume`: 成交量数组

---

### ADOSC - AD 振荡器

```python
adosc(high, low, close, volume, fastperiod=3, slowperiod=10)
```

---

### OBV - 能量潮

```python
obv(close, volume)
```

---

### VWAP - 成交量加权平均价

```python
vwap(high, low, close, volume, rolling_period=0, use_typical_price=False, band_multiplier=0.0)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `volume`: 成交量数组
- `rolling_period`: 滚动周期 (默认 0，表示累积计算)
- `use_typical_price`: 是否使用典型价格 (默认 False)
- `band_multiplier`: 通道倍数 (默认 0，不计算通道)

**返回:**
- `(vwap, upper, lower)` 三个数组

**用途:**
- 判断当日交易的平均成本
- 支撑/阻力位识别
- 短线交易参考

---

### Anchored VWAP - 锚定 VWAP

```python
anchored_vwap(high, low, close, volume, anchor_index, use_typical_price=False)
```

**参数:**
- `anchor_index`: 锚定起始位置索引

**用途:**
- 从特定事件点开始计算 VWAP
- 分析重大事件后的平均成本

---

### MFI - 资金流量指标

```python
mfi(high, low, close, volume, timeperiod=14)
```

**返回:**
- 0-100 范围的值

**解读:**
- > 80: 超买
- < 20: 超卖

---

### Volume Oscillator - 成交量震荡指标

```python
volume_oscillator(volume, fast_period=5, slow_period=10)
```

**用途:**
- 衡量成交量趋势变化
- 判断成交量放大或萎缩

---

### Twiggs Money Flow - Twiggs 资金流量

```python
twiggs_money_flow(high, low, close, volume, period=21)
```

**用途:**
- 改进的累积/派发指标
- 使用 EMA 平滑和真实波幅

---

### VZO - 成交量区域震荡指标

```python
vzo(close, volume, period=14)
```

**返回:**
- -100 到 100 范围的值

**解读:**
- > 40: 强势上涨
- < -40: 强势下跌

---

### Volume Momentum - 成交量动量

```python
volume_momentum(volume, period=10)
```

---

### Volume ROC - 成交量变化率

```python
volume_roc(volume, period=10)
```

---

### CMF - 赛金流量指标

```python
cmf(high, low, close, volume, timeperiod=20)
```

---

## 波动率指标

### ATR - 平均真实波幅

```python
atr(high, low, close, timeperiod=14)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `timeperiod`: 周期 (默认 14)

**用途:**
- 衡量波动性
- 设置止损位
- 计算仓位大小

---

### NATR - 归一化 ATR

```python
natr(high, low, close, timeperiod=14)
```

---

### TRANGE - 真实波幅

```python
trange(high, low, close)
```

---

### Keltner Channel - 肯特纳通道

```python
keltner(high, low, close, period=20, multiplier=2.0)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `close`: 收盘价数组
- `period`: ATR 和 EMA 周期 (默认 20)
- `multiplier`: ATR 倍数 (默认 2.0)

**返回:**
- `(upper, middle, lower, width)` 四个数组

**用途:**
- 趋势识别和通道突破
- 波动率衡量
- 与布林带类似的通道指标

---

### Historical Volatility - 历史波动率

```python
historical_volatility(close, period=20, trading_days=252)
```

**参数:**
- `close`: 收盘价数组
- `period`: 计算周期 (默认 20)
- `trading_days`: 年交易日数 (默认 252)

**返回:**
- 年化波动率百分比

**用途:**
- 风险评估
- 期权定价参考
- 波动率分析

---

### Ulcer Index - 溃疡指数

```python
ulcer_index(close, period=14)
```

**用途:**
- 衡量下行风险
- 关注回撤深度而非波动率
- 适合长期投资分析

---

### Choppiness Index - 震荡指数

```python
choppiness_index(high, low, close, period=14)
```

**返回:**
- 0-100 范围的值

**解读:**
- > 61.8: 市场震荡/盘整
- < 38.2: 市场趋势明显

---

### Mass Index - 质量指数

```python
mass_index(high, low, period=25, ema_period=9)
```

**用途:**
- 识别趋势反转
- "反转凸起"信号 (> 27)

---

### Chaikin Volatility - 蔡金波动率

```python
chaikin_volatility(high, low, ema_period=10, roc_period=10)
```

**用途:**
- 衡量高低点价差的波动率
- 波动率扩张/收缩分析

---

### ADR - 平均日波幅

```python
adr(high, low, close, period=14, mode='absolute')
```

**参数:**
- `mode`: 计算模式 ('absolute' 或 'percent')

**用途:**
- 日内波动幅度分析
- 支撑/阻力位估算

---

## 周期指标

### HT_DCPERIOD - 希尔伯特变换主导周期

```python
ht_dcperiod(close)
```

---

### HT_DCPHASE - 希尔伯特变换主导相位

```python
ht_dcphase(close)
```

---

### HT_PHASOR - 希尔伯特变换相量分量

```python
ht_phasor(close)
```

**返回:**
- `(inphase, quadrature)` 两个数组

---

### HT_SINE - 希尔伯特变换正弦波

```python
ht_sine(close)
```

**返回:**
- `(sine, leadsine)` 两个数组

---

### HT_TRENDMODE - 希尔伯特变换趋势模式

```python
ht_trendmode(close)
```

---

## 价格变换

### AVGPRICE - 平均价格

```python
avgprice(open, high, low, close)
```

**返回:**
- `(open + high + low + close) / 4`

---

### MEDPRICE - 中间价

```python
medprice(high, low)
```

**返回:**
- `(high + low) / 2`

---

### TYPPRICE - 典型价格

```python
typprice(high, low, close)
```

**返回:**
- `(high + low + close) / 3`

---

### WCLPRICE - 加权收盘价

```python
wclprice(high, low, close)
```

**返回:**
- `(high + low + 2*close) / 4`

---

## 统计指标

### STDDEV - 标准差

```python
stddev(close, timeperiod=5, nbdev=1)
```

---

### VAR - 方差

```python
var(close, timeperiod=5, nbdev=1)
```

---

### LINEARREG - 线性回归

```python
linearreg(close, timeperiod=14)
```

---

### LINEARREG_ANGLE - 线性回归角度

```python
linearreg_angle(close, timeperiod=14)
```

---

### LINEARREG_INTERCEPT - 线性回归截距

```python
linearreg_intercept(close, timeperiod=14)
```

---

### LINEARREG_SLOPE - 线性回归斜率

```python
linearreg_slope(close, timeperiod=14)
```

---

### TSF - 时间序列预测

```python
tsf(close, timeperiod=14)
```

---

### CORREL - 相关系数

```python
correl(x, y, timeperiod=30)
```

---

### BETA - Beta 系数

```python
beta(x, y, timeperiod=5)
```

---

## K线形态识别

所有 K 线形态函数返回整数数组：
- `100`: 看涨形态
- `-100`: 看跌形态
- `0`: 无形态

### CDL2CROWS - 两只乌鸦

```python
cdl_2crows(open, high, low, close)
```

---

### CDL3BLACKCROWS - 三只乌鸦

```python
cdl_3blackcrows(open, high, low, close)
```

---

### CDL3INSIDE - 三内部上涨/下跌

```python
cdl_3inside(open, high, low, close)
```

---

### CDL3OUTSIDE - 三外部上涨/下跌

```python
cdl_3outside(open, high, low, close)
```

---

### CDL3STARSINSOUTH - 南方三星

```python
cdl_3starsinsouth(open, high, low, close)
```

---

### CDL3WHITESOLDIERS - 三白兵

```python
cdl_3whitesoldiers(open, high, low, close)
```

---

### CDLABANDONEDBABY - 弃婴

```python
cdl_abandonedbaby(open, high, low, close, penetration=0.3)
```

---

### CDLADVANCEBLOCK - 前进受阻

```python
cdl_advanceblock(open, high, low, close)
```

---

### CDLBELTHOLD - 捉腰带线

```python
cdl_belthold(open, high, low, close)
```

---

### CDLBREAKAWAY - 突破

```python
cdl_breakaway(open, high, low, close)
```

---

### CDLCLOSINGMARUBOZU - 收盘长黑/长白

```python
cdl_closingmarubozu(open, high, low, close)
```

---

### CDLCONCEALBABYSWALL - 藏婴吞没

```python
cdl_concealbabyswall(open, high, low, close)
```

---

### CDLCOUNTERATTACK - 反击线

```python
cdl_counterattack(open, high, low, close)
```

---

### CDLDARKCLOUDCOVER - 乌云盖顶

```python
cdl_darkcloudcover(open, high, low, close, penetration=0.5)
```

---

### CDLDOJI - 十字星

```python
cdl_doji(open, high, low, close, doji_pct=0.1)
```

**参数:**
- `doji_pct`: 十字星判定阈值 (默认 0.1)

---

### CDLDOJISTAR - 十字星形态

```python
cdl_dojistar(open, high, low, close)
```

---

### CDLDRAGONFLYDOJI - 蜻蜓十字

```python
cdl_dragonflydoji(open, high, low, close)
```

---

### CDLENGULFING - 吞没形态

```python
cdl_engulfing(open, high, low, close)
```

---

### CDLEVENINGDOJISTAR - 晚星十字

```python
cdl_eveningdojistar(open, high, low, close, penetration=0.3)
```

---

### CDLEVENINGSTAR - 晚星

```python
cdl_eveningstar(open, high, low, close, penetration=0.3)
```

---

### CDLGAPSIDESIDEWHITE - 跳空并列阳线

```python
cdl_gapsidesidewhite(open, high, low, close)
```

---

### CDLGRAVESTONEDOJI - 墓碑十字

```python
cdl_gravestonedoji(open, high, low, close)
```

---

### CDLHAMMER - 锤子线

```python
cdl_hammer(open, high, low, close)
```

---

### CDLHANGINGMAN - 上吊线

```python
cdl_hangingman(open, high, low, close)
```

---

### CDLHARAMI - 母子线

```python
cdl_harami(open, high, low, close)
```

---

### CDLHARAMICROSS - 母子十字

```python
cdl_haramicross(open, high, low, close)
```

---

### CDLHIGHWAVE - 高浪线

```python
cdl_highwave(open, high, low, close)
```

---

### CDLHIKKAKE - Hikkake 形态

```python
cdl_hikkake(open, high, low, close)
```

---

### CDLHIKKAKEMOD - 修正 Hikkake

```python
cdl_hikkakemod(open, high, low, close)
```

---

### CDLHOMINGPIGEON - 归巢鸽

```python
cdl_homingpigeon(open, high, low, close)
```

---

### CDLIDENTICAL3CROWS - 相同三乌鸦

```python
cdl_identical3crows(open, high, low, close)
```

---

### CDLINNECK - 颈内线

```python
cdl_inneck(open, high, low, close)
```

---

### CDLINVERTEDHAMMER - 倒锤线

```python
cdl_invertedhammer(open, high, low, close)
```

---

### CDLKICKING - 跳空缺口

```python
cdl_kicking(open, high, low, close)
```

---

### CDLKICKINGBYLENGTH - 按长度跳空

```python
cdl_kickingbylength(open, high, low, close)
```

---

### CDLLADDERBOTTOM - 梯底

```python
cdl_ladderbottom(open, high, low, close)
```

---

### CDLLONGLEGGEDDOJI - 长腿十字

```python
cdl_longleggeddoji(open, high, low, close)
```

---

### CDLLONGLINE - 长阳/长阴

```python
cdl_longline(open, high, low, close)
```

---

### CDLMARUBOZU - 光头光脚

```python
cdl_marubozu(open, high, low, close)
```

---

### CDLMATCHINGLOW - 匹配低点

```python
cdl_matchinglow(open, high, low, close)
```

---

### CDLMATHOLD - 垫肩顶

```python
cdl_mathold(open, high, low, close, penetration=0.5)
```

---

### CDLMORNINGDOJISTAR - 晨星十字

```python
cdl_morningdojistar(open, high, low, close, penetration=0.3)
```

---

### CDLMORNINGSTAR - 晨星

```python
cdl_morningstar(open, high, low, close, penetration=0.3)
```

---

### CDLONNECK - 颈上线

```python
cdl_onneck(open, high, low, close)
```

---

### CDLPIERCING - 刺穿形态

```python
cdl_piercing(open, high, low, close)
```

---

### CDLRICKSHAWMAN - 车夫线

```python
cdl_rickshawman(open, high, low, close)
```

---

### CDLRISEFALL3METHODS - 三线上升/下跌

```python
cdl_risefall3methods(open, high, low, close)
```

---

### CDLSEPARATINGLINES - 分离线

```python
cdl_separatinglines(open, high, low, close)
```

---

### CDLSHOOTINGSTAR - 流星线

```python
cdl_shootingstar(open, high, low, close)
```

---

### CDLSHORTLINE - 短阳/短阴

```python
cdl_shortline(open, high, low, close)
```

---

### CDLSPINNINGTOP -纺锤顶

```python
cdl_spinningtop(open, high, low, close)
```

---

### CDLSTALLEDPATTERN - 停顿形态

```python
cdl_stalledpattern(open, high, low, close)
```

---

### CDLSTICKSANDWICH - 三明治

```python
cdl_sticksandwich(open, high, low, close)
```

---

### CDLTAKURI - 探底线

```python
cdl_takuri(open, high, low, close)
```

---

### CDLTASUKIGAP - 跳空缺口

```python
cdl_tasukigap(open, high, low, close)
```

---

### CDLTHRUSTING - 插入线

```python
cdl_thrusting(open, high, low, close)
```

---

### CDLTRISTAR - 三星形态

```python
cdl_tristar(open, high, low, close)
```

---

### CDLUNIQUE3RIVER - 独特三河

```python
cdl_unique3river(open, high, low, close)
```

---

### CDLUPSIDEGAP2CROWS - 向上跳空两只乌鸦

```python
cdl_upsidegap2crows(open, high, low, close)
```

---

### CDLXSIDEGAP3METHODS - 跳空三法

```python
cdl_xsidegap3methods(open, high, low, close)
```

---

## 图表形态识别

### detect_double_top - 双顶检测

```python
detect_double_top(high, lookback=20, tolerance=0.03)
```

**参数:**
- `high`: 最高价数组
- `lookback`: 回看周期 (默认 20)
- `tolerance`: 价格容差 (默认 0.03)

---

### detect_double_bottom - 双底检测

```python
detect_double_bottom(low, lookback=20, tolerance=0.03)
```

---

### detect_head_shoulders - 头肩顶检测

```python
detect_head_shoulders(high, lookback=30, tolerance=0.05)
```

---

### detect_head_shoulders_bottom - 头肩底检测

```python
detect_head_shoulders_bottom(low, lookback=30, tolerance=0.05)
```

---

### detect_triangle - 三角形检测

```python
detect_triangle(high, low, lookback=20)
```

---

### detect_wedge - 楔形检测

```python
detect_wedge(high, low, lookback=20)
```

---

### detect_flag - 旗形检测

```python
detect_flag(high, low, lookback=20)
```

---

## 经典形态指标

### Andrews Pitchfork - 安德鲁斯叉子

```python
andrews_pitchfork(high, low, pivot_a_idx, pivot_b_idx, pivot_c_idx, use_warning_lines=False)
```

**参数:**
- `high`: 最高价数组
- `low`: 最低价数组
- `pivot_a_idx`: 第一个转折点索引 (手柄起点)
- `pivot_b_idx`: 第二个转折点索引
- `pivot_c_idx`: 第三转折点索引
- `use_warning_lines`: 是否计算警告线 (默认 False)

**返回:**
- `(median_line, upper_line, lower_line, upper_warning, lower_warning)` 数组

**用途:**
- 趋势通道分析
- 支撑/阻力位识别
- 价格目标预测

---

### Gann Angles - 江恩角度线

```python
gann_angles(price, pivot_idx, pivot_price, price_unit=1.0, time_unit=1.0)
```

**参数:**
- `price`: 价格数组
- `pivot_idx`: 转折点索引
- `pivot_price`: 转折点价格
- `price_unit`: 价格单位 (默认 1.0)
- `time_unit`: 时间单位 (默认 1.0)

**返回:**
- `(angles, lines)` 包含各角度线和对应价格水平

**用途:**
- 时间/价格平衡分析
- 关键角度支撑/阻力

---

### Speed Resistance Lines - 速度阻力线

```python
speed_resistance_lines(high, low, start_idx, end_idx, is_uptrend=True)
```

**参数:**
- `start_idx`: 起始点索引
- `end_idx`: 结束点索引
- `is_uptrend`: 是否为上升趋势 (默认 True)

**返回:**
- `(line_1_3, line_2_3, line_1_2)` 三条速度线

**用途:**
- 趋势回调分析
- 1/3、2/3、1/2 速度线支撑/阻力

---

### Median Price - 中间价

```python
median_price(high, low)
```

**返回:**
- `(high + low) / 2`

---

### Weighted Close - 加权收盘价

```python
weighted_close(high, low, close)
```

**返回:**
- `(high + low + 2*close) / 4`

**用途:**
- 给收盘价更多权重
- 平滑价格数据

---

### Darvas Box - 达维斯箱体

```python
darvas_box(high, low, close, box_top_period=5, breakout_threshold=0.02)
```

**返回:**
- `(box_top, box_bottom, signal)` 三个数组

---

### Renko - 砖形图

```python
renko(close, brick_size=1.0)
```

**返回:**
- `(renko_high, renko_low, trend)` 三个数组

---

### Kagi - 卡吉图

```python
kagi(close, reversal_amount=0.01)
```

**返回:**
- `(kagi_price, kagi_direction)` 两个数组

---

### Point & Figure - 点数图

```python
point_and_figure(close, box_size=1.0, reversal_boxes=3)
```

**返回:**
- `(pf_column, pf_direction)` 两个数组

---

### Three Line Break - 三线反转

```python
three_line_break(close)
```

**返回:**
- `(tlb_price, tlb_direction)` 两个数组

---

### Williams Alligator - 威廉鳄鱼线

```python
williams_alligator(high, low, jaw_period=13, teeth_period=8, lips_period=5)
```

**返回:**
- `(jaw, teeth, lips)` 三个数组

---

### Heikin-Ashi - 平均K线

```python
heikin_ashi(open, high, low, close)
```

**返回:**
- `(ha_open, ha_high, ha_low, ha_close)` 四个数组

---

## 流式指标

流式指标支持 O(1) 每根 K 线增量更新，适合实时数据流处理。

### StreamingRSI

```python
streaming_rsi = ta.StreamingRSI(period=14)

# 逐根 K 线更新
for price in prices:
    rsi_value = streaming_rsi.update(price)
    print(f"RSI: {rsi_value}")

# 保存状态
state = streaming_rsi.save()

# 恢复状态
new_rsi = ta.StreamingRSI.from_state(state)
```

---

### StreamingMACD

```python
streaming_macd = ta.StreamingMACD(fast=12, slow=26, signal=9)

for price in prices:
    macd, signal, hist = streaming_macd.update(price)
```

---

### StreamingSMA

```python
streaming_sma = ta.StreamingSMA(period=20)

for price in prices:
    sma_value = streaming_sma.update(price)
```

---

### StreamingEMA

```python
streaming_ema = ta.StreamingEMA(period=12)

for price in prices:
    ema_value = streaming_ema.update(price)
```

---

### StreamingATR

```python
streaming_atr = ta.StreamingATR(period=14)

for h, l, c in zip(high, low, close):
    atr_value = streaming_atr.update(h, l, c)
```

---

## 公式引擎

公式引擎支持表达式计算，类似通达信/同花顺公式。

### 基础使用

```python
engine = ta.FormulaEngine()

# 计算布林带上轨
result = engine.evaluate("""
    MA(CLOSE, 20) + 2 * STDDEV(CLOSE, 20)
""", close=close)

# 计算自定义指标
result = engine.evaluate("""
    (CLOSE - MA(CLOSE, 20)) / STDDEV(CLOSE, 20)
""", close=close)
```

### 高频执行建议

同一公式需要重复执行时，建议先编译一次，并复用输出缓冲区。Rust 集成可使用
`FormulaEngine::eval_into`，减少最终结果和中间缓冲区的重复分配：

```rust
let mut engine = finkit::formula::FormulaEngine::new();
let formula = engine.compile("MA(CLOSE, 20)")?;
let mut output = ndarray::Array1::zeros(ctx.data_len);

for _ in 0..iterations {
    engine.eval_into(&formula, &mut ctx, &mut output)?;
}
```

`output` 长度必须等于 `ctx.data_len`。第一次执行完成预热后，公式执行器会复用内部缓冲区，
适合批量扫描、回测和实时循环。

### 支持的表达式

| 表达式 | 说明 |
|--------|------|
| `MA(CLOSE, period)` | 简单移动平均 |
| `EMA(CLOSE, period)` | 指数移动平均 |
| `STDDEV(CLOSE, period)` | 标准差 |
| `MAX(CLOSE, period)` | 周期内最大值 |
| `MIN(CLOSE, period)` | 周期内最小值 |
| `REF(CLOSE, n)` | n 周期前的值 |
| `SUM(CLOSE, period)` | 周期内求和 |
| `ABS(x)` | 绝对值 |
| `SQRT(x)` | 平方根 |
| `LOG(x)` | 自然对数 |
| `EXP(x)` | 指数函数 |
| `CLOSE` | 收盘价 |
| `OPEN` | 开盘价 |
| `HIGH` | 最高价 |
| `LOW` | 最低价 |
| `VOLUME` | 成交量 |

---

## 相关文档

- [完整使用指南](usage.md)
- [文档索引](README.md)
- [开发指南](development.md)
- [指标完整列表](indicators.md)