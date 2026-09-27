# Built-in Formula Template Catalogue

**317 templates in 12 categories.**

Generated from `core/src/formula/templates.rs` by
`python scripts/gen_ssot_docs.py --generate`; the CI gate
`python scripts/gen_ssot_docs.py --check` fails when this file and the
source disagree. Do not edit it by hand.

The `Key` column is the argument to `FormulaTemplates::get` (Rust),
`formula_get_template` (Python) and `formulaGetTemplate` (Node).

## Categories

| Category | Templates |
|----------|-----------|
| `Classic` | 27 |
| `DZHMoneyFlow` | 11 |
| `EMClassic` | 15 |
| `FoxTrader` | 3 |
| `MovingAverage` | 13 |
| `Oscillator` | 33 |
| `Pattern` | 45 |
| `Strategy` | 99 |
| `TDXClassic` | 15 |
| `THSSmartSelect` | 10 |
| `Trend` | 21 |
| `Volume` | 25 |

## Classic

### `bottom_fish` — 底部吸筹

判断底部区域主力吸筹的指标

```text
VAR1:=(CLOSE-LLV(LOW,36))/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR4,VAR3) AND VAR4<20
```

### `cause_effect` — 因果分析

威科夫因果分析

```text
TRADING_RANGE:=HHV(HIGH,N)-LLV(LOW,N); CAUSE:=TRADING_RANGE/LLV(LOW,N)*100; TARGET_MOVE:=CAUSE*1.5; CURRENT_MOVE:=ABS(CLOSE-LLV(LOW,N))/LLV(LOW,N)*100; CURRENT_MOVE<TARGET_MOVE
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `change_rate` — 换手率指标

换手率异常放大的信号

```text
HSL:=VOLUME/CAPITAL*100; MA_HSL:=MA(HSL,N); CROSS(HSL,MA_HSL*2)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 20 | 5 |

### `chip_peak` — 筹码峰

基于成交量分布的筹码集中度分析

```text
COST1:=WINNER(CLOSE)*100; COST2:=WINNER(CLOSE*0.9)*100; CHIP_RATIO:=(COST1-COST2)/COST1*100; CHIP_RATIO>50
```

### `composite_operator` — 综合操作者

威科夫综合操作者分析

```text
SMART_MONEY_BUY:=VOLUME>MA(VOLUME,20)*2 AND CLOSE>REF(CLOSE,1); SMART_MONEY_SELL:=VOLUME>MA(VOLUME,20)*2 AND CLOSE<REF(CLOSE,1); NET_SMART:=SUM(IF(SMART_MONEY_BUY,1,IF(SMART_MONEY_SELL,-1,0)),10); NET_SMART>3
```

### `dragon_head` — 龙头指标

通达信经典龙头股识别指标

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; ZF>5 AND LTP>3
```

### `dragon_tiger` — 龙虎榜追踪

追踪龙虎榜机构买卖方向

```text
VAR1:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; VAR2:=VOLUME/CAPITAL*100; VAR1>5 AND VAR2>REF(VAR2,1)*2
```

### `elliott_wave` — 波浪计数

艾略特波浪理论辅助

```text
MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); WAVE_UP:=MA5>REF(MA5,1) AND MA20>REF(MA20,1); WAVE_DOWN:=MA5<REF(MA5,1) AND MA20<REF(MA20,1); WAVE_UP AND REF(WAVE_DOWN,5)
```

### `fibonacci_retracement` — 斐波那契回撤

斐波那契回撤位策略

```text
HH:=HHV(HIGH,N); LL:=LLV(LOW,N); RANGE:=HH-LL; RETRACE_382:=HH-RANGE*0.382; RETRACE_618:=HH-RANGE*0.618; CLOSE>RETRACE_382 AND CLOSE<RETRACE_618
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `gann_angle` — 江恩角度线

江恩角度线支撑阻力

```text
LOW_N:=LLV(LOW,N); GANN_1X1:=LOW_N+(HIGH-LOW_N)*0.5; GANN_2X1:=LOW_N+(HIGH-LOW_N)*0.333; CLOSE>GANN_1X1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `gap_fill` — 缺口回补

跳空缺口及其回补信号

```text
GAP_UP:=LOW>REF(HIGH,1); GAP_DOWN:=HIGH<REF(LOW,1); FILLED:=LOW<=REF(HIGH,1) AND REF(LOW,1)>REF(HIGH,2); (GAP_UP OR GAP_DOWN) AND FILLED
```

### `golden_pit` — 黄金坑

深度回调后的黄金坑形态

```text
VAR1:=LLV(LOW,60); VAR2:=CLOSE-VAR1; VAR3:=VAR2/VAR1*100; MA5:=MA(CLOSE,5); VAR3<20 AND CLOSE>MA5 AND REF(CLOSE,1)<REF(MA5,1)
```

### `jue_lu_biao` — 绝路航标

通达信经典指标，底部反转信号

```text
VAR1:=LLV(LOW,21); VAR2:=HHV(HIGH,21); VAR3:=(CLOSE-VAR1)/(VAR2-VAR1)*100; VAR4:=SMA(VAR3,5,1); CROSS(VAR4,20)
```

### `law_of_cause_effect` — 因果法则

威科夫因果法则分析

```text
RANGE_DAYS:=HHV(HIGH,N)-LLV(LOW,N); CAUSE_DAYS:=N; EFFECT_MIN:=RANGE_DAYS*1.5; EFFECT_MAX:=RANGE_DAYS*3; CURRENT_MOVE:=ABS(CLOSE-REF(CLOSE,N)); CURRENT_MOVE>EFFECT_MIN AND CURRENT_MOVE<EFFECT_MAX
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `law_of_effort_result` — 努力结果法则

威科夫努力结果法则分析

```text
EFFORT:=VOLUME/MA(VOLUME,10); RESULT:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100); EFFORT_RESULT_RATIO:=RESULT/EFFORT; EFFORT_RESULT_RATIO>0.5 AND EFFORT_RESULT_RATIO<2
```

### `law_of_supply_demand` — 供需法则

威科夫供需法则分析

```text
DEMAND:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SUPPLY:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); DEMAND_MA:=MA(DEMAND,10); SUPPLY_MA:=MA(SUPPLY,10); DEMAND_MA>SUPPLY_MA*1.5
```

### `limit_up_capture` — 涨停捕捉

捕捉即将涨停的 signals

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ZF>3 AND ZF<9 AND LTP>5 AND MA5>MA10
```

### `main_force` — 主力资金

主力资金流入流出指标

```text
MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_NET:=SUM(MF,N); CROSS(MF_NET,0)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 30 | 10 |

### `market_profile` — 市场轮廓

市场轮廓价值区域

```text
POC:=MA(CLOSE,N); VALUE_HIGH:=POC+STD(CLOSE,N); VALUE_LOW:=POC-STD(CLOSE,N); CLOSE>VALUE_HIGH OR CLOSE<VALUE_LOW
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `mcst_cost` — MCST成本

市场成本指标，反映平均持仓成本

```text
MCST:=DMA(AMOUNT/VOLUME,VOLUME/CAPITAL); COST_DIFF:=(CLOSE-MCST)/MCST*100; COST_DIFF>-5 AND COST_DIFF<5
```

### `money_flow` — 资金流向

大单资金净流入指标

```text
BIG:=IF(VOLUME>MA(VOLUME,5)*2,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); SMALL:=IF(VOLUME<MA(VOLUME,5)*0.5,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); NET:=SUM(BIG-SMALL,N); NET>0
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 20 | 5 |

### `pivot_point` — 枢轴点策略

枢轴点支撑阻力策略

```text
PP:=(REF(HIGH,1)+REF(LOW,1)+REF(CLOSE,1))/3; R1:=2*PP-REF(LOW,1); S1:=2*PP-REF(HIGH,1); R2:=PP+(REF(HIGH,1)-REF(LOW,1)); S2:=PP-(REF(HIGH,1)-REF(LOW,1)); CROSS(CLOSE,R1)
```

### `pressure_support` — 压力支撑位

计算关键的压力位和支撑位

```text
PP:=(HIGH+LOW+CLOSE)/3; R1:=PP*2-LOW; S1:=PP*2-HIGH; R2:=PP+(HIGH-LOW); S2:=PP-(HIGH-LOW); CLOSE>R1 OR CLOSE<S1
```

### `top_escape` — 顶部逃离

判断顶部区域主力出货的指标

```text
VAR1:=(HHV(HIGH,36)-CLOSE)/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR3,VAR4) AND VAR3>80
```

### `trend_acceleration` — 趋势加速

价格上涨加速的信号

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ACCEL:=(MA5-REF(MA5,1))-(REF(MA5,1)-REF(MA5,2)); ACCEL>0 AND CLOSE>MA5
```

### `volume_price_divergence` — 量价背离

价格与成交量出现背离

```text
PRICE_UP:=CLOSE>REF(CLOSE,1); VOL_DOWN:=VOLUME<REF(VOLUME,1); PRICE_UP AND VOL_DOWN AND CLOSE>MA(CLOSE,20)
```

### `wave_theory` — 波浪理论指标

基于波浪理论的买卖点判断

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); MA5>MA10 AND MA10>MA20 AND MA20>MA60 AND CLOSE>MA5
```


## DZHMoneyFlow

### `dzh_accumulation` — 主力吸筹监控

大智慧主力吸筹行为监控，识别底部吸筹

```text
LOW_PRICE:=CLOSE<MA(CLOSE,60); VOL_SHRINK:=VOLUME<MA(VOLUME,20)*0.7; SMALL_UP:=IF(CLOSE>REF(CLOSE,1) AND VOLUME<MA(VOLUME,20),VOLUME,0); ACCUM:=SUM(SMALL_UP,10); ACCUM_TREND:=ACCUM>REF(ACCUM,5); LOW_PRICE AND ACCUM_TREND
```

### `dzh_big_order_net` — 大单净流入

大智慧大单净流入统计，分析超大单买卖方向

```text
AVG_PRICE:=AMOUNT/VOLUME; BIG_ORDER:=VOLUME>MA(VOLUME,20)*2; SUPER_BIG:=VOLUME>MA(VOLUME,20)*5; BIG_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_ORDER,VOLUME,0); BIG_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_ORDER,VOLUME,0); NET_BIG:=SUM(BIG_BUY-BIG_SELL,3); NET_BIG>MA(VOLUME,20)
```

### `dzh_continuous_inflow` — 连续资金流入

大智慧连续资金流入统计，识别持续流入股票

```text
DAY_FLOW:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); POS_DAYS:=COUNT(DAY_FLOW>0,N); CONSEC_UP:=POS_DAYS>=M; MA_FLOW:=MA(DAY_FLOW,5); FLOW_TREND:=DAY_FLOW>MA_FLOW; CONSEC_UP AND FLOW_TREND
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 10 | 5 |
| `M` | 2 | 5 | 3 |

### `dzh_distribution` — 主力出货监控

大智慧主力出货行为监控，识别高位出货

```text
HIGH_PRICE:=CLOSE>MA(CLOSE,60)*1.2; VOL_EXPAND:=VOLUME>MA(VOLUME,20)*1.5; BIG_DOWN:=IF(CLOSE<REF(CLOSE,1) AND VOLUME>MA(VOLUME,20),VOLUME,0); DISTRIB:=SUM(BIG_DOWN,5); DISTRIB_TREND:=DISTRIB>REF(DISTRIB,3); HIGH_PRICE AND DISTRIB_TREND
```

### `dzh_flow_trend` — 资金流向趋势

大智慧资金流向趋势分析，判断资金持续流入流出

```text
MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_MA5:=MA(MF,5); MF_MA10:=MA(MF,10); MF_MA20:=MA(MF,20); TREND_UP:=MF_MA5>MF_MA10 AND MF_MA10>MF_MA20; FLOW_POS:=SUM(MF,5)>0; TREND_UP AND FLOW_POS
```

### `dzh_main_inflow` — 主力资金流入

大智慧主力资金流入监控，追踪大资金动向

```text
BIG_VOL:=VOLUME>MA(VOLUME,20)*1.5; MAIN_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,VOLUME*CLOSE,0); MAIN_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,VOLUME*CLOSE,0); NET_FLOW:=SUM(MAIN_BUY-MAIN_SELL,N); NET_FLOW>0 AND REF(NET_FLOW,1)<0
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 20 | 5 |

### `dzh_retail_main` — 散户主力对比

大智慧散户与主力资金对比分析

```text
MAIN_VOL:=IF(VOLUME>MA(VOLUME,20)*1.5,VOLUME,0); RETAIL_VOL:=IF(VOLUME<MA(VOLUME,20)*0.8,VOLUME,0); MAIN_NET:=SUM(IF(CLOSE>REF(CLOSE,1),MAIN_VOL,-MAIN_VOL),5); RETAIL_NET:=SUM(IF(CLOSE>REF(CLOSE,1),RETAIL_VOL,-RETAIL_VOL),5); MAIN_NET>0 AND RETAIL_NET<0
```

