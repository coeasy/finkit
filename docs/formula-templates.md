# Formula Templates Reference

This document provides a complete reference of all 67 built-in formula templates in the Rust TA-Lib formula engine.

## Table of Contents

- [Overview](#overview)
- [Moving Average Templates](#moving-average-templates)
- [Oscillator Templates](#oscillator-templates)
- [Volatility Templates](#volatility-templates)
- [Volume Templates](#volume-templates)
- [Trend Templates](#trend-templates)
- [Strategy Templates](#strategy-templates)
- [Classic Templates (通达信经典)](#classic-templates)
- [Pattern Templates](#pattern-templates)
- [API Usage](#api-usage)

## Overview

The formula template library provides pre-built formulas for common technical analysis scenarios. Templates are organized into 8 categories:

| Category | Count | Description |
|----------|-------|-------------|
| Moving Average | 8 | MA/EMA crossovers, golden/death crosses |
| Oscillator | 12 | RSI, KDJ, MACD, CCI, Williams %R, etc. |
| Volatility | 8 | Bollinger Bands, ATR, Donchian Channels |
| Volume | 6 | Volume-price analysis, OBV, volume ratio |
| Trend | 6 | ADX, SAR, SuperTrend, Ichimoku |
| Strategy | 10 | Combined multi-indicator strategies |
| Classic | 12 | Chinese stock market classics (通达信) |
| Pattern | 5 | Candlestick patterns |

## Moving Average Templates

### ma_golden_cross (均线金叉)

**Description**: 短期均线上穿长期均线形成买入信号

**Formula**:
```
MA5:=MA(CLOSE,SHORT); MA10:=MA(CLOSE,LONG); CROSS(MA5,MA10)
```

**Parameters**:
- `SHORT`: 3 ~ 15, default 5 (短期周期)
- `LONG`: 10 ~ 60, default 10 (长期周期)

**Example**:
```python
template = Indicators.formula_get_template("ma_golden_cross")
result = Indicators.formula_eval(
    template["formula"], open, high, low, close, volume
)
```

### ma_death_cross (均线死叉)

**Description**: 短期均线下穿长期均线形成卖出信号

**Formula**:
```
MA5:=MA(CLOSE,SHORT); MA10:=MA(CLOSE,LONG); CROSS(MA10,MA5)
```

**Parameters**: Same as ma_golden_cross

### ma_alignment (均线多头排列)

**Description**: 短期、中期、长期均线呈多头排列

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20
```

### ema_cross (EMA交叉)

**Description**: 快速EMA上穿慢速EMA

**Formula**:
```
EMA12:=EMA(CLOSE,FAST); EMA26:=EMA(CLOSE,SLOW); CROSS(EMA12,EMA26)
```

**Parameters**:
- `FAST`: 5 ~ 20, default 12
- `SLOW`: 20 ~ 60, default 26

### ma_support (均线支撑)

**Description**: 价格回调至均线获得支撑

**Formula**:
```
MA20:=MA(CLOSE,N); LOW<=MA20*1.01 AND CLOSE>MA20
```

**Parameters**:
- `N`: 10 ~ 60, default 20

### ma_resistance (均线压力)

**Description**: 价格反弹至均线遇阻

**Formula**:
```
MA20:=MA(CLOSE,N); HIGH>=MA20*0.99 AND CLOSE<MA20
```

**Parameters**: Same as ma_support

### double_ma_cross (双均线交叉)

**Description**: 两条移动平均线的交叉信号

**Formula**:
```
MA1:=MA(CLOSE,FAST); MA2:=MA(CLOSE,SLOW); CROSS(MA1,MA2)
```

**Parameters**:
- `FAST`: 3 ~ 20, default 5
- `SLOW`: 10 ~ 100, default 20

### triple_ma_cross (三均线共振)

**Description**: 三条均线同时呈多头排列

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20 AND MA5>REF(MA5,1)
```

## Oscillator Templates

### macd_golden_cross (MACD金叉)

**Description**: DIF上穿DEA形成买入信号

**Formula**:
```
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; CROSS(DIF,DEA)
```

### macd_death_cross (MACD死叉)

**Description**: DEA上穿DIF形成卖出信号

**Formula**:
```
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; CROSS(DEA,DIF)
```

### macd_divergence (MACD底背离)

**Description**: 价格创新低但MACD未创新低

**Formula**:
```
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; CLOSE<REF(CLOSE,1) AND MACD>REF(MACD,1)
```

### kdj_golden_cross (KDJ金叉)

**Description**: K线上穿D线形成买入信号

**Formula**:
```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; CROSS(K,D)
```

### kdj_oversold (KDJ超卖)

**Description**: J值低于20进入超卖区域

**Formula**:
```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J<20
```

### rsi_golden_cross (RSI金叉)

**Description**: 短期RSI上穿长期RSI

**Formula**:
```
RSI1:=SMA(MAX(CLOSE-REF(CLOSE,1),0),SHORT,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),SHORT,1)*100; RSI2:=SMA(MAX(CLOSE-REF(CLOSE,1),0),LONG,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),LONG,1)*100; CROSS(RSI1,RSI2)
```

**Parameters**:
- `SHORT`: 3 ~ 14, default 6
- `LONG`: 12 ~ 30, default 12

### rsi_overbought (RSI超买)

**Description**: RSI超过70进入超买区域

**Formula**:
```
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI>70
```

**Parameters**:
- `N`: 6 ~ 30, default 14

### rsi_oversold (RSI超卖)

**Description**: RSI低于30进入超卖区域

**Formula**:
```
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI<30
```

**Parameters**: Same as rsi_overbought

### rsi_divergence (RSI背离)

**Description**: RSI与价格出现背离信号

**Formula**:
```
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; REF(RSI,1)<RSI AND CLOSE<REF(CLOSE,1)
```

### boll_break_up (布林带突破上轨)

**Description**: 收盘价突破布林带上轨

**Formula**:
```
MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*2; LOWER:=MID-STD(CLOSE,N)*2; CROSS(CLOSE,UPPER)
```

**Parameters**:
- `N`: 10 ~ 60, default 20

### boll_break_down (布林带跌破下轨)

**Description**: 收盘价跌破布林带下轨

**Formula**:
```
MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; CROSS(LOWER,CLOSE)
```

### boll_squeeze (布林带缩口)

**Description**: 布林带上下轨收窄，预示即将突破

**Formula**:
```
MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; (UPPER-LOWER)/MID*100<10
```

### boll_mid_support (布林带中轨支撑)

**Description**: 回调至布林带中轨获得支撑

**Formula**:
```
MID:=MA(CLOSE,20); CLOSE>MID AND REF(CLOSE,1)<MID
```

### stoch_overbought (随机指标超买)

**Description**: KD值超过80超买线

**Formula**:
```
RSV:=(CLOSE-LLV(LOW,N))/(HHV(HIGH,N)-LLV(LOW,N))*100; K:=SMA(RSV,M,1); K>80
```

**Parameters**:
- `N`: 5 ~ 30, default 14
- `M`: 2 ~ 10, default 3

### stoch_oversold (随机指标超卖)

**Description**: KD值低于20超卖线

**Formula**:
```
RSV:=(CLOSE-LLV(LOW,14))/(HHV(HIGH,14)-LLV(LOW,14))*100; K:=SMA(RSV,3,1); K<20
```

### williams_r (威廉指标)

**Description**: 威廉超买超卖指标

**Formula**:
```
WR:=(HHV(HIGH,N)-CLOSE)/(HHV(HIGH,N)-LLV(LOW,N))*100; WR>80
```

**Parameters**:
- `N`: 5 ~ 30, default 14

### cci_signal (CCI顺势指标)

**Description**: CCI突破+100或-100的信号

**Formula**:
```
TP:=(HIGH+LOW+CLOSE)/3; MA_TP:=MA(TP,N); MD_TP:=SUM(ABS(TP-MA_TP),N)/N; CCI:=(TP-MA_TP)/(0.015*MD_TP); CROSS(CCI,100)
```

**Parameters**:
- `N`: 5 ~ 30, default 14

### roc_momentum (ROC变动率)

**Description**: 价格变动率动量指标

**Formula**:
```
ROC:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; ROC>0
```

**Parameters**:
- `N`: 5 ~ 30, default 12

### momentum_signal (动量指标)

**Description**: 价格动量变化信号

**Formula**:
```
MTM:=CLOSE-REF(CLOSE,N); MA_MTM:=MA(MTM,M); CROSS(MTM,MA_MTM)
```

**Parameters**:
- `N`: 5 ~ 30, default 12
- `M`: 3 ~ 20, default 6

## Volatility Templates

### atr_breakout (ATR突破)

**Description**: 价格突破基于ATR的通道

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); UPPER:=REF(CLOSE,1)+ATR*MULT; LOWER:=REF(CLOSE,1)-ATR*MULT; CROSS(CLOSE,UPPER)
```

**Parameters**:
- `N`: 7 ~ 21, default 14
- `MULT`: 1.0 ~ 3.0, default 2.0

### volatility_expansion (波动率放大)

**Description**: 价格波动率显著放大

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); ATR/MA(ATR,M)>1.5
```

**Parameters**:
- `N`: 7 ~ 21, default 14
- `M`: 20 ~ 100, default 50

### volatility_contraction (波动率收缩)

**Description**: 价格波动率显著收缩

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); ATR/MA(ATR,M)<0.7
```

**Parameters**: Same as volatility_expansion

### donchian_breakout (唐安奇突破)

**Description**: 价格突破唐安奇通道

**Formula**:
```
UPPER:=HHV(HIGH,N); LOWER:=LLV(LOW,N); CROSS(CLOSE,UPPER)
```

**Parameters**:
- `N`: 10 ~ 60, default 20

### donchian_breakdown (唐安奇跌破)

**Description**: 价格跌破唐安奇通道

**Formula**:
```
UPPER:=HHV(HIGH,N); LOWER:=LLV(LOW,N); CROSS(LOWER,CLOSE)
```

**Parameters**: Same as donchian_breakout

### keltner_breakout (肯特纳突破)

**Description**: 价格突破肯特纳通道

**Formula**:
```
MID:=EMA(CLOSE,N); TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); UPPER:=MID+ATR*MULT; LOWER:=MID-ATR*MULT; CROSS(CLOSE,UPPER)
```

**Parameters**:
- `N`: 10 ~ 30, default 20
- `MULT`: 1.0 ~ 3.0, default 2.0

### standard_deviation (标准差突破)

**Description**: 价格突破N倍标准差通道

**Formula**:
```
MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*MULT; CROSS(CLOSE,UPPER)
```

**Parameters**:
- `N`: 10 ~ 60, default 20
- `MULT`: 1.0 ~ 4.0, default 2.0

### true_range (真实波幅)

**Description**: 真实波动范围分析

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); TR/MA(TR,N)>1.5
```

**Parameters**:
- `N`: 10 ~ 100, default 50

## Volume Templates

### volume_price_rise (量价齐升)

**Description**: 成交量和价格同时上涨

**Formula**:
```
CLOSE>REF(CLOSE,1) AND VOLUME>REF(VOLUME,1)
```

### volume_shrink_back (缩量回调)

**Description**: 价格回调但成交量萎缩，支撑有效

**Formula**:
```
CLOSE<REF(CLOSE,1) AND VOLUME<REF(VOLUME,1)
```

### volume_breakout (放量突破)

**Description**: 成交量显著放大伴随价格突破

**Formula**:
```
MAVOL:=MA(VOLUME,N); VOLUME>MAVOL*2 AND CLOSE>REF(HHV(HIGH,N),1)
```

**Parameters**:
- `N`: 5 ~ 60, default 20

### volume_ratio (量比指标)

**Description**: 当前成交量与平均成交量的比值

**Formula**:
```
MAVOL:=MA(VOLUME,N); VOLUME/MAVOL
```

**Parameters**:
- `N`: 3 ~ 30, default 5

### obv_trend (OBV能量潮)

**Description**: On Balance Volume能量潮趋势

**Formula**:
```
OBV:=SUM(IF(CLOSE>REF(CLOSE,1),VOLUME,IF(CLOSE<REF(CLOSE,1),-VOLUME,0)),N); CROSS(OBV,MA(OBV,M))
```

**Parameters**:
- `N`: 10 ~ 60, default 30
- `M`: 3 ~ 20, default 6

### volume_ma_cross (成交量均线交叉)

**Description**: 成交量短期均线上穿长期均线

**Formula**:
```
V5:=MA(VOLUME,5); V10:=MA(VOLUME,10); CROSS(V5,V10)
```

### volatility_volume (成交量变异率)

**Description**: 成交量的波动程度

**Formula**:
```
MAVOL:=MA(VOLUME,N); STD(VOLUME,N)/MAVOL*100
```

**Parameters**:
- `N`: 5 ~ 60, default 20

## Trend Templates

### adx_trend (ADX趋势强度)

**Description**: 平均方向性指数判断趋势强度

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); DMP:=SUM(IF(HIGH>REF(HIGH,1) AND HIGH-REF(HIGH,1)>REF(LOW,1)-LOW,MAX(HIGH-REF(HIGH,1),HIGH-REF(HIGH,1)),0),N); DMM:=SUM(IF(LOW<REF(LOW,1) AND REF(LOW,1)-LOW>REF(HIGH,1)-HIGH,MAX(REF(LOW,1)-LOW,REF(HIGH,1)-HIGH),0),N); DI1:=DMP/TR*N*100; DI2:=DMM/TR*N*100; ADX:=MA(ABS(DI1-DI2)/(DI1+DI2)*100,M); ADX>25
```

**Parameters**:
- `N`: 7 ~ 21, default 14
- `M`: 3 ~ 12, default 6

### sar_trend (SAR抛物线趋势)

**Description**: 抛物线转向指标判断趋势方向

**Formula**:
```
CLOSE>SAR
```

**Parameters**:
- `N`: 1 ~ 10, default 4
- `STEP`: 0.01 ~ 0.05, default 0.02
- `MAXSTEP`: 0.1 ~ 0.3, default 0.2

### trend_strength (趋势强度)

**Description**: 基于均线斜率的趋势强度指标

**Formula**:
```
MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); (MA5-REF(MA5,1))/MA5*100
```

### supertrend (超级趋势)

**Description**: 基于ATR的趋势跟踪指标

**Formula**:
```
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); MID:=MA(CLOSE,N); UPPER:=MID+ATR*MULT; LOWER:=MID-ATR*MULT; CROSS(CLOSE,LOWER)
```

**Parameters**:
- `N`: 7 ~ 21, default 10
- `MULT`: 1.0 ~ 4.0, default 3.0

### ichimoku_signal (一目均衡信号)

**Description**: 一目均衡图转换线与基准线交叉

**Formula**:
```
TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; CROSS(TENKAN,KIJUN)
```

### dmi_trend (DMI趋向指标)

**Description**: 上升下降方向线判断趋势

**Formula**:
```
MTR:=SUM(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),N); HD:=HIGH-REF(HIGH,1); LD:=REF(LOW,1)-LOW; DMP:=SUM(IF(HD>0 AND HD>LD,HD,0),N); DMM:=SUM(IF(LD>0 AND LD>HD,LD,0),N); PDI:=DMP/MTR*100; MDI:=DMM/MTR*100; PDI>MDI
```

**Parameters**:
- `N`: 7 ~ 28, default 14

## Strategy Templates

### ma_macd_strategy (均线+MACD策略)

**Description**: 均线多头排列且MACD金叉的综合买入策略

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MA5>MA10 AND MA10>MA20 AND CROSS(DIF,DEA)
```

### rsi_volume_strategy (RSI+成交量策略)

**Description**: RSI超卖且放量反弹的买入策略

**Formula**:
```
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MAVOL:=MA(VOLUME,5); RSI<30 AND VOLUME>MAVOL*1.5 AND CLOSE>REF(CLOSE,1)
```

### kdj_macd_strategy (KDJ+MACD策略)

**Description**: KDJ金叉与MACD金叉共振的买入策略

**Formula**:
```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); CROSS(K,D) AND CROSS(DIF,DEA)
```

### boll_rsi_strategy (布林带+RSI策略)

**Description**: 触及布林下轨且RSI超卖的反弹策略

**Formula**:
```
MID:=MA(CLOSE,20); LOWER:=MID-STD(CLOSE,20)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; CLOSE<LOWER AND RSI<30
```

### ma_volume_strategy (均线+成交量策略)

**Description**: 均线金叉且成交量放大的确认策略

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MAVOL:=MA(VOLUME,5); CROSS(MA5,MA10) AND VOLUME>MAVOL*1.5
```

### trend_reversal (趋势反转策略)

**Description**: MACD底背离加KDJ超卖的底部反转策略

**Formula**:
```
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); J:=3*K-2*SMA(K,3,1); MACD<0 AND REF(MACD,1)<MACD AND J<20
```

### breakout_strategy (突破策略)

**Description**: 放量突破近期高点的买入策略

**Formula**:
```
HIGH_N:=HHV(HIGH,N); VMA:=MA(VOLUME,M); CROSS(CLOSE,HIGH_N) AND VOLUME>VMA*2
```

**Parameters**:
- `N`: 10 ~ 60, default 20
- `M`: 3 ~ 20, default 5

### golden_triangle (黄金三角策略)

**Description**: 5日、10日、20日均线形成黄金三角

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20 AND MA5>REF(MA5,1)
```

### divergence_strategy (背离共振策略)

**Description**: MACD与RSI同时底背离的共振买入策略

**Formula**:
```
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MACD<0 AND REF(MACD,1)<MACD AND RSI<30 AND REF(RSI,1)<RSI
```

### ma_pullback (均线回踩策略)

**Description**: 上升趋势中回踩均线获得支撑

**Formula**:
```
MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); MA20>REF(MA20,1) AND CLOSE<MA5 AND CLOSE>MA20
```

## Classic Templates

### chip_peak (筹码峰)

**Description**: 基于成交量分布的筹码集中度分析

**Formula**:
```
COST1:=WINNER(CLOSE)*100; COST2:=WINNER(CLOSE*0.9)*100; CHIP_RATIO:=(COST1-COST2)/COST1*100; CHIP_RATIO>50
```

### jue_lu_biao (绝路航标)

**Description**: 通达信经典指标，底部反转信号

**Formula**:
```
VAR1:=LLV(LOW,21); VAR2:=HHV(HIGH,21); VAR3:=(CLOSE-VAR1)/(VAR2-VAR1)*100; VAR4:=SMA(VAR3,5,1); CROSS(VAR4,20)
```

### dragon_head (龙头指标)

**Description**: 通达信经典龙头股识别指标

**Formula**:
```
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; ZF>5 AND LTP>3
```

### main_force (主力资金)

**Description**: 主力资金流入流出指标

**Formula**:
```
MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_NET:=SUM(MF,N); CROSS(MF_NET,0)
```

**Parameters**:
- `N`: 3 ~ 30, default 10

### money_flow (资金流向)

**Description**: 大单资金净流入指标

**Formula**:
```
BIG:=IF(VOLUME>MA(VOLUME,5)*2,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); SMALL:=IF(VOLUME<MA(VOLUME,5)*0.5,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); NET:=SUM(BIG-SMALL,N); NET>0
```

**Parameters**:
- `N`: 3 ~ 20, default 5

### limit_up_capture (涨停捕捉)

**Description**: 捕捉即将涨停的信号

**Formula**:
```
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ZF>3 AND ZF<9 AND LTP>5 AND MA5>MA10
```

### bottom_fish (底部吸筹)

**Description**: 判断底部区域主力吸筹的指标

**Formula**:
```
VAR1:=(CLOSE-LLV(LOW,36))/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR4,VAR3) AND VAR4<20
```

### top_escape (顶部逃离)

**Description**: 判断顶部区域主力出货的指标

**Formula**:
```
VAR1:=(HHV(HIGH,36)-CLOSE)/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR3,VAR4) AND VAR3>80
```

### dragon_tiger (龙虎榜追踪)

**Description**: 追踪龙虎榜机构买卖方向

**Formula**:
```
VAR1:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; VAR2:=VOLUME/CAPITAL*100; VAR1>5 AND VAR2>REF(VAR2,1)*2
```

### golden_pit (黄金坑)

**Description**: 深度回调后的黄金坑形态

**Formula**:
```
VAR1:=LLV(LOW,60); VAR2:=CLOSE-VAR1; VAR3:=VAR2/VAR1*100; MA5:=MA(CLOSE,5); VAR3<20 AND CLOSE>MA5 AND REF(CLOSE,1)<REF(MA5,1)
```

### wave_theory (波浪理论指标)

**Description**: 基于波浪理论的买卖点判断

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); MA5>MA10 AND MA10>MA20 AND MA20>MA60 AND CLOSE>MA5
```

### pressure_support (压力支撑位)

**Description**: 计算关键的压力位和支撑位

**Formula**:
```
PP:=(HIGH+LOW+CLOSE)/3; R1:=PP*2-LOW; S1:=PP*2-HIGH; R2:=PP+(HIGH-LOW); S2:=PP-(HIGH-LOW); CLOSE>R1 OR CLOSE<S1
```

### change_rate (换手率指标)

**Description**: 换手率异常放大的信号

**Formula**:
```
HSL:=VOLUME/CAPITAL*100; MA_HSL:=MA(HSL,N); CROSS(HSL,MA_HSL*2)
```

**Parameters**:
- `N`: 3 ~ 20, default 5

### volume_price_divergence (量价背离)

**Description**: 价格与成交量出现背离

**Formula**:
```
PRICE_UP:=CLOSE>REF(CLOSE,1); VOL_DOWN:=VOLUME<REF(VOLUME,1); PRICE_UP AND VOL_DOWN AND CLOSE>MA(CLOSE,20)
```

### gap_fill (缺口回补)

**Description**: 跳空缺口及其回补信号

**Formula**:
```
GAP_UP:=LOW>REF(HIGH,1); GAP_DOWN:=HIGH<REF(LOW,1); FILLED:=LOW<=REF(HIGH,1) AND REF(LOW,1)>REF(HIGH,2); (GAP_UP OR GAP_DOWN) AND FILLED
```

### trend_acceleration (趋势加速)

**Description**: 价格上涨加速的信号

**Formula**:
```
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ACCEL:=(MA5-REF(MA5,1))-(REF(MA5,1)-REF(MA5,2)); ACCEL>0 AND CLOSE>MA5
```

## Pattern Templates

### three_white_soldiers (红三兵)

**Description**: 连续三根阳线，看涨形态

**Formula**:
```
CLOSE>OPEN AND REF(CLOSE,1)>REF(OPEN,1) AND REF(CLOSE,2)>REF(OPEN,2) AND CLOSE>REF(CLOSE,1) AND REF(CLOSE,1)>REF(CLOSE,2)
```

### three_black_crows (三只乌鸦)

**Description**: 连续三根阴线，看跌形态

**Formula**:
```
CLOSE<OPEN AND REF(CLOSE,1)<REF(OPEN,1) AND REF(CLOSE,2)<REF(OPEN,2) AND CLOSE<REF(CLOSE,1) AND REF(CLOSE,1)<REF(CLOSE,2)
```

### morning_star (早晨之星)

**Description**: 底部反转形态

**Formula**:
```
REF(CLOSE,2)<REF(OPEN,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))/REF(OPEN,1)<0.02 AND CLOSE>OPEN AND CLOSE>REF(CLOSE,1)
```

### evening_star (黄昏之星)

**Description**: 顶部反转形态

**Formula**:
```
REF(CLOSE,2)>REF(OPEN,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))/REF(OPEN,1)<0.02 AND CLOSE<OPEN AND CLOSE<REF(CLOSE,1)
```

### hammer (锤子线)

**Description**: 底部反转锤子线

**Formula**:
```
BODY:=ABS(CLOSE-OPEN); UPPER:=HIGH-MAX(CLOSE,OPEN); LOWER:=MIN(CLOSE,OPEN)-LOW; LOWER>2*BODY AND UPPER<BODY*0.5 AND CLOSE>OPEN
```

## API Usage

### Python

```python
from alpha_ta import Indicators