### `dzh_sector_flow` — 板块资金流向

大智慧板块资金流向分析，识别板块资金动向

```text
SECTOR_VOL:=SUM(VOLUME,N); SECTOR_AMOUNT:=SUM(AMOUNT,N); AVG_PRICE:=SECTOR_AMOUNT/SECTOR_VOL; INDIV_FLOW:=IF(CLOSE>AVG_PRICE,VOLUME,-VOLUME); NET_FLOW:=SUM(INDIV_FLOW,5); MA_FLOW:=MA(NET_FLOW,10); CROSS(NET_FLOW,MA_FLOW)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 20 | 5 |

### `dzh_smart_money` — 聪明资金追踪

大智慧聪明资金追踪，识别机构资金动向

```text
SMART_BUY:=IF(CLOSE>REF(CLOSE,1) AND VOLUME>MA(VOLUME,20)*2,VOLUME*CLOSE,0); SMART_SELL:=IF(CLOSE<REF(CLOSE,1) AND VOLUME>MA(VOLUME,20)*2,VOLUME*CLOSE,0); NET_SMART:=SUM(SMART_BUY-SMART_SELL,10); SMA_NET:=MA(NET_SMART,5); NET_SMART>SMA_NET AND NET_SMART>0
```

### `dzh_top_bottom` — 资金顶底判断

大智慧资金流向判断顶底，资金背离分析

```text
MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_MA:=MA(MF,10); PRICE_UP:=CLOSE>REF(CLOSE,1); FLOW_DOWN:=MF<MF_MA; TOP_DIVERGE:=PRICE_UP AND FLOW_DOWN; PRICE_DOWN:=CLOSE<REF(CLOSE,1); FLOW_UP:=MF>MF_MA; BOTTOM_DIVERGE:=PRICE_DOWN AND FLOW_UP; TOP_DIVERGE OR BOTTOM_DIVERGE
```

### `dzh_turnover_rate` — 换手率资金流

大智慧换手率与资金流结合分析

```text
HSL:=VOLUME/CAPITAL*100; MA_HSL:=MA(HSL,5); HIGH_HSL:=HSL>MA_HSL*1.5; PRICE_UP:=CLOSE>REF(CLOSE,1); MONEY_IN:=IF(PRICE_UP AND HIGH_HSL,AMOUNT,0); NET_IN:=SUM(MONEY_IN,5); NET_IN>MA(AMOUNT,5)*1.2
```


## EMClassic

### `em_bull_bear` — 多空博弈

东方财富多空博弈指标，基于买卖力量对比判断多空趋势

```text
BUYV:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SELLV:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); NET_BUY:=BUYV-SELLV; MA_NET:=MA(NET_BUY,5); EM_CROSS(NET_BUY,MA_NET)
```

### `em_bull_bear_strength` — 多空强度

多空列强度对比

```text
DKCOL()/MA(VOLUME,5)
```

### `em_cost_dist` — 成本分布

东方财富成本分布指标，计算加权平均成本价

```text
AVG_COST:=EM_COSTEX(CLOSE,VOLUME); MA_COST:=MA(AVG_COST,10); DEV:=(CLOSE-AVG_COST)/AVG_COST*100; DEV<-5 AND CLOSE>MA_COST
```

### `em_cost_support` — 成本支撑

成本价支撑位

```text
EM_COSTEX(CLOSE,VOLUME)
```

### `em_cross_dead` — EM死叉

EM版死叉信号

```text
EM_CROSS(MA(CLOSE,20),MA(CLOSE,5))
```

### `em_cross_golden` — EM金叉

EM版金叉信号

```text
EM_CROSS(MA(CLOSE,5),MA(CLOSE,20))
```

### `em_fund_inflow` — 资金流入

主力资金流入判断

```text
EM_ZLCCV()
```

### `em_fund_trend` — 资金趋势

东方财富资金趋势指标，追踪主力资金流入流出方向

```text
MF:=IF(CLOSE>REF(CLOSE,1),AMOUNT,-AMOUNT); MF_MA5:=MA(MF,5); MF_MA10:=MA(MF,10); MF_MA20:=MA(MF,20); TREND_UP:=MF_MA5>MF_MA10 AND MF_MA10>MF_MA20; TREND_UP AND MF>0
```

### `em_main_track` — 主力追踪

东方财富主力追踪指标，监控主力资金动向

```text
BIG_VOL:=VOLUME>MA(VOLUME,20)*1.5; MAIN_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,AMOUNT,0); MAIN_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,AMOUNT,0); NET_MAIN:=SUM(MAIN_BUY-MAIN_SELL,5); MA_MAIN:=MA(NET_MAIN,10); EM_CROSS(NET_MAIN,MA_MAIN)
```

### `em_peak_resistance` — 峰值阻力

之字峰值阻力位

```text
EM_PEAK(1,5,1)
```

### `em_smart_combo` — 智能综合

多信号综合

```text
DKCOL();EM_CROSS(MA(C,5),MA(C,20));EM_ZLCCV()
```

### `em_smart_select` — 智能选股EM版

东方财富智能选股，综合多空、资金、趋势信号

```text
BUYV:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SELLV:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); NET_BUY:=BUYV-SELLV; MA_NET:=MA(NET_BUY,5); BULL_SIGNAL:=EM_CROSS(NET_BUY,MA_NET); MF:=IF(CLOSE>REF(CLOSE,1),AMOUNT,-AMOUNT); MF_MA:=MA(MF,10); FUND_SIGNAL:=MF>MF_MA; TREND:=MA(CLOSE,5)>MA(CLOSE,20); BULL_SIGNAL AND FUND_SIGNAL AND TREND
```

### `em_trough_support` — 谷值支撑

之字谷值支撑位

```text
EM_TROUGH(1,5,1)
```

### `em_volume_price` — 量价分析

量价配合分析

```text
EM_COSTEX(CLOSE,VOLUME);DKCOL()
```

### `em_zig_trend` — 之字趋势

EM之字转向趋势

```text
EM_ZIG(1,5)
```


## FoxTrader

### `fox_peak` — 飞狐峰标记

FoxTrader PEAK 峰识别，M 为邻域半径

```text
FOX_PEAK(CLOSE, 5, 1)
```

### `fox_trough` — 飞狐谷标记

FoxTrader TROUGH 谷识别，M 为邻域半径

```text
FOX_TROUGH(CLOSE, 5, 1)
```

### `fox_zig` — 飞狐之字转向

FoxTrader ZIG 之字转向线，标记价格转折点

```text
FOX_ZIG(CLOSE, 5)
```


## MovingAverage

### `bbi_multi_ma` — BBI多空指标

多空指数，多条均线综合判断

```text
BBI:=(MA(CLOSE,3)+MA(CLOSE,6)+MA(CLOSE,12)+MA(CLOSE,24))/4; CROSS(CLOSE,BBI)
```

### `bull_bear_line` — 多空线

基于均线的多空分界线

```text
DKX:=(3*MA(CLOSE,9)+MA(CLOSE,18)+MA(CLOSE,36))/5; CROSS(CLOSE,DKX)
```

### `ema_cross` — EMA金叉死叉

指数移动平均线交叉信号

```text
E1:=EMA(CLOSE,SHORT); E2:=EMA(CLOSE,LONG); CROSS(E1,E2)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 5 | 60 | 12 |
| `LONG` | 10 | 200 | 26 |

### `expma_cross` — EXPMA交叉

指数平均线交叉信号，反应更快

```text
EXP1:=EMA(CLOSE,SHORT); EXP2:=EMA(CLOSE,LONG); CROSS(EXP1,EXP2)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 5 | 30 | 12 |
| `LONG` | 10 | 60 | 50 |

### `expmi_exponential` — EMI指数移动

指数移动创新指标

```text
EMI:=EMA(CLOSE,N)-EMA(EMA(CLOSE,N),N); SIGNAL:=EMA(EMI,M); CROSS(EMI,SIGNAL)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 13 |
| `M` | 3 | 15 | 5 |

### `hma_trend` — 赫尔均线趋势

赫尔移动平均线趋势判断

```text
HMA:=2*EMA(CLOSE,N/2)-EMA(CLOSE,N); CLOSE>HMA
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 60 | 21 |

### `kama_kaufman` — KAMA考夫曼均线

考夫曼自适应移动平均线

```text
DIRECTION:=ABS(CLOSE-REF(CLOSE,N)); VOLATILITY:=SUM(ABS(CLOSE-REF(CLOSE,1)),N); ER:=DIRECTION/VOLATILITY; FAST:=2/(FAST_P+1); SLOW:=2/(SLOW_P+1); SC:=ER*(FAST-SLOW)+SLOW; KAMA:=REF(CLOSE,1)+SC*(CLOSE-REF(CLOSE,1)); CROSS(CLOSE,KAMA)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |
| `FAST_P` | 2 | 10 | 2 |
| `SLOW_P` | 20 | 40 | 30 |

### `ma_cross` — 均线金叉死叉

短周期均线上穿长周期均值为买入信号，下穿为卖出信号

```text
MA5:=MA(CLOSE,SHORT); MA10:=MA(CLOSE,LONG); CROSS(MA5,MA10)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 2 | 60 | 5 |
| `LONG` | 5 | 120 | 10 |

### `ma_deviation` — 均线偏离度

收盘价与均线的偏离百分比

```text
MA20:=MA(CLOSE,20); (CLOSE-MA20)/MA20*100
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 120 | 20 |

### `ma_multi` — 多均线多头排列

短期均线在长期均线之上，表示多头排列

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20
```

### `sma_ribbon` — 均线带

多条均线形成的带状区域，判断趋势方向

```text
M1:=MA(CLOSE,5); M2:=MA(CLOSE,10); M3:=MA(CLOSE,20); M4:=MA(CLOSE,60); M1-M4
```

### `vwap_deviation` — VWAP偏离

成交量加权平均价偏离度

```text
VWAP:=SUM(AMOUNT,N)/SUM(VOLUME,N); DEV:=(CLOSE-VWAP)/VWAP*100; DEV>THRESHOLD
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 20 |
| `THRESHOLD` | 1 | 5 | 2 |

### `wma_cross` — 加权均线交叉

加权移动平均线交叉信号

```text
W1:=WMA(CLOSE,SHORT); W2:=WMA(CLOSE,LONG); CROSS(W1,W2)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 5 | 30 | 10 |
| `LONG` | 20 | 120 | 30 |


## Oscillator

### `ac_accelerator` — AC加速震荡

加速震荡指标，AO的动量

```text
MEDIAN:=(HIGH+LOW)/2; AO:=MA(MEDIAN,5)-MA(MEDIAN,34); AC:=AO-MA(AO,5); CROSS(AC,0)
```

### `ao_awesome` — AO动量震荡

动量震荡指标，比尔威廉姆斯

```text
MEDIAN:=(HIGH+LOW)/2; AO:=MA(MEDIAN,5)-MA(MEDIAN,34); CROSS(AO,0)
```

### `asi_accumulation` — ASI累积震荡

累积摆动指标，真实波动幅度

```text
SI:=(CLOSE-REF(CLOSE,1)+REF(CLOSE,1)-REF(CLOSE,2))/2; ASI:=SUM(SI,N); MA_ASI:=MA(ASI,M); CROSS(ASI,MA_ASI)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |
| `M` | 3 | 10 | 5 |

### `boll_break_down` — 布林带跌破下轨

收盘价跌破布林带下轨

```text
MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; CROSS(LOWER,CLOSE)
```

### `boll_break_up` — 布林带突破上轨

收盘价突破布林带上轨

```text
MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*2; LOWER:=MID-STD(CLOSE,N)*2; CROSS(CLOSE,UPPER)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `boll_mid_support` — 布林带中轨支撑

回调至布林带中轨获得支撑

```text
MID:=MA(CLOSE,20); CLOSE>MID AND REF(CLOSE,1)<MID
```

### `boll_squeeze` — 布林带缩口

布林带上下轨收窄，预示即将突破

```text
MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; (UPPER-LOWER)/MID*100<10
```

### `boll_width` — 布林带宽度

布林带宽度变化，判断波动性

```text
MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*2; LOWER:=MID-STD(CLOSE,N)*2; WIDTH:=(UPPER-LOWER)/MID*100; WIDTH<REF(WIDTH,1)*0.7
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `brar_emotion` — BRAR情绪指标

买卖意愿指标，市场情绪分析

```text
AR:=SUM(HIGH-OPEN,N)/SUM(OPEN-LOW,N)*100; BR:=SUM(MAX(HIGH-REF(CLOSE,1),0),N)/SUM(MAX(REF(CLOSE,1)-LOW,0),N)*100; AR<50 AND BR<40
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 26 |

### `cci_signal` — CCI顺势指标

CCI突破+100或-100的信号

```text
TP:=(HIGH+LOW+CLOSE)/3; MA_TP:=MA(TP,N); MD_TP:=SUM(ABS(TP-MA_TP),N)/N; CCI:=(TP-MA_TP)/(0.015*MD_TP); CROSS(CCI,100)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 14 |

### `dpo_oscillator` — DPO去趋势

去趋势价格震荡指标，消除趋势影响

```text
DPO:=CLOSE-REF(MA(CLOSE,N),N/2+1); MA_DPO:=MA(DPO,M); CROSS(DPO,MA_DPO)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |
| `M` | 3 | 10 | 5 |

### `env_envelope` — ENV包络线

价格包络线指标，判断超买超卖

```text
MID:=MA(CLOSE,N); UPPER:=MID*(1+PCT/100); LOWER:=MID*(1-PCT/100); CLOSE<LOWER
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 14 |
| `PCT` | 1 | 10 | 3 |

### `kdj_death_cross` — KDJ死叉

K线下穿D线形成死叉卖出信号

```text
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); CROSS(D,K)
```

### `kdj_golden_cross` — KDJ金叉

K线上穿D线形成金叉买入信号

```text
RSV:=(CLOSE-LLV(LOW,N))/(HHV(HIGH,N)-LLV(LOW,N))*100; K:=SMA(RSV,M1,1); D:=SMA(K,M2,1); J:=3*K-2*D; CROSS(K,D)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 30 | 9 |
| `M1` | 2 | 10 | 3 |
| `M2` | 2 | 10 | 3 |

### `kdj_overbought` — KDJ超买

J值超过80进入超买区域

```text
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J>80
```

### `kdj_oversold` — KDJ超卖

J值低于20进入超卖区域

```text
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J<20
```

### `keltner_channel` — 肯特纳通道

肯特纳通道，基于ATR的波动通道

```text
TYP:=(HIGH+LOW+CLOSE)/3; ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),N); UPPER:=MA(TYP,N)+ATR*M; LOWER:=MA(TYP,N)-ATR*M; CLOSE<LOWER
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |
| `M` | 1 | 3 | 2 |

### `kst_momentum` — KST确知指标

确知指标，综合动量分析

```text
ROC1:=MA((CLOSE-REF(CLOSE,10))/REF(CLOSE,10)*100,10); ROC2:=MA((CLOSE-REF(CLOSE,15))/REF(CLOSE,15)*100,10); ROC3:=MA((CLOSE-REF(CLOSE,20))/REF(CLOSE,20)*100,10); ROC4:=MA((CLOSE-REF(CLOSE,30))/REF(CLOSE,30)*100,15); KST:=ROC1+ROC2*2+ROC3*3+ROC4*4; SIGNAL:=MA(KST,9); CROSS(KST,SIGNAL)
```

### `mass_index` — MIK质量指数

质量指数，识别趋势反转

```text
RANGE:=HIGH-LOW; ER:=EMA(RANGE,N)/EMA(EMA(RANGE,N),N); MIK:=SUM(ER,M); MIK>27
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 15 | 9 |
| `M` | 15 | 35 | 25 |

### `momentum_signal` — 动量指标

价格动量变化信号

```text
MTM:=CLOSE-REF(CLOSE,N); MA_MTM:=MA(MTM,M); CROSS(MTM,MA_MTM)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 12 |
| `M` | 3 | 20 | 6 |

### `ppo_percentage` — PPO百分比震荡

价格震荡百分比指标

```text
PPO:=(EMA(CLOSE,SHORT)-EMA(CLOSE,LONG))/EMA(CLOSE,LONG)*100; SIGNAL:=EMA(PPO,M); CROSS(PPO,SIGNAL)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 5 | 15 | 12 |
| `LONG` | 20 | 35 | 26 |
| `M` | 5 | 15 | 9 |

### `psy_psychological` — PSY心理线

心理线指标，反映投资者心理预期

```text
UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),N); PSY:=UP_DAYS/N*100; PSY<25
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 12 |

### `roc_momentum` — ROC变动率

价格变动率动量指标

```text
ROC:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; ROC>0
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 12 |

### `rsi_divergence` — RSI背离

RSI与价格出现背离信号

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; REF(RSI,1)<RSI AND CLOSE<REF(CLOSE,1)
```

### `rsi_golden_cross` — RSI金叉

短期RSI上穿长期RSI

```text
RSI1:=SMA(MAX(CLOSE-REF(CLOSE,1),0),SHORT,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),SHORT,1)*100; RSI2:=SMA(MAX(CLOSE-REF(CLOSE,1),0),LONG,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),LONG,1)*100; CROSS(RSI1,RSI2)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 3 | 14 | 6 |
| `LONG` | 12 | 30 | 12 |

### `rsi_overbought` — RSI超买

RSI超过70进入超买区域

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI>70
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 6 | 30 | 14 |

### `rsi_oversold` — RSI超卖

RSI低于30进入超卖区域

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI<30
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 6 | 30 | 14 |

### `stoch_overbought` — 随机指标超买

KD值超过80超买线

```text
RSV:=(CLOSE-LLV(LOW,N))/(HHV(HIGH,N)-LLV(LOW,N))*100; K:=SMA(RSV,M,1); K>80
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 14 |
| `M` | 2 | 10 | 3 |

### `stoch_oversold` — 随机指标超卖

KD值低于20超卖线

```text
RSV:=(CLOSE-LLV(LOW,14))/(HHV(HIGH,14)-LLV(LOW,14))*100; K:=SMA(RSV,3,1); K<20
```

### `trix_signal` — TRIX指标

三重指数平滑移动平均指标

```text
TR:=EMA(EMA(EMA(CLOSE,N),N),N); TRIX:=(TR-REF(TR,1))/REF(TR,1)*100; TRMA:=MA(TRIX,M); CROSS(TRIX,TRMA)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 12 |
| `M` | 5 | 20 | 9 |

### `tsi_true` — TSI真实强度

真实强度指数，双重平滑动量

```text
MOM:=CLOSE-REF(CLOSE,1); SMOOTH1:=EMA(EMA(MOM,N),N); SMOOTH2:=EMA(EMA(ABS(MOM),N),N); TSI:=SMOOTH1/SMOOTH2*100; SIGNAL:=EMA(TSI,M); CROSS(TSI,SIGNAL)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 25 |
| `M` | 5 | 20 | 13 |

### `uo_ultimate` — UO终极震荡

终极震荡指标，多周期加权

```text
BP:=CLOSE-MIN(LOW,REF(CLOSE,1)); TR1:=MAX(HIGH,REF(CLOSE,1))-MIN(LOW,REF(CLOSE,1)); AVG7:=SUM(BP,7)/SUM(TR1,7); AVG14:=SUM(BP,14)/SUM(TR1,14); AVG28:=SUM(BP,28)/SUM(TR1,28); UO:=100*((4*AVG7+2*AVG14+AVG28)/7); UO<30
```

### `williams_r` — 威廉指标

威廉超买超卖指标

```text
WR:=(HHV(HIGH,N)-CLOSE)/(HHV(HIGH,N)-LLV(LOW,N))*100; WR>80
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 14 |


## Pattern

### `accumulation_zone` — 吸筹区间

威科夫吸筹区间识别

```text
RANGE:=HHV(HIGH,N)-LLV(LOW,N); RANGE_PCT:=RANGE/LLV(LOW,N)*100; NARROW_RANGE:=RANGE_PCT<15; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,20); NARROW_RANGE AND VOL_DECLINE
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 40 | 20 |

### `ar_pattern` — AR形态

威科夫自动反弹

```text
AFTER_SC:=REF(LOW,1)==LLV(LOW,10); PRICE_UP:=CLOSE>REF(CLOSE,1)*1.02; VOL_MODERATE:=VOLUME>MA(VOLUME,10)*0.8 AND VOLUME<MA(VOLUME,10)*1.5; AFTER_SC AND PRICE_UP AND VOL_MODERATE
```

### `backup_deep` — 深回调

威科夫深回调形态

```text
BREAK_HIGH:=REF(CLOSE,2)>REF(HHV(HIGH,20),2); DEEP_PULLBACK:=CLOSE<REF(CLOSE,2)*0.95; VOL_MODERATE:=VOLUME>MA(VOLUME,10)*0.5; BREAK_HIGH AND DEEP_PULLBACK AND VOL_MODERATE
```

### `backup_shallow` — 浅回调

威科夫浅回调形态

```text
BREAK_HIGH:=REF(CLOSE,1)>REF(HHV(HIGH,20),1); PULLBACK:=CLOSE<REF(CLOSE,1); SHALLOW:=CLOSE>REF(CLOSE,1)*0.97; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; BREAK_HIGH AND PULLBACK AND SHALLOW AND VOL_LOW
```

### `cup_handle` — 杯柄形态

杯柄形态，圆底后小幅回调再突破

```text
HIGH_N:=HHV(HIGH,N); LOW_N:=LLV(LOW,N); MID:=(HIGH_N+LOW_N)/2; CLOSE>HIGH_N*0.95 AND LOW_N>MID*0.9
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 90 | 40 |

### `dark_cloud` — 乌云盖顶

乌云盖顶看跌信号

```text
C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN>REF(HIGH,1); C4:=CLOSE<REF(OPEN,1)+(REF(CLOSE,1)-REF(OPEN,1))/2; C1 AND C2 AND C3 AND C4
```

### `distribution_zone` — 派发区间

威科夫派发区间识别

```text
RANGE:=HHV(HIGH,N)-LLV(LOW,N); RANGE_PCT:=RANGE/LLV(LOW,N)*100; NARROW_RANGE:=RANGE_PCT<15; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,20); HIGH_PRICE:=CLOSE>MA(CLOSE,60); NARROW_RANGE AND VOL_DECLINE AND HIGH_PRICE
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 40 | 20 |

### `doji_star` — 十字星

十字星形态，市场犹豫

```text
BODY:=ABS(CLOSE-OPEN); RANGE:=HIGH-LOW; BODY<RANGE*0.1 AND RANGE>REF(RANGE,1)*0.5
```

### `double_bottom` — 双底形态

W底双底形态，两个低点接近且中间有反弹

```text
L1:=LLV(LOW,N); L2:=REF(L1,N); BOUNCE:=HHV(HIGH,N/2); L1<REF(L1,1)*1.02 AND CLOSE>BOUNCE*0.98
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `double_top` — 双顶形态

M头双顶形态，两个高点接近且中间有回调

```text
H1:=HHV(HIGH,N); H2:=REF(H1,N); DROP:=LLV(LOW,N/2); H1>REF(H1,1)*0.98 AND CLOSE<DROP*1.02
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `engulfing_bear` — 看跌吞没

看跌吞没形态

```text
C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN>REF(CLOSE,1) AND CLOSE<REF(OPEN,1); C1 AND C2 AND C3
```

### `engulfing_bull` — 看涨吞没

看涨吞没形态

```text
C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN<REF(CLOSE,1) AND CLOSE>REF(OPEN,1); C1 AND C2 AND C3
```

### `evening_star` — 黄昏星

黄昏星看跌反转形态

```text
C1:=REF(CLOSE,2)>REF(OPEN,2) AND (REF(CLOSE,2)-REF(OPEN,2))/REF(OPEN,2)>0.02; C2:=REF(OPEN,1)>REF(CLOSE,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))<REF(OPEN,2)*0.01; C3:=CLOSE<OPEN AND CLOSE<(REF(OPEN,2)+REF(CLOSE,2))/2; C1 AND C2 AND C3
```

### `falling_three` — 下降三法

下降三法持续形态

```text
C1:=REF(CLOSE,4)<REF(OPEN,4) AND (REF(OPEN,4)-REF(CLOSE,4))/REF(OPEN,4)>0.02; C2:=REF(CLOSE,3)>REF(OPEN,3) AND REF(CLOSE,3)<REF(OPEN,4); C3:=REF(CLOSE,2)>REF(OPEN,2) AND REF(CLOSE,2)<REF(OPEN,4); C4:=REF(CLOSE,1)>REF(OPEN,1) AND REF(CLOSE,1)<REF(OPEN,4); C5:=CLOSE<OPEN AND CLOSE<REF(CLOSE,4); C1 AND C2 AND C3 AND C4 AND C5
```

### `flag_pattern` — 旗形形态

旗形整理形态，急涨后窄幅整理