# Get a specific template
template = Indicators.formula_get_template("macd_golden_cross")
print(f"Name: {template['name']}")
print(f"Formula: {template['formula']}")

# Search templates by keyword
results = Indicators.formula_search_templates("MACD")
for t in results:
    print(f"{t['name']}: {t['description']}")

# List all categories
categories = Indicators.formula_list_categories()
for cat in categories:
    print(f"{cat['category']}: {cat['count']} templates")

# Evaluate a template
template = Indicators.formula_get_template("ma_golden_cross")
result = Indicators.formula_eval_bytecode(
    template["formula"], open, high, low, close, volume
)
```

### Node.js

```javascript
const { Indicators } = require('AlphaTA');

// Get template
const template = Indicators.formulaGetTemplate("macd_golden_cross");
console.log(template.name, template.formula);

// Search
const results = Indicators.formulaSearchTemplates("金叉");
console.log(results);

// List categories
const categories = Indicators.formulaListCategories();
console.log(categories);
```

### Java

```java
import com.alphata.Indicators;
import java.util.Map;

Map<String, Object> template = Indicators.formulaGetTemplate("macd_golden_cross");
System.out.println(template.get("name"));
System.out.println(template.get("formula"));
```

### Go

```go
import "github.com.alphata/ta"

template := ta.FormulaGetTemplate("macd_golden_cross")
fmt.Println(template.Name, template.Formula)

results := ta.FormulaSearchTemplates("金叉")
categories := ta.FormulaListCategories()
```

### .NET

```csharp
using AlphaTA;

var template = Indicators.FormulaGetTemplate("macd_golden_cross");
Console.WriteLine(template.Name);
Console.WriteLine(template.Formula);
```

### C++

```cpp
#include <alphata.hpp>

auto template = ta::FormulaGetTemplate("macd_golden_cross");
std::cout << template.name << std::endl;
```