```text
RANGE:=(HHV(HIGH,N)-LLV(LOW,N))/LLV(LOW,N)*100; RANGE<5 AND CLOSE>REF(CLOSE,N)*1.05
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |

### `fractal_break` — 分形突破

比尔威廉姆斯分形突破

```text
UP_FRACTAL:=HIGH>REF(HIGH,1) AND HIGH>REF(HIGH,2) AND REF(HIGH,2)>REF(HIGH,3) AND REF(HIGH,2)>REF(HIGH,4); DOWN_FRACTAL:=REF(LOW,2)<REF(LOW,1) AND REF(LOW,2)<LOW AND REF(LOW,2)<REF(LOW,3) AND REF(LOW,2)<REF(LOW,4); CLOSE>REF(HHV(HIGH,5),1)
```

### `hammer_pattern` — 锤子线

锤子线底部反转形态

```text
BODY:=ABS(CLOSE-OPEN); LOWER_SHADOW:=MIN(CLOSE,OPEN)-LOW; UPPER_SHADOW:=HIGH-MAX(CLOSE,OPEN); LOWER_SHADOW>BODY*2 AND UPPER_SHADOW<BODY*0.5 AND BODY>0
```

### `harami_bear` — 看跌孕育

看跌孕育形态

```text
C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN<REF(CLOSE,1) AND CLOSE>REF(OPEN,1); C1 AND C2 AND C3
```

### `harami_bull` — 看涨孕育

看涨孕育形态

```text
C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN>REF(CLOSE,1) AND CLOSE<REF(OPEN,1); C1 AND C2 AND C3
```

### `harmonic_pattern` — 谐波形态

谐波形态识别

```text
X:=REF(LOW,4); A:=REF(HIGH,3); B:=REF(LOW,2); C:=REF(HIGH,1); AB_RATIO:=(A-B)/(A-X); BC_RATIO:=(C-B)/(A-B); AB_RATIO>0.382 AND AB_RATIO<0.886 AND BC_RATIO>0.382 AND BC_RATIO<0.886
```

### `head_shoulder_top` — 头肩顶形态

头肩顶反转形态，左肩头部右肩依次形成

```text
H1:=HHV(HIGH,N); L1:=LLV(LOW,N/2); H2:=HHV(HIGH,N/3); CLOSE<L1 AND H1>REF(H1,1) AND H2<H1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 15 | 60 | 30 |

### `ice_breaking` — 冰线突破

威科夫冰线突破形态

```text
ICE_LEVEL:=MA(LOW,20); BREAK_DOWN:=CLOSE<ICE_LEVEL; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; NO_RECOVERY:=CLOSE<OPEN; BREAK_DOWN AND VOL_HIGH AND NO_RECOVERY
```

### `jump_across_creek` — 跳过小溪

威科夫跳过小溪形态

```text
RESISTANCE:=HHV(HIGH,20); BREAK_UP:=CLOSE>RESISTANCE; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; CONFIRM:=CLOSE>OPEN; BREAK_UP AND VOL_HIGH AND CONFIRM
```

### `lps_pattern` — LPS形态

威科夫最后支撑点

```text
HH:=HHV(HIGH,20); NEAR_HIGH:=CLOSE>HH*0.95; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; PULLBACK:=CLOSE<REF(CLOSE,1); NEAR_HIGH AND VOL_LOW AND PULLBACK
```

### `morning_star` — 启明星

启明星看涨反转形态

```text
C1:=REF(CLOSE,2)<REF(OPEN,2) AND (REF(OPEN,2)-REF(CLOSE,2))/REF(OPEN,2)>0.02; C2:=REF(OPEN,1)<REF(CLOSE,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))<REF(OPEN,2)*0.01; C3:=CLOSE>OPEN AND CLOSE>(REF(OPEN,2)+REF(CLOSE,2))/2; C1 AND C2 AND C3
```

### `piercing_line` — 刺透形态

刺透形态看涨信号

```text
C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN<REF(LOW,1); C4:=CLOSE>REF(OPEN,1)+(REF(CLOSE,1)-REF(OPEN,1))/2; C1 AND C2 AND C3 AND C4
```

### `ps_pattern` — PS形态

威科夫初步支撑

```text
DOWNTREND:=MA(CLOSE,10)<MA(CLOSE,20); VOL_HIGH:=VOLUME>MA(VOLUME,20)*2; PRICE_BOUNCE:=CLOSE>REF(CLOSE,1); DOWNTREND AND VOL_HIGH AND PRICE_BOUNCE
```

### `reaccumulation` — 再吸筹

威科夫再吸筹形态

```text
UPTREND:=MA(CLOSE,10)>MA(CLOSE,30); CONSOLIDATION:=HHV(HIGH,10)-LLV(LOW,10)<MA(CLOSE,10)*0.05; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,10); UPTREND AND CONSOLIDATION AND VOL_DECLINE
```

### `redistribution` — 再派发

威科夫再派发形态

```text
DOWNTREND:=MA(CLOSE,10)<MA(CLOSE,30); CONSOLIDATION:=HHV(HIGH,10)-LLV(LOW,10)<MA(CLOSE,10)*0.05; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,10); DOWNTREND AND CONSOLIDATION AND VOL_DECLINE
```

### `rising_three` — 上升三法

上升三法持续形态

```text
C1:=REF(CLOSE,4)>REF(OPEN,4) AND (REF(CLOSE,4)-REF(OPEN,4))/REF(OPEN,4)>0.02; C2:=REF(CLOSE,3)<REF(OPEN,3) AND REF(CLOSE,3)>REF(OPEN,4); C3:=REF(CLOSE,2)<REF(OPEN,2) AND REF(CLOSE,2)>REF(OPEN,4); C4:=REF(CLOSE,1)<REF(OPEN,1) AND REF(CLOSE,1)>REF(OPEN,4); C5:=CLOSE>OPEN AND CLOSE>REF(CLOSE,4); C1 AND C2 AND C3 AND C4 AND C5
```

### `sc_pattern` — SC形态

威科夫卖出高潮

```text
VOL_EXTREME:=VOLUME>MA(VOLUME,20)*3; PRICE_PANIC:=CLOSE<REF(CLOSE,1)*0.95; RECOVERY:=CLOSE>LOW+(HIGH-LOW)*0.5; VOL_EXTREME AND PRICE_PANIC AND RECOVERY
```

### `secondary_test` — 二次测试

威科夫二次测试形态

```text
FIRST_LOW:=REF(LOW,5)==LLV(LOW,10); SECOND_LOW:=LOW<REF(LOW,1)*1.01; VOL_DECREASE:=VOLUME<REF(VOLUME,5); FIRST_LOW AND SECOND_LOW AND VOL_DECREASE
```

### `shakeout_pattern` — 震仓形态

威科夫震仓形态

```text
SHARP_DROP:=LOW<REF(LOW,1)*0.95; QUICK_RECOVERY:=CLOSE>REF(CLOSE,1); VOL_SPIKE:=VOLUME>MA(VOLUME,10)*2; SHARP_DROP AND QUICK_RECOVERY AND VOL_SPIKE
```

### `shooting_star` — 流星线

流星线顶部反转形态

```text
BODY:=ABS(CLOSE-OPEN); UPPER_SHADOW:=HIGH-MAX(CLOSE,OPEN); LOWER_SHADOW:=MIN(CLOSE,OPEN)-LOW; UPPER_SHADOW>BODY*2 AND LOWER_SHADOW<BODY*0.5 AND BODY>0
```

### `sos_sign` — SOS信号

威科夫强势信号

```text
PRICE_UP:=CLOSE>REF(CLOSE,1)*1.02; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_HIGH:=(HIGH-LOW)>MA(HIGH-LOW,10)*1.3; PRICE_UP AND VOL_HIGH AND SPREAD_HIGH
```

### `sow_pattern` — SOW形态

威科夫弱势信号

```text
PRICE_DOWN:=CLOSE<REF(CLOSE,1)*0.98; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_HIGH:=(HIGH-LOW)>MA(HIGH-LOW,10)*1.3; PRICE_DOWN AND VOL_HIGH AND SPREAD_HIGH
```

### `spring_pattern` — 弹簧形态

威科夫弹簧形态

```text
LL:=LLV(LOW,20); SPRING:=LOW<LL AND CLOSE>LL; VOL_LOW:=VOLUME<MA(VOLUME,10); SPRING AND VOL_LOW
```

### `st_pattern` — ST形态

威科夫二次测试

```text
AR_HIGH:=REF(HIGH,3)==HHV(HIGH,10); RETEST_LOW:=LOW<=REF(LOW,3)*1.01; VOL_LOWER:=VOLUME<REF(VOLUME,3); AR_HIGH AND RETEST_LOW AND VOL_LOWER
```

### `test_bar` — 测试K线

威科夫测试K线形态

```text
LOW_TEST:=LOW<REF(LOW,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; CLOSE_UP:=CLOSE>REF(CLOSE,1); LOW_TEST AND VOL_LOW AND CLOSE_UP
```

### `three_black_crows` — 三乌鸦

三乌鸦看跌形态

```text
C1:=CLOSE<OPEN AND (OPEN-CLOSE)/OPEN>0.02; C2:=REF(CLOSE,1)<REF(OPEN,1) AND (REF(OPEN,1)-REF(CLOSE,1))/REF(OPEN,1)>0.02; C3:=REF(CLOSE,2)<REF(OPEN,2) AND (REF(OPEN,2)-REF(CLOSE,2))/REF(OPEN,2)>0.02; DOWN_TREND:=CLOSE<REF(CLOSE,1) AND REF(CLOSE,1)<REF(CLOSE,2); C1 AND C2 AND C3 AND DOWN_TREND
```

### `three_white_soldiers` — 三白兵

三白兵看涨形态

```text
C1:=CLOSE>OPEN AND (CLOSE-OPEN)/OPEN>0.02; C2:=REF(CLOSE,1)>REF(OPEN,1) AND (REF(CLOSE,1)-REF(OPEN,1))/REF(OPEN,1)>0.02; C3:=REF(CLOSE,2)>REF(OPEN,2) AND (REF(CLOSE,2)-REF(OPEN,2))/REF(OPEN,2)>0.02; UP_TREND:=CLOSE>REF(CLOSE,1) AND REF(CLOSE,1)>REF(CLOSE,2); C1 AND C2 AND C3 AND UP_TREND
```

### `upthrust_pattern` — 上冲回落

威科夫上冲回落形态

```text
HH:=HHV(HIGH,20); UPTHRUST:=HIGH>HH AND CLOSE<HH; VOL_HIGH:=VOLUME>MA(VOLUME,10); UPTHRUST AND VOL_HIGH
```

### `utad_pattern` — UTAD形态

威科夫终极洗盘

```text
RANGE_HIGH:=HIGH>HHV(HIGH,20); FAIL_HOLD:=CLOSE<REF(HIGH,20); VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; RANGE_HIGH AND FAIL_HOLD AND VOL_HIGH
```

### `wyckoff_accumulation` — 威科夫吸筹

威科夫吸筹区间识别

```text
PS:=VOLUME>MA(VOLUME,20)*2 AND CLOSE>REF(CLOSE,1); SC:=VOLUME>MA(VOLUME,20)*1.5 AND CLOSE<REF(CLOSE,1); AR:=VOLUME>MA(VOLUME,20) AND CLOSE>REF(CLOSE,1); LOW_RANGE:=HHV(HIGH,20)-LLV(LOW,20); LOW_RANGE<MA(LOW_RANGE,60)*0.5
```

### `wyckoff_distribution` — 威科夫派发

威科夫派发区间识别

```text
PSY:=VOLUME>MA(VOLUME,20)*2 AND CLOSE<REF(CLOSE,1); BC:=VOLUME>MA(VOLUME,20)*1.5 AND CLOSE>REF(CLOSE,1); AR:=VOLUME>MA(VOLUME,20) AND CLOSE<REF(CLOSE,1); HIGH_RANGE:=HHV(HIGH,20)-LLV(LOW,20); HIGH_RANGE<MA(HIGH_RANGE,60)*0.5
```


## Strategy

### `add_position` — 加仓信号

趋势确认加仓信号

```text
MA20:=MA(CLOSE,20); TREND_UP:=CLOSE>MA20 AND MA20>REF(MA20,5); PULLBACK:=CLOSE<REF(CLOSE,1); SUPPORT_HOLD:=CLOSE>MA20*0.98; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.8; TREND_UP AND PULLBACK AND SUPPORT_HOLD AND VOL_LOW
```

### `alpha_generation` — Alpha生成

Alpha生成策略

```text
STOCK_RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MARKET_RET:=(INDEX-REF(INDEX,1))/REF(INDEX,1); ALPHA:=STOCK_RET-BETA*MARKET_RET; ALPHA>0.02
```

### `anti_martingale` — 反马丁策略

反马丁格尔策略信号

```text
PROFIT_PCT:=(CLOSE-REF(CLOSE,5))/REF(CLOSE,5)*100; PROFIT_PCT>5 AND VOLUME>MA(VOLUME,10)*1.2
```

### `arbitrage_signal` — 套利信号

统计套利信号

```text
SPREAD:=CLOSE-REF(CLOSE,1)*CORREL(CLOSE,REF(CLOSE,1),N); MEAN_SPREAD:=MA(SPREAD,N); STD_SPREAD:=STD(SPREAD,N); Z_SCORE:=(SPREAD-MEAN_SPREAD)/STD_SPREAD; ABS(Z_SCORE)>2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `beta_neutral` — Beta中性

Beta中性策略

```text
BETA:=CORREL(CLOSE,INDEX,N)*STD(CLOSE,N)*STD(INDEX,N)/VAR(INDEX,N); BETA_HEDGE:=1-BETA; ABS(BETA_HEDGE)<0.2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `boll_rsi_strategy` — 布林带+RSI策略

触及布林下轨且RSI超卖的反弹策略

```text
MID:=MA(CLOSE,20); LOWER:=MID-STD(CLOSE,20)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; CLOSE<LOWER AND RSI<30
```

### `breakout_strategy` — 突破策略

放量突破近期高点的买入策略

```text
HIGH_N:=HHV(HIGH,N); VMA:=MA(VOLUME,M); CROSS(CLOSE,HIGH_N) AND VOLUME>VMA*2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |
| `M` | 3 | 20 | 5 |

### `breakout_strategy_new` — 突破策略增强

增强版突破策略

```text
HH:=HHV(HIGH,N); LL:=LLV(LOW,N); RANGE:=HH-LL; BREAK_UP:=CLOSE>REF(HH,1); VOL_CONFIRM:=VOLUME>MA(VOLUME,20)*1.5; TREND_CONFIRM:=MA(CLOSE,10)>MA(CLOSE,30); BREAK_UP AND VOL_CONFIRM AND TREND_CONFIRM
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `calmar_ratio` — 卡玛比率

卡玛比率评估

```text
RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); ANN_RET:=MA(RETURNS,N)*252; PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK; MAX_DD:=HHV(DRAWDOWN,N); CALMAR:=ANN_RET/MAX_DD; CALMAR>3
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |

### `correlation_trade` — 相关性交易

基于相关性的交易

```text
CORR_VAL:=CORREL(CLOSE,INDEX,N); DIVERGENCE:=CLOSE>MA(CLOSE,20) AND INDEX<MA(INDEX,20); CORR_VAL>0.7 AND DIVERGENCE
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `divergence_strategy` — 背离共振策略

MACD与RSI同时底背离的共振买入策略

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MACD<0 AND REF(MACD,1)<MACD AND RSI<30 AND REF(RSI,1)<RSI
```

### `drawdown_factor` — 回撤因子

最大回撤因子

```text
PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK; MAX_DD:=HHV(DRAWDOWN,N); DD_RANK:=RANK(-MAX_DD,N); DD_RANK>70
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 60 | 30 |

### `dual_momentum` — 双动量策略

Gary Antonacci双动量策略

```text
MOM_12:=CLOSE/REF(CLOSE,12)-1; MOM_6:=CLOSE/REF(CLOSE,6)-1; REL_MOM:=CLOSE/INDEX-1; MOM_12>0 AND MOM_6>0 AND REL_MOM>0
```

### `dual_thrust` — Dual Thrust策略

Dual Thrust区间突破策略

```text
HH:=HHV(HIGH,N); LL:=LLV(LOW,N); HC:=HHV(CLOSE,N); LC:=LLV(CLOSE,N); RANGE:=MAX(HH-LC,HC-LL); UPPER:=OPEN+K1*RANGE; LOWER:=OPEN-K2*RANGE; CLOSE>UPPER
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 10 | 4 |
| `K1` | 0.3 | 0.7 | 0.4 |
| `K2` | 0.3 | 0.7 | 0.4 |

### `expectancy_calc` — 期望值计算

交易期望值计算

```text
WIN_RATE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; AVG_WIN:=SUM(IF(CLOSE>REF(CLOSE,1),CLOSE/REF(CLOSE,1)-1,0),N)/COUNT(CLOSE>REF(CLOSE,1),N); AVG_LOSS:=SUM(IF(CLOSE<REF(CLOSE,1),REF(CLOSE,1)/CLOSE-1,0),N)/COUNT(CLOSE<REF(CLOSE,1),N); EXPECTANCY:=WIN_RATE*AVG_WIN-(1-WIN_RATE)*AVG_LOSS; EXPECTANCY>0
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 60 | 30 |

### `factor_low_vol` — 因子低波

低波动因子策略

```text
VOL_20:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),20); VOL_60:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),60); VOL_RANK:=RANK(VOL_20+VOL_60,60); VOL_RANK<30
```

### `factor_momentum` — 因子动量

因子动量策略

```text
MOM_1M:=(CLOSE-REF(CLOSE,20))/REF(CLOSE,20); MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60); MOM_6M:=(CLOSE-REF(CLOSE,120))/REF(CLOSE,120); FACTOR_MOM:=MOM_1M*0.5+MOM_3M*0.3+MOM_6M*0.2; FACTOR_MOM>0
```

### `golden_triangle` — 黄金三角策略

5日、10日、20日均线形成黄金三角

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20 AND MA5>REF(MA5,1)
```

### `grid_trading` — 网格交易信号

网格交易策略信号

```text
BASE:=MA(CLOSE,N); GRID_PCT:=PCT/100; UPPER1:=BASE*(1+GRID_PCT); LOWER1:=BASE*(1-GRID_PCT); CLOSE<LOWER1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |
| `PCT` | 1 | 5 | 2 |

### `hedge_signal` — 对冲信号

对冲交易信号

```text
BETA:=CORREL(CLOSE,INDEX,N); MARKET_DOWN:=INDEX<MA(INDEX,20); STOCK_UP:=CLOSE>MA(CLOSE,20); DIVERGENCE:=BETA>0.5 AND MARKET_DOWN AND STOCK_UP; DIVERGENCE
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `holding_period` — 持仓周期

持仓周期分析

```text
MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); HOLD_DAYS:=BARSLAST(CROSS(MA5,MA20)); HOLD_DAYS>5 AND HOLD_DAYS<20
```

### `kdj_macd_strategy` — KDJ+MACD策略

KDJ金叉与MACD金叉共振的买入策略

```text
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); CROSS(K,D) AND CROSS(DIF,DEA)
```

### `kelly_criterion` — 凯利公式

凯利公式仓位计算

```text
WIN_RATE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; AVG_WIN:=SUM(IF(CLOSE>REF(CLOSE,1),CLOSE/REF(CLOSE,1)-1,0),N)/COUNT(CLOSE>REF(CLOSE,1),N); AVG_LOSS:=SUM(IF(CLOSE<REF(CLOSE,1),REF(CLOSE,1)/CLOSE-1,0),N)/COUNT(CLOSE<REF(CLOSE,1),N); KELLY:=WIN_RATE-(1-WIN_RATE)/AVG_LOSS*AVG_WIN; KELLY>0.2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 60 | 30 |

### `kurtosis_factor` — 峰度因子

收益分布峰度因子

```text
RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MEAN_RET:=MA(RET,N); STD_RET:=STD(RET,N); KURT:=SUM((RET-MEAN_RET)^4,N)/(N*STD_RET^4)-3; KURT>3
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 60 | 30 |

### `left_transaction` — 左侧交易

左侧交易信号

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; BOTTOM_ZONE:=RSI<30; VOL_SHRINK:=VOLUME<MA(VOLUME,10)*0.7; BOTTOM_ZONE AND VOL_SHRINK
```

### `liquidity_factor` — 流动性因子

流动性因子策略

```text
TURNOVER:=VOLUME/CAPITAL; AMIHUD:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1))/AMOUNT; LIQUIDITY_RANK:=RANK(TURNOVER)-RANK(AMIHUD); LIQUIDITY_RANK>50
```

### `loss_streak` — 连亏统计

连续亏损统计

```text
DOWN_DAYS:=COUNT(CLOSE<REF(CLOSE,1),N); STREAK:=DOWN_DAYS/N*100; STREAK>60
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |

### `low_vol_factor` — 低波动因子

低波动因子选股

```text
VOLATILITY:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N)*SQRT(252); VOLATILITY<0.3
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |

### `ma_macd_strategy` — 均线+MACD策略

均线多头排列且MACD金叉的综合买入策略

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MA5>MA10 AND MA10>MA20 AND CROSS(DIF,DEA)
```

### `ma_pullback` — 均线回踩策略

上升趋势中回踩均线获得支撑

```text
MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); MA20>REF(MA20,1) AND CLOSE<MA5 AND CLOSE>MA20
```

### `ma_volume_strategy` — 均线+成交量策略

均线金叉且成交量放大的确认策略

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MAVOL:=MA(VOLUME,5); CROSS(MA5,MA10) AND VOLUME>MAVOL*1.5
```

### `martingale_strategy` — 马丁策略

马丁格尔策略信号

```text
LOSS_PCT:=(REF(CLOSE,5)-CLOSE)/REF(CLOSE,5)*100; LOSS_PCT>10 AND VOLUME>MA(VOLUME,10)*1.5
```

### `max_drawdown` — 最大回撤

最大回撤监控

```text
PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK*100; MAX_DD:=HHV(DRAWDOWN,N); MAX_DD<20
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |

### `max_sharpe` — 最大夏普

最大夏普比率组合

```text
RET:=MA((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); VOL:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); SHARPE:=RET/VOL; SHARPE>1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `mean_reversion` — 均值回归策略

均值回归策略

```text
MA20:=MA(CLOSE,20); STD20:=STD(CLOSE,20); UPPER:=MA20+STD20*2; LOWER:=MA20-STD20*2; CLOSE<LOWER
```

### `min_variance` — 最小方差

最小方差组合

```text
VAR_STOCK:=VAR((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); VAR_MARKET:=VAR((INDEX-REF(INDEX,1))/REF(INDEX,1),N); VAR_STOCK<VAR_MARKET*0.8
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `momentum_breakout` — 动量突破策略

动量突破交易策略

```text
MOM:=CLOSE-REF(CLOSE,N); MOM_MA:=MA(MOM,M); HH:=HHV(HIGH,N); MOM>0 AND MOM>MOM_MA AND CLOSE>REF(HH,1)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |
| `M` | 3 | 10 | 5 |

### `momentum_factor` — 动量因子

动量因子选股

```text
MOM_12M:=(CLOSE-REF(CLOSE,250))/REF(CLOSE,250)*100; MOM_6M:=(CLOSE-REF(CLOSE,125))/REF(CLOSE,125)*100; MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60)*100; MOM_12M>0 AND MOM_6M>0 AND MOM_3M>0
```

### `momentum_factor_new` — 动量因子增强

增强版动量因子

```text
MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60); MOM_6M:=(CLOSE-REF(CLOSE,120))/REF(CLOSE,120); MOM_12M:=(CLOSE-REF(CLOSE,240))/REF(CLOSE,240); MOM_COMPOSITE:=MOM_3M*0.5+MOM_6M*0.3+MOM_12M*0.2; MOM_RANK:=RANK(MOM_COMPOSITE,240); MOM_RANK>70
```

### `pair_ratio` — 配对比率

配对交易比率分析

```text
RATIO:=CLOSE/REF(CLOSE,1); RATIO_MA:=MA(RATIO,N); RATIO_STD:=STD(RATIO,N); Z_SCORE:=(RATIO-RATIO_MA)/RATIO_STD; ABS(Z_SCORE)>2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `pair_trading_signal` — 配对交易信号

配对交易均值回归信号

```text
SPREAD:=CLOSE/REF(CLOSE,1)-MA(CLOSE/REF(CLOSE,1),N); STD_SPREAD:=STD(SPREAD,N); Z_SCORE:=SPREAD/STD_SPREAD; Z_SCORE<-2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `partial_profit` — 部分止盈

部分仓位止盈信号

```text
PROFIT_PCT:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; PARTIAL_PROFIT:=PROFIT_PCT>10 AND PROFIT_PCT<20; VOL_CONFIRM:=VOLUME>MA(VOLUME,5); PARTIAL_PROFIT AND VOL_CONFIRM
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |

### `position_management` — 仓位管理信号

基于趋势强度的仓位管理

```text
MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); STRONG:=MA5>MA20 AND MA20>MA60; TREND_UP:=MA5>REF(MA5,1); STRONG AND TREND_UP
```

### `position_sizing` — 仓位计算

基于波动率的仓位计算

```text
ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),14); RISK_PCT:=2; POSITION_SIZE:=RISK_PCT/100/(ATR/CLOSE*100); POSITION_SIZE>0.02
```

### `profit_taking_signal` — 止盈信号

动态止盈信号

```text
MA20:=MA(CLOSE,20); PROFIT_PCT:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; PROFIT_PCT>15 AND CLOSE<MA20
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |

### `pyramid_buy` — 金字塔买入

金字塔加仓策略

```text
MA20:=MA(CLOSE,20); TREND_UP:=CLOSE>MA20 AND MA20>REF(MA20,5); PULLBACK:=CLOSE<REF(CLOSE,1); SUPPORT_HOLD:=CLOSE>MA20*0.98; TREND_UP AND PULLBACK AND SUPPORT_HOLD
```

### `r_breaker` — R-Breaker策略

R-Breaker反转突破策略

```text
HH:=REF(HHV(HIGH,N),1); LL:=REF(LLV(LOW,N),1); CC:=REF(CLOSE,1); OBSERVE:=HH-LL; B_BREAK:=HH+OBSERVE*0.1; S_SETUP:=HH-OBSERVE*0.2; B_SETUP:=LL+OBSERVE*0.2; S_BREAK:=LL-OBSERVE*0.1; CLOSE>B_BREAK
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 10 | 4 |

### `reduce_position` — 减仓信号

风险控制减仓信号

```text
MA20:=MA(CLOSE,20); TREND_WEAK:=CLOSE<MA20*1.02; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; PROFIT_TAKEN:=(CLOSE-REF(CLOSE,10))/REF(CLOSE,10)*100>15; TREND_WEAK AND VOL_HIGH AND PROFIT_TAKEN
```

### `reentry_signal` — 重新入场信号

离场后重新入场信号

```text
MA20:=MA(CLOSE,20); EXIT:=CROSS(MA20,CLOSE); RECENT_EXIT:=EXIT OR REF(EXIT,1) OR REF(EXIT,2); RE_ENTRY:=CROSS(CLOSE,MA20); RECENT_EXIT AND RE_ENTRY
```

### `reversal_catch` — 反转捕捉策略

捕捉价格反转机会

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; RSI<30 AND CLOSE>REF(CLOSE,1) AND VOLUME>MA(VOLUME,5)
```

### `reversal_factor` — 反转因子

短期反转因子

```text
SHORT_RET:=(CLOSE-REF(CLOSE,5))/REF(CLOSE,5); REVERSAL_RANK:=RANK(-SHORT_RET,60); REVERSAL_RANK>70
```

### `reversal_strategy_new` — 反转策略增强

增强版反转策略

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; OVERSOLD:=RSI<30; VOL_SHRINK:=VOLUME<MA(VOLUME,20)*0.5; BOUNCE:=CLOSE>REF(CLOSE,1); OVERSOLD AND VOL_SHRINK AND BOUNCE
```

### `right_transaction` — 右侧交易

右侧交易信号

```text
MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); CONFIRM_UP:=CROSS(MA5,MA20); VOL_CONFIRM:=VOLUME>MA(VOLUME,5); CONFIRM_UP AND VOL_CONFIRM
```

### `risk_control_signal` — 风控信号

风险控制预警信号

```text
MA20:=MA(CLOSE,20); ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),14); DRAWDOWN:=(HHV(CLOSE,20)-CLOSE)/HHV(CLOSE,20)*100; DRAWDOWN>ATR/MA20*100*3
```

### `risk_parity` — 风险平价

风险平价配置

```text
VOL:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); RISK_CONTRIBUTION:=1/VOL; WEIGHT:=RISK_CONTRIBUTION/SUM(RISK_CONTRIBUTION,N); WEIGHT>0.1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `risk_reward_ratio` — 风险收益比

风险收益比评估

```text
ENTRY:=CLOSE; STOP:=LLV(LOW,10); TARGET:=HHV(HIGH,20); RISK:=ENTRY-STOP; REWARD:=TARGET-ENTRY; RR_RATIO:=REWARD/RISK; RR_RATIO>2
```

### `rsi_volume_strategy` — RSI+成交量策略

RSI超卖且放量反弹的买入策略

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MAVOL:=MA(VOLUME,5); RSI<30 AND VOLUME>MAVOL*1.5 AND CLOSE>REF(CLOSE,1)
```

### `scalping_signal` — 日内短线信号

日内短线交易信号

```text
MA_FAST:=MA(CLOSE,5); MA_SLOW:=MA(CLOSE,20); VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; TREND:=MA_FAST>MA_SLOW; CROSS(MA_FAST,MA_SLOW) AND VOL_UP
```

### `sector_rotation` — 板块轮动

板块轮动信号

```text
SECTOR_MA:=MA(CLOSE,20); MARKET_MA:=MA(INDEX,20); REL_STRENGTH:=SECTOR_MA/MARKET_MA; REL_STRENGTH>REF(REL_STRENGTH,5)*1.05
```

### `sentiment_factor` — 情绪因子

市场情绪因子

```text
ADVANCE_DECLINE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; VOL_RATIO:=VOLUME/MA(VOLUME,N); SENTIMENT:=ADVANCE_DECLINE*VOL_RATIO; SENTIMENT>1.2
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |

### `sharpe_signal` — 夏普比率信号

基于夏普比率的信号

```text
RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); AVG_RET:=MA(RETURNS,N); STD_RET:=STD(RETURNS,N); SHARPE:=AVG_RET/STD_RET; SHARPE>1
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `skewness_factor` — 偏度因子

收益分布偏度因子

```text
RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MEAN_RET:=MA(RET,N); STD_RET:=STD(RET,N); SKEW:=SUM((RET-MEAN_RET)^3,N)/(N*STD_RET^3); SKEW<-0.5
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 20 | 60 | 30 |

### `sortino_signal` — 索提诺比率信号

基于索提诺比率的信号

```text
RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); AVG_RET:=MA(RETURNS,N); DOWN_RET:=IF(RETURNS<0,RETURNS,0); DOWN_STD:=STD(DOWN_RET,N); SORTINO:=AVG_RET/DOWN_STD; SORTINO>1.5
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `stop_loss_signal` — 止损信号

动态止损信号

```text
MA20:=MA(CLOSE,20); LOSS_PCT:=(REF(CLOSE,N)-CLOSE)/REF(CLOSE,N)*100; LOSS_PCT>8 AND CLOSE<MA20
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 15 | 5 |

### `strat_adx_trend` — ADX趋势

ADX强趋势过滤

```text
ADX(HIGH,LOW,CLOSE,14)>25
```

### `strat_cci_overbought` — CCI超买

CCI超买超卖

```text
CCI(HIGH,LOW,CLOSE,14)>100
```

### `strat_chaikin_vol` — 佳庆波动

佳庆波动扩张

```text
EMA(HIGH-LOW,10)-EMA(HIGH-LOW,20)>0
```

### `strat_choppiness` — 趋势强度

趋势强度判断

```text
CHOP(HIGH,LOW,CLOSE,14)<61.8
```

### `strat_coppock` — 估波指标

估波指标底部

```text
WMA(ROC(CLOSE,14)+ROC(CLOSE,11),10)>0
```

### `strat_donchian` — 唐奇安通道

唐奇安突破

```text
CLOSE=HHV(HIGH,20)
```

### `strat_elder_ray` — 老鹰射线

老鹰射线多头

```text
HIGH-EMA(CLOSE,13)>0
```

### `strat_fisher` — 费舍尔变换

费舍尔变换多头

```text
FISHER(HIGH,LOW,9)>0
```

### `strat_force_index` — 力度指数

力度指数多头

```text
EMA((CLOSE-REF(CLOSE,1))*VOLUME,13)>0
```

### `strat_ichimoku` — 一目均衡策略

一目均衡表交叉

```text
TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; CROSS(TENKAN,KIJUN)
```

### `strat_keltner` — 肯特纳通道

肯特纳突破

```text
TYP:=(HIGH+LOW+CLOSE)/3; ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),10); CLOSE>MA(TYP,10)+ATR*2
```

### `strat_mass_index` — 质量指数

质量指数反转

```text
SUM(EMA(HIGH-LOW,9)/EMA(EMA(HIGH-LOW,9),9),25)>27
```

### `strat_mfi_divergence` — MFI背离

MFI超卖

```text
MFI(HIGH,LOW,CLOSE,VOLUME,14)<20
```

### `strat_squeeze` — 挤压动量

TTM挤压

```text
AVG:=SMA(CLOSE,20); MID:=(HHV(HIGH,20)+LLV(LOW,20))/2; CLOSE-(AVG+MID)/2>0
```

### `strat_stc` — Schaff趋势周期

STC超买

```text
MACD_LINE:=EMA(CLOSE,23)-EMA(CLOSE,50); K1:=100*(MACD_LINE-LLV(MACD_LINE,25))/(HHV(MACD_LINE,25)-LLV(MACD_LINE,25)); D1:=EMA(K1,3); K2:=100*(D1-LLV(D1,25))/(HHV(D1,25)-LLV(D1,25)); STC_LINE:=EMA(K2,3); STC_LINE>80
```

### `strat_supertrend` — 超级趋势策略

超级趋势跟踪

```text
CLOSE>SUPERTREND(HIGH,LOW,CLOSE,10,3)
```

### `strat_tsi` — 真实强度

真实强度指数多头

```text
TSI(CLOSE,25,13)>0
```

### `strat_ultimate_osc` — 终极振荡

终极振荡超买

```text
ULTOSC(HIGH,LOW,CLOSE,7,14,28)>70
```

### `strat_vortex` — 涡旋指标

涡旋交叉

```text
TR1:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); VIP:=SUM(ABS(HIGH-REF(LOW,1)),14)/SUM(TR1,14); VIM:=SUM(ABS(LOW-REF(HIGH,1)),14)/SUM(TR1,14); CROSS(VIP,VIM)
```

### `strat_vwap_revert` — VWAP回归

VWAP均值回归

```text
CLOSE<VWAP(CLOSE,VOLUME)
```

### `strat_willr_extreme` — 威廉极值

威廉指标极值

```text
WILLR(HIGH,LOW,CLOSE,14)<-80
```

### `swing_trade_signal` — 波段交易信号

波段交易入场信号

```text
MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MA20>MA60 AND RSI<40 AND CLOSE>MA20
```

### `technical_factor` — 技术因子

技术分析因子

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MACD_HIST:=MACD(12,26,9); TECH_SCORE:=RANK(RSI,14)+RANK(MACD_HIST,26); TECH_SCORE>100
```

### `time_exit` — 时间离场

基于时间的离场策略

```text
HOLD_DAYS:=BARSLAST(CROSS(MA(CLOSE,5),MA(CLOSE,10))); HOLD_DAYS>N AND CLOSE<MA(CLOSE,5)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |

### `trade_frequency` — 交易频率

交易频率控制

```text
MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); SIGNAL:=CROSS(MA5,MA20); SIGNAL_COUNT:=COUNT(SIGNAL,N); SIGNAL_COUNT<N*0.3
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |

### `trailing_stop` — 移动止损

追踪止损策略

```text
HH:=HHV(HIGH,N); TRAIL_STOP:=HH*(1-PCT/100); CLOSE<TRAIL_STOP
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |
| `PCT` | 3 | 15 | 8 |

### `trend_following` — 趋势跟踪策略

经典趋势跟踪策略

```text
MA_FAST:=MA(CLOSE,50); MA_SLOW:=MA(CLOSE,200); VOL_FILTER:=VOLUME>MA(VOLUME,50); CROSS(MA_FAST,MA_SLOW) AND VOL_FILTER
```

### `trend_reversal` — 趋势反转策略

MACD底背离加KDJ超卖的底部反转策略

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); J:=3*K-2*SMA(K,3,1); MACD<0 AND REF(MACD,1)<MACD AND J<20
```

### `turtle_breakout` — 海龟交易突破

海龟交易法则突破策略

```text
HH:=HHV(HIGH,N); LL:=LLV(LOW,N); BREAK_UP:=CLOSE>REF(HH,1); BREAK_DOWN:=CLOSE<REF(LL,1); BREAK_UP
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `turtle_exit` — 海龟交易离场

海龟交易法则离场策略

```text
LL:=LLV(LOW,N); EXIT:=CLOSE<REF(LL,1); EXIT
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 10 |

### `twap_strategy` — TWAP交易策略

时间加权均价交易策略

```text
TWAP:=MA(CLOSE,N); CROSS(CLOSE,TWAP) AND VOLUME>MA(VOLUME,5)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 20 |

### `volatility_breakout` — 波动率突破

基于波动率的突破策略

```text
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); RANGE:=ATR*MULT; UPPER:=REF(CLOSE,1)+RANGE; LOWER:=REF(CLOSE,1)-RANGE; CLOSE>UPPER
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 14 |
| `MULT` | 1 | 3 | 2 |

### `volume_factor` — 成交量因子

成交量分析因子

```text
VOL_RATIO:=VOLUME/MA(VOLUME,20); VOL_TREND:=MA(VOLUME,5)/MA(VOLUME,20); VOL_SCORE:=RANK(VOL_RATIO,20)+RANK(VOL_TREND,20); VOL_SCORE>100
```

### `vwap_strategy` — VWAP交易策略

成交量加权均价交易策略

```text
VWAP:=SUM(AMOUNT,N)/SUM(VOLUME,N); CROSS(CLOSE,VWAP) AND VOLUME>MA(VOLUME,5)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 20 |

### `win_streak` — 连胜统计

连续盈利统计

```text
UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),N); STREAK:=UP_DAYS/N*100; STREAK>60
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |


## TDXClassic

### `tdx_bklh` — 板块联动BKLH

通达信板块联动分析，识别板块内领涨股

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); STRONG:=MA5>MA10 AND MA10>REF(MA10,1); LEAD:=ZF>3 AND HSL>5; STRONG AND LEAD
```

### `tdx_cyfp` — 筹码分布CYFP

通达信筹码分布分析，计算不同价位筹码占比

```text
WINNER10:=WINNER(CLOSE*0.9)*100; WINNER20:=WINNER(CLOSE*0.95)*100; WINNER50:=WINNER(CLOSE)*100; WINNER80:=WINNER(CLOSE*1.05)*100; CHIP_DENSITY:=WINNER80-WINNER10; CHIP_DENSITY>50 AND WINNER50>30
```

### `tdx_cyqkl` — 筹码峰指标CYQKL

通达信筹码分布指标，分析筹码集中度和获利比例

```text
CYQKL:=(WINNER(CLOSE*1.1)-WINNER(CLOSE*0.9))*100; COST90:=COST(90); COST10:=COST(10); CONCENTR:=(COST90-COST10)/(COST90+COST10)*100; CYQKL>60 AND CONCENTR<30
```

### `tdx_dtct` — 跌停捕捉DTCT

通达信跌停板捕捉，识别可能跌停的弱势股风险

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); WEAK:=MA5<MA10 AND MA10<MA20; VOL_UP:=VOLUME>MA(VOLUME,5)*2; ZF<-5 AND WEAK AND VOL_UP
```

### `tdx_flzt` — 分时涨停FLZT

通达信分时涨停预警，盘中实时监控涨停概率

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; VOL_RATIO:=VOLUME/MA(VOLUME,5); BUY_RATIO:=IF(CLOSE>REF(CLOSE,1),VOLUME,0)/VOLUME; ZF>6 AND HSL>5 AND VOL_RATIO>2 AND BUY_RATIO>0.6
```

### `tdx_hpdr` — 横盘突破HPDR

通达信横盘突破识别，捕捉整理后的突破机会

```text
HHV_N:=HHV(HIGH,N); LLV_N:=LLV(LOW,N); RANGE_PCT:=(HHV_N-LLV_N)/LLV_N*100; VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; BREAK_OUT:=CLOSE>HHV_N*0.98; RANGE_PCT<15 AND VOL_UP AND BREAK_OUT
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `tdx_jcmm` — 进出明细JCMM

通达信资金进出明细，分析大单买卖方向

```text
BIG_VOL:=VOLUME>MA(VOLUME,20)*2; BIG_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,VOLUME,0); BIG_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,VOLUME,0); NET_BIG:=SUM(BIG_BUY-BIG_SELL,5); NET_BIG>0
```

### `tdx_jdcs` — 阶段涨幅JDCS

通达信阶段涨幅统计，计算N日累计涨幅

```text
ZF_N:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,N); MA_ZF:=MA((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); ZF_N>M*MA_ZF
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 60 | 20 |
| `M` | 1 | 3 | 1.5 |

### `tdx_lhb` — 龙虎榜指标LHB

通达信龙虎榜追踪，监控机构与游资动向

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; BIGBUY:=IF(VOLUME>MA(VOLUME,20)*3,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); BIGSELL:=IF(VOLUME>MA(VOLUME,20)*3,IF(CLOSE<REF(CLOSE,1),VOLUME,0),0); NET:=SUM(BIGBUY-BIGSELL,5); ZF>7 AND HSL>10 AND NET>0
```

### `tdx_qsgl` — 强势股筛选QSGL

通达信强势股筛选，识别连续上涨的强势股票

```text
UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),5); ZF_SUM:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); UP_DAYS>=4 AND ZF_SUM>10 AND MA5>MA10 AND MA10>MA20 AND CLOSE>MA5
```

### `tdx_rsgl` — 弱势股筛选RSGL

通达信弱势股筛选，识别连续下跌的弱势股票

```text
DOWN_DAYS:=COUNT(CLOSE<REF(CLOSE,1),5); ZF_SUM:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); DOWN_DAYS>=4 AND ZF_SUM<-10 AND MA5<MA10 AND MA10<MA20 AND CLOSE<MA5
```

### `tdx_sxbd` — 双响炮SXD

通达信双响炮形态，连续涨停后的回调再启动

```text
ZT1:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100-9.9)<0.5; ZT_DAYS:=COUNT(ZT1,10); ADJ_DAYS:=COUNT(CLOSE<REF(CLOSE,1),3); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ZT_DAYS>=2 AND ADJ_DAYS<=2 AND CROSS(MA5,MA10)
```

### `tdx_zjtj` — 主力统计ZJTJ

通达信主力资金统计，追踪主力买卖行为

```text
VAR1:=IF(CLOSE>REF(CLOSE,1),VOLUME,IF(CLOSE<REF(CLOSE,1),-VOLUME,0)); MAIN_FORCE:=SUM(VAR1,5); MAIN_MA:=MA(MAIN_FORCE,10); CROSS(MAIN_FORCE,MAIN_MA)
```

### `tdx_zlcp` — 主力成本ZLCP

通达信主力成本分析，计算主力平均持仓成本

```text
AVG_COST:=SUM(AMOUNT,N)/SUM(VOLUME,N); COST_DIFF:=(CLOSE-AVG_COST)/AVG_COST*100; COST_DIFF>-5 AND COST_DIFF<5 AND VOLUME>MA(VOLUME,5)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `tdx_ztct` — 涨停捕捉ZTCT

通达信涨停板捕捉，识别即将涨停的强势股

```text
ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); STRONG:=MA5>MA10 AND MA10>MA20; VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; ZF>5 AND ZF<9.5 AND STRONG AND VOL_UP AND HSL>3
```


## THSSmartSelect

### `ths_boll_support` — 布林支撑选股

同花顺布林带下轨支撑选股，触及下轨后反弹

```text
MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; TOUCH_LOWER:=REF(LOW,1)<LOWER OR REF(LOW,2)<LOWER; REBOUND:=CLOSE>REF(CLOSE,1); VOL_UP:=VOLUME>MA(VOLUME,5); TOUCH_LOWER AND REBOUND AND VOL_UP
```

### `ths_breakout` — 突破选股

同花顺突破选股，放量突破前期高点

```text
HHV_N:=HHV(HIGH,N); VOL_MA:=MA(VOLUME,M); BREAK_PRICE:=CLOSE>REF(HHV_N,1); BREAK_VOL:=VOLUME>REF(VOL_MA,1)*1.5; TREND_UP:=MA(CLOSE,5)>MA(CLOSE,10); BREAK_PRICE AND BREAK_VOL AND TREND_UP
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |
| `M` | 3 | 20 | 5 |

### `ths_golden_cross` — 三线金叉选股

同花顺三线金叉选股，均线、成交量、MACD同时金叉

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA_CROSS:=CROSS(MA5,MA10); V5:=MA(VOLUME,5); V10:=MA(VOLUME,10); VOL_CROSS:=CROSS(V5,V10); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD_CROSS:=CROSS(DIF,DEA); MA_CROSS AND VOL_CROSS AND MACD_CROSS
```

### `ths_kdj_oversold` — KDJ超卖选股

同花顺KDJ超卖选股，J值低于20且出现金叉

```text
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J<20 AND CROSS(K,D)
```

### `ths_ma_bullish` — 均线多头排列选股

同花顺均线多头排列选股，短中长期均线依次排列

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); MA120:=MA(CLOSE,120); BULLISH:=MA5>MA10 AND MA10>MA20 AND MA20>MA60; TREND_UP:=MA5>REF(MA5,1) AND MA10>REF(MA10,1); BULLISH AND TREND_UP AND CLOSE>MA5
```

### `ths_macd_select` — MACD金叉选股

同花顺MACD金叉智能选股，DIF上穿DEA且在零轴上方

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; ZERO_ABOVE:=DIF>0 AND DEA>0; GOLDEN_CROSS:=CROSS(DIF,DEA); ZERO_ABOVE AND GOLDEN_CROSS
```

### `ths_rsi_rebound` — RSI反弹选股

同花顺RSI超卖反弹选股，RSI从超卖区回升

```text
RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; RSI_LOW:=REF(RSI,1)<30; RSI_UP:=RSI>REF(RSI,1); VOL_UP:=VOLUME>MA(VOLUME,5); RSI_LOW AND RSI_UP AND VOL_UP
```

### `ths_strong_pullback` — 强势回踩选股

同花顺强势股回踩选股，强势股回调至支撑位

```text
MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); STRONG:=MA5>MA10 AND MA10>MA20 AND MA20>REF(MA20,1); PULLBACK:=CLOSE<MA5 AND CLOSE>MA10; VOL_SHRINK:=VOLUME<MA(VOLUME,5); STRONG AND PULLBACK AND VOL_SHRINK
```

### `ths_vol_price` — 量价齐升选股

同花顺量价齐升选股，价涨量增确认趋势

```text
PRICE_UP:=CLOSE>REF(CLOSE,1) AND REF(CLOSE,1)>REF(CLOSE,2); VOL_UP:=VOLUME>REF(VOLUME,1) AND REF(VOLUME,1)>REF(VOLUME,2); MA5_UP:=MA(CLOSE,5)>REF(MA(CLOSE,5),1); MA10_UP:=MA(CLOSE,10)>REF(MA(CLOSE,10),1); PRICE_UP AND VOL_UP AND MA5_UP AND MA10_UP
```

### `ths_volume_break` — 放量突破选股

同花顺放量突破选股，成交量显著放大突破

```text
VOL_MA5:=MA(VOLUME,5); VOL_MA10:=MA(VOLUME,10); VOL_RATIO:=VOLUME/VOL_MA5; PRICE_BREAK:=CLOSE>HHV(HIGH,20); VOL_BREAK:=VOL_RATIO>2 AND VOLUME>VOL_MA10; TREND_UP:=MA(CLOSE,5)>MA(CLOSE,10); PRICE_BREAK AND VOL_BREAK AND TREND_UP
```


## Trend

### `adx_trend` — ADX趋势强度

平均方向性指数判断趋势强度

```text
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); DMP:=SUM(IF(HIGH>REF(HIGH,1) AND HIGH-REF(HIGH,1)>REF(LOW,1)-LOW,MAX(HIGH-REF(HIGH,1),HIGH-REF(HIGH,1)),0),N); DMM:=SUM(IF(LOW<REF(LOW,1) AND REF(LOW,1)-LOW>REF(HIGH,1)-HIGH,MAX(REF(LOW,1)-LOW,REF(HIGH,1)-HIGH),0),N); DI1:=DMP/TR*N*100; DI2:=DMM/TR*N*100; ADX:=MA(ABS(DI1-DI2)/(DI1+DI2)*100,M); ADX>25
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 7 | 21 | 14 |
| `M` | 3 | 12 | 6 |

### `alligator_trend` — 鳄鱼线趋势

鳄鱼线趋势判断

```text
JAW:=MA((HIGH+LOW)/2,13); TEETH:=MA((HIGH+LOW)/2,8); LIPS:=MA((HIGH+LOW)/2,5); CLOSE>JAW AND CLOSE>TEETH AND CLOSE>LIPS
```

### `dma_difference` — DMA平行线差

平行线差指标，中长期趋势判断

```text
DIF:=MA(CLOSE,SHORT)-MA(CLOSE,LONG); AMA:=MA(DIF,M); CROSS(DIF,AMA)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 5 | 20 | 10 |
| `LONG` | 20 | 60 | 50 |
| `M` | 3 | 10 | 6 |

### `dmi_trend` — DMI趋向指标

上升下降方向线判断趋势

```text
MTR:=SUM(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),N); HD:=HIGH-REF(HIGH,1); LD:=REF(LOW,1)-LOW; DMP:=SUM(IF(HD>0 AND HD>LD,HD,0),N); DMM:=SUM(IF(LD>0 AND LD>HD,LD,0),N); PDI:=DMP/MTR*100; MDI:=DMM/MTR*100; PDI>MDI
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 7 | 28 | 14 |

### `donchian_breakout` — 唐奇安通道突破

唐奇安通道突破策略

```text
UPPER:=HHV(HIGH,N); LOWER:=LLV(LOW,N); MID:=(UPPER+LOWER)/2; CROSS(CLOSE,UPPER)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `gator_oscillator` — Gator鳄鱼线

鳄鱼线震荡指标，趋势休眠与活跃

```text
JAW:=MA((HIGH+LOW)/2,13); TEETH:=MA((HIGH+LOW)/2,8); LIPS:=MA((HIGH+LOW)/2,5); GATOR_UP:=JAW-TEETH; GATOR_DOWN:=TEETH-LIPS; GATOR_UP>0 AND GATOR_DOWN>0
```

### `heikinashi_trend` — 平均K线趋势

Heikin-Ashi平均K线趋势

```text
HA_CLOSE:=(OPEN+HIGH+LOW+CLOSE)/4; HA_OPEN:=EMA(HA_CLOSE,3); HA_OPEN<HA_CLOSE
```

### `ichimoku_cloud` — 云图穿越

一目均衡表云图穿越

```text
TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; SENKOU_A:=(TENKAN+KIJUN)/2; SENKOU_B:=(HHV(HIGH,52)+LLV(LOW,52))/2; CLOSE>SENKOU_A AND CLOSE>SENKOU_B
```

### `ichimoku_signal` — 一目均衡信号

一目均衡图转换线与基准线交叉

```text
TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; CROSS(TENKAN,KIJUN)
```

### `macd_death_cross` — MACD死叉

DIF下穿DEA形成MACD死叉卖出信号

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; CROSS(DEA,DIF)
```

### `macd_divergence_bottom` — MACD底背离

股价创新低但MACD没有创新低，看涨背离

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; REF(MACD,1)<MACD AND MACD<0
```

### `macd_golden_cross` — MACD金叉

DIF上穿DEA形成MACD金叉买入信号

```text
DIF:=EMA(CLOSE,SHORT)-EMA(CLOSE,LONG); DEA:=EMA(DIF,M); MACD:=(DIF-DEA)*2; CROSS(DIF,DEA)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `SHORT` | 6 | 24 | 12 |
| `LONG` | 12 | 60 | 26 |
| `M` | 4 | 20 | 9 |

### `macd_red_green` — MACD红绿柱

MACD柱状图红绿柱变化

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); (DIF-DEA)*2
```

### `macd_zero_cross` — MACD零轴穿越

DIF穿越零轴的趋势确认信号

```text
DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); CROSS(DIF,0)
```

### `markdown_phase` — 下跌阶段

威科夫下跌阶段识别

```text
MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); DOWN_TREND:=MA20<MA60 AND MA60<REF(MA60,5); VOL_INCREASE:=MA(VOLUME,5)>MA(VOLUME,20); DOWN_TREND AND VOL_INCREASE
```

### `markup_phase` — 上涨阶段

威科夫上涨阶段识别

```text
MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); UP_TREND:=MA20>MA60 AND MA60>REF(MA60,5); VOL_INCREASE:=MA(VOLUME,5)>MA(VOLUME,20); UP_TREND AND VOL_INCREASE
```

### `sar_trend` — SAR抛物线趋势

抛物线转向指标判断趋势方向

```text
SAR_VAL:=SAR(HIGH,LOW,STEP,MAXSTEP); CLOSE>SAR_VAL
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 1 | 10 | 4 |
| `STEP` | 0.01 | 0.05 | 0.02 |
| `MAXSTEP` | 0.1 | 0.3 | 0.2 |

### `supertrend` — 超级趋势

基于ATR的趋势跟踪指标

```text
TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); MID:=MA(CLOSE,N); UPPER:=MID+ATR*MULT; LOWER:=MID-ATR*MULT; CROSS(CLOSE,LOWER)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 7 | 21 | 10 |
| `MULT` | 1 | 4 | 3 |

### `trend_strength` — 趋势强度

基于均线斜率的趋势强度指标

```text
MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); (MA5-REF(MA5,1))/MA5*100
```

### `vortex_indicator` — VI涡旋指标

涡旋指标，识别趋势方向

```text
VM_PLUS:=ABS(HIGH-REF(LOW,1)); VM_MINUS:=ABS(LOW-REF(HIGH,1)); TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); VI_PLUS:=SUM(VM_PLUS,N)/SUM(TR,N); VI_MINUS:=SUM(VM_MINUS,N)/SUM(TR,N); CROSS(VI_PLUS,VI_MINUS)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 14 |

### `zigzag_trend` — 之字转向

之字转向指标，识别趋势转折点

```text
HH:=HHV(HIGH,N); LL:=LLV(LOW,N); PCT_CHANGE:=(HH-LL)/LL*100; TREND_UP:=CLOSE>REF(HH,1)*0.95; PCT_CHANGE>THRESHOLD AND TREND_UP
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |
| `THRESHOLD` | 3 | 10 | 5 |


## Volume

### `adl_accumulation` — ADL累积派发

累积派发线，资金流向分析

```text
MFM:=((CLOSE-LOW)-(HIGH-CLOSE))/(HIGH-LOW); MFV:=MFM*VOLUME; ADL:=SUM(MFV,N); MA_ADL:=MA(ADL,M); CROSS(ADL,MA_ADL)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 30 | 14 |
| `M` | 3 | 15 | 5 |

### `churn_bar` — 搅动K线

高量窄幅搅动K线

```text
VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.5; VOL_HIGH AND SPREAD_LOW
```

### `climax_volume` — 高潮量

成交量高潮识别

```text
VOL_EXTREME:=VOLUME>MA(VOLUME,20)*3; PRICE_EXTREME:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100)>5; VOL_EXTREME AND PRICE_EXTREME
```

### `cmf_chaikin` — CMF佳庆资金流

佳庆资金流量指标

```text
MFM:=((CLOSE-LOW)-(HIGH-CLOSE))/(HIGH-LOW); MFV:=MFM*VOLUME; CMF:=SUM(MFV,N)/SUM(VOLUME,N); CMF>0
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 20 |

### `effort_result` — 努力结果分析

威科夫努力与结果分析

```text
EFFORT:=VOLUME/MA(VOLUME,10); RESULT:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; EFFORT_UP:=EFFORT>1.5; RESULT_DOWN:=RESULT<0.5; EFFORT_UP AND RESULT_DOWN
```

### `evm_ease` — EVM简易波动

简易波动指标，量价关系分析

```text
DM:=((HIGH+LOW)/2-(REF(HIGH,1)+REF(LOW,1))/2); BR:=(HIGH-LOW); EVM:=DM/BR*VOLUME; MA_EVM:=MA(EVM,N); CROSS(EVM,MA_EVM)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 14 |

### `fi_force` — FI力度指数

力度指数，价格与成交量综合

```text
FI:=(CLOSE-REF(CLOSE,1))*VOLUME; MA_FI:=MA(FI,N); CROSS(FI,MA_FI)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 13 |

### `mfi_money_flow` — MFI资金流量

资金流量指标，结合价格和成交量

```text
TP:=(HIGH+LOW+CLOSE)/3; MF:=TP*VOLUME; PMF:=SUM(IF(TP>REF(TP,1),MF,0),N); NMF:=SUM(IF(TP<REF(TP,1),MF,0),N); MFI:=PMF/(PMF+NMF)*100; MFI<20
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 14 |

### `no_demand` — 无需求

威科夫无需求形态

```text
PRICE_DOWN:=CLOSE<REF(CLOSE,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.7; PRICE_DOWN AND VOL_LOW AND SPREAD_LOW
```

### `no_supply` — 无供给

威科夫无供给形态

```text
PRICE_UP:=CLOSE>REF(CLOSE,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.7; PRICE_UP AND VOL_LOW AND SPREAD_LOW
```

### `nvi_negative` — NVI负量指标

负量指标，缩量日价格变化

```text
NVI:=IF(VOLUME<REF(VOLUME,1),REF(NVI,1)+(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*REF(NVI,1),REF(NVI,1)); MA_NVI:=MA(NVI,M); CROSS(NVI,MA_NVI)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `M` | 10 | 30 | 20 |

### `obv_trend` — OBV能量潮

On Balance Volume能量潮趋势

```text
OBV:=SUM(IF(CLOSE>REF(CLOSE,1),VOLUME,IF(CLOSE<REF(CLOSE,1),-VOLUME,0)),N); CROSS(OBV,MA(OBV,M))
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |
| `M` | 3 | 20 | 6 |

### `pvi_positive` — PVI正量指标

正量指标，放量日价格变化

```text
PVI_LN:=IF(VOLUME>REF(VOLUME,1),LN(1+(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)),0); PVI:=1000*EXP(CUMSUM(PVI_LN)); MA_PVI:=MA(PVI,M); CROSS(PVI,MA_PVI)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `M` | 10 | 30 | 20 |

### `stopping_volume` — 止损量

威科夫止损量形态

```text
PRICE_DOWN:=CLOSE<REF(CLOSE,1); VOL_HIGH:=VOLUME>MA(VOLUME,10)*2; CLOSE_NEAR_LOW:=CLOSE>LOW+(HIGH-LOW)*0.5; PRICE_DOWN AND VOL_HIGH AND CLOSE_NEAR_LOW
```

### `volatility_volume` — 成交量变异率

成交量的波动程度

```text
MAVOL:=MA(VOLUME,N); STD(VOLUME,N)/MAVOL*100
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 60 | 20 |

### `volume_breakout` — 放量突破

成交量显著放大伴随价格突破

```text
MAVOL:=MA(VOLUME,N); VOLUME>MAVOL*2 AND CLOSE>REF(HHV(HIGH,N),1)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 60 | 20 |

### `volume_ma_cross` — 成交量均线交叉

成交量短期均线上穿长期均线

```text
V5:=MA(VOLUME,5); V10:=MA(VOLUME,10); CROSS(V5,V10)
```

### `volume_price_rise` — 量价齐升

成交量和价格同时上涨

```text
CLOSE>REF(CLOSE,1) AND VOLUME>REF(VOLUME,1)
```

### `volume_profile` — 成交量分布

成交量分布分析

```text
PRICE_LEVEL:=CLOSE; VOL_AT_LEVEL:=SUM(IF(ABS(CLOSE-PRICE_LEVEL)<PRICE_LEVEL*0.01,VOLUME,0),N); TOTAL_VOL:=SUM(VOLUME,N); VOL_PCT:=VOL_AT_LEVEL/TOTAL_VOL*100; VOL_PCT>5
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 20 |

### `volume_ratio` — 量比指标

当前成交量与平均成交量的比值

```text
MAVOL:=MA(VOLUME,N); VOLUME/MAVOL
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 3 | 30 | 5 |

### `volume_shrink_back` — 缩量回调

价格回调但成交量萎缩，支撑有效

```text
CLOSE<REF(CLOSE,1) AND VOLUME<REF(VOLUME,1)
```

### `volume_spread` — 量价差分析

成交量与价差关系分析

```text
SPREAD:=HIGH-LOW; VOL_MA:=MA(VOLUME,N); SPREAD_MA:=MA(SPREAD,N); VOL_UP:=VOLUME>VOL_MA*1.5; SPREAD_DOWN:=SPREAD<SPREAD_MA*0.7; VOL_UP AND SPREAD_DOWN
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 5 | 20 | 10 |

### `vpt_trend` — VPT量价趋势

量价趋势指标，累积成交量变化

```text
VPT:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*VOLUME,N); MA_VPT:=MA(VPT,M); CROSS(VPT,MA_VPT)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 60 | 30 |
| `M` | 3 | 20 | 6 |

### `vr_volume_ratio` — VR容量比率

容量比率指标，量价关系分析

```text
TH:=SUM(IF(CLOSE>REF(CLOSE,1),VOLUME,0),N); TL:=SUM(IF(CLOSE<REF(CLOSE,1),VOLUME,0),N); TQ:=SUM(IF(CLOSE==REF(CLOSE,1),VOLUME,0),N); VR:=(TH+TQ/2)/(TL+TQ/2)*100; VR<70
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 26 |

### `wvad_volume` — WVAD威廉变异

威廉变异离散量，量价分析

```text
WVAD:=SUM((CLOSE-OPEN)/(HIGH-LOW)*VOLUME,N); MA_WVAD:=MA(WVAD,M); CROSS(WVAD,MA_WVAD)
```

| Parameter | Default | Min | Max |
|-----------|---------|-----|-----|
| `N` | 10 | 30 | 24 |
| `M` | 3 | 15 | 6 |
