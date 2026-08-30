# 公式系统参考文档

## 概述

Rust TA-Lib 公式系统提供了一套类似通达信、同花顺等主流股票软件的公式语言，支持技术指标计算、条件判断和信号生成。公式引擎基于表达式求值器，支持数组运算和标量运算。

---

## 运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `+` | 加法 | `CLOSE + OPEN` |
| `-` | 减法 | `HIGH - LOW` |
| `*` | 乘法 | `CLOSE * 2` |
| `/` | 除法 | `VOLUME / 100` |
| `>` | 大于 | `CLOSE > MA(CLOSE, 20)` |
| `<` | 小于 | `VOLUME < REF(VOLUME, 1)` |
| `>=` | 大于等于 | `CLOSE >= OPEN` |
| `<=` | 小于等于 | `HIGH <= REF(HIGH, 1)` |
| `==` 或 `=` | 等于 | `CLOSE == OPEN` |
| `!=` 或 `<>` | 不等于 | `CLOSE != OPEN` |
| `AND` 或 `&&` | 逻辑与 | `CLOSE > MA5 AND VOLUME > 10000` |
| `OR` 或 `||` | 逻辑或 | `CROSS(MA5, MA10) OR CROSS(MA10, MA20)` |
| `NOT` 或 `!` | 逻辑非 | `NOT CROSS(MA5, MA10)` |

---

## 数据变量

| 变量 | 别名 | 说明 |
|------|------|------|
| `OPEN` | `O` | 开盘价 |
| `HIGH` | `H` | 最高价 |
| `LOW` | `L` | 最低价 |
| `CLOSE` | `C` | 收盘价 |
| `VOLUME` | `V` | 成交量 |
| `AMOUNT` | `A` | 成交额 |
| `DATE` | `D` | 日期 |

---

## 内置函数参考

### 移动平均类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `MA(X, N)` | X: 数据序列, N: 周期 | 简单移动平均 | 数组 |
| `EMA(X, N)` | X: 数据序列, N: 周期 | 指数移动平均 | 数组 |
| `SMA(X, N, M)` | X: 数据序列, N: 周期, M: 权重 | 加权移动平均 (Y = (M*X + (N-M)*Y')/N) | 数组 |
| `WMA(X, N)` | X: 数据序列, N: 周期 | 加权移动平均 | 数组 |
| `DEMA(X, N)` | X: 数据序列, N: 周期 | 双指数移动平均 | 数组 |
| `TEMA(X, N)` | X: 数据序列, N: 周期 | 三指数移动平均 | 数组 |
| `KAMA(X, N)` | X: 数据序列, N: 周期 | 考夫曼自适应移动平均 | 数组 |
| `T3(X, N, V)` | X: 数据序列, N: 周期, V: 量因子(0.7) | T3移动平均 | 数组 |
| `MAMA(X, F, S)` | X: 数据序列, F: 快速限制(0.5), S: 慢速限制(0.05) | MESA自适应移动平均 | 数组(MAMA) |
| `FAMA(X, F, S)` | X: 数据序列, F: 快速限制, S: 慢速限制 | 跟随自适应移动平均 | 数组(FAMA) |

### 趋势类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `MACD(X, FAST, SLOW, SIGNAL)` | X: 数据, FAST: 快周期(12), SLOW: 慢周期(26), SIGNAL: 信号周期(9) | MACD指标 | DIF线 |
| `DIFF(X, FAST, SLOW)` | X: 数据, FAST: 快周期, SLOW: 慢周期 | DIF = EMA(FAST) - EMA(SLOW) | 数组 |
| `DEA(X, FAST, SLOW, SIGNAL)` | X: 数据, FAST, SLOW, SIGNAL | MACD信号线 | 数组 |
| `ADX(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 平均趋向指数 | 数组 |
| `ADXR(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 平均趋向指数评估 | 数组 |
| `PLUS_DI(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 上升方向指标 | 数组 |
| `MINUS_DI(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 下降方向指标 | 数组 |
| `DX(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 趋向指数 | 数组 |
| `AROON(HIGH, LOW, N)` | N: 周期(14) | 阿隆指标(向上) | 数组 |
| `AROONDOWN(HIGH, LOW, N)` | N: 周期(14) | 阿隆指标(向下) | 数组 |
| `SAR(HIGH, LOW, ACCEL, MAX)` | ACCEL: 加速因子(0.02), MAX: 最大值(0.2) | 抛物线转向 | 数组 |
| `PSAR(HIGH, LOW, ACCEL, MAX)` | 同上 | 抛物线SAR | 数组 |

### 震荡类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `RSI(X, N)` | X: 数据, N: 周期(14) | 相对强弱指标 | 数组 |
| `KDJ(HIGH, LOW, CLOSE, N, M1, M2)` | N: 周期(9), M1: 3, M2: 3 | KDJ指标(K值) | 数组 |
| `KD(HIGH, LOW, CLOSE, N, M1, M2)` | N: 周期(9), M1: 3, M2: 3 | KD指标(K值) | 数组 |
| `STOCH(HIGH, LOW, CLOSE, FASTK, SLOWK, SLOWD)` | FASTK: 14, SLOWK: 3, SLOWD: 3 | 随机振荡器(K值) | 数组 |
| `CCI(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 商品通道指数 | 数组 |
| `WILLR(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 威廉指标 | 数组 |
| `WR(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 威廉指标(别名) | 数组 |
| `MOM(X, N)` | N: 周期(10) | 动量指标 | 数组 |
| `ROC(X, N)` | N: 周期(10) | 变动率指标 | 数组 |
| `CMO(X, N)` | N: 周期(14) | 钱德动量振荡器 | 数组 |
| `TRIX(X, N)` | N: 周期(12) | 三重指数平滑平均 | 数组 |
| `MFI(HIGH, LOW, CLOSE, VOLUME, N)` | N: 周期(14) | 资金流量指标 | 数组 |
| `APO(X, FAST, SLOW)` | FAST: 12, SLOW: 26 | 绝对价格振荡器 | 数组 |
| `BOP(OPEN, HIGH, LOW, CLOSE)` | - | 平衡力量 | 数组 |

### 波动类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `ATR(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 平均真实波动范围 | 数组 |
| `NATR(HIGH, LOW, CLOSE, N)` | N: 周期(14) | 归一化真实波动范围 | 数组 |
| `TRANGE(HIGH, LOW, CLOSE)` | - | 真实波动范围 | 数组 |
| `BBANDS(X, N, NBDEV)` | N: 周期(20), NBDEV: 标准差倍数(2) | 布林带(上轨) | 数组 |
| `BOLLUP(X, N, NBDEV)` | 同BBANDS | 布林带上轨 | 数组 |
| `BOLLMID(X, N)` | N: 周期(20) | 布林带中轨 | 数组 |
| `BOLLDN(X, N, NBDEV)` | 同BBANDS | 布林带下轨 | 数组 |
| `BOLLWIDTH(X, N, NBDEV)` | 同BBANDS | 布林带宽度 | 数组 |
| `DONCHIAN(HIGH, LOW, N)` | N: 周期(20) | 唐安奇通道(上轨) | 数组 |
| `DONCHIAN_UPPER(HIGH, LOW, N)` | N: 周期(20) | 唐安奇通道上轨 | 数组 |
| `DONCHIAN_LOWER(HIGH, LOW, N)` | N: 周期(20) | 唐安奇通道下轨 | 数组 |
| `DONCHIAN_MIDDLE(HIGH, LOW, N)` | N: 周期(20) | 唐安奇通道中轨 | 数组 |

### 成交量类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `OBV(CLOSE, VOLUME)` | - | 能量潮指标 | 数组 |
| `AD(HIGH, LOW, CLOSE, VOLUME)` | - | 累积/分配线 | 数组 |
| `ADOSC(HIGH, LOW, CLOSE, VOLUME, FAST, SLOW)` | FAST: 3, SLOW: 10 | Chaikin A/D振荡器 | 数组 |

### 价格变换类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `AVGPRICE(OPEN, HIGH, LOW, CLOSE)` | - | 平均价格 | 数组 |
| `MEDPRICE(HIGH, LOW)` | - | 中间价格 | 数组 |
| `TYPPRICE(HIGH, LOW, CLOSE)` | - | 典型价格 | 数组 |
| `WCLPRICE(HIGH, LOW, CLOSE)` | - | 加权收盘价 | 数组 |

### 周期类 (希尔伯特变换)

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `HT_DCPERIOD(X)` | X: 数据序列 | 主导周期 | 数组 |
| `HT_DCPHASE(X)` | X: 数据序列 | 主导相位 | 数组 |
| `HT_PHASOR_IN(X)` | X: 数据序列 | 同相分量 | 数组 |
| `HT_PHASOR_QUAD(X)` | X: 数据序列 | 正交分量 | 数组 |
| `HT_SINE(X)` | X: 数据序列 | 正弦波 | 数组 |
| `HT_LEADSINE(X)` | X: 数据序列 | 领先正弦波 | 数组 |
| `HT_TRENDMODE(X)` | X: 数据序列 | 趋势/周期模式(1/0) | 数组 |
| `HT_TRENDLINE(X)` | X: 数据序列 | 瞬时趋势线 | 数组 |

### 统计类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `STDDEV(X, N)` | N: 周期 | 标准差 | 数组 |
| `ZSCORE(X, N)` | N: 周期 | Z-Score标准化 | 数组 |
| `BETA(X, Y, N)` | X: 资产数据, Y: 基准数据, N: 周期 | Beta系数 | 数组 |
| `CORREL(X, Y, N)` | X, Y: 数据序列, N: 周期 | 相关系数 | 数组 |
| `LINEAR_REG(X, N)` | N: 周期 | 线性回归 | 数组 |
| `TSF(X, N)` | N: 周期 | 时间序列预测 | 数组 |
| `PERCENT_RANK(X, N)` | N: 周期 | 百分比排名 | 数组 |
| `MIDPOINT(X, N)` | N: 周期 | 中点值 (最高+最低)/2 | 数组 |
| `MIDPRICE(HIGH, LOW, N)` | N: 周期 | 中间价格 | 数组 |

### 引用类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `REF(X, N)` | X: 数据, N: 向前引用期数 | 引用N天前的值 | 数组 |
| `REFDATE(X, D)` | X: 数据, D: 日期 | 引用指定日期的值 | 数组 |

### 逻辑与条件类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `CROSS(A, B)` | A, B: 数据序列 | A上穿B (A从下方穿越B) | 数组(0/1) |
| `LONGCROSS(A, B, N)` | A, B: 数据序列, N: 维持周期 | A维持在B下方N天后上穿 | 数组(0/1) |
| `EVERY(X, N)` | X: 条件, N: 周期 | 条件X在N周期内一直成立 | 数组(0/1) |
| `EXIST(X, N)` | X: 条件, N: 周期 | 条件X在N周期内存在成立 | 数组(0/1) |
| `FILTER(X, N)` | X: 条件, N: 过滤周期 | 信号过滤(N周期内只保留首次) | 数组(0/1) |
| `IF(X, T, F)` | X: 条件, T: 真值, F: 假值 | 条件选择 | 数组/标量 |
| `IFTHEN(X, T)` | X: 条件, T: 值 | 条件满足时返回T否则0 | 数组 |
| `COUNT(X, N)` | X: 条件, N: 周期 | 统计N周期内满足条件的次数 | 数组 |
| `BARSLAST(X)` | X: 条件 | 上一次条件成立到当前的周期数 | 数组 |
| `BETWEEN(X, A, B)` | X: 数据, A: 下界, B: 上界 | X是否在[A, B]区间内 | 数组(0/1) |
| `NOT(X)` | X: 数据/条件 | 逻辑非 | 数组(0/1) |

### 数学类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `ABS(X)` | X: 数据 | 绝对值 | 数组/标量 |
| `MAX(A, B)` | A, B: 数据 | 最大值 | 数组/标量 |
| `MIN(A, B)` | A, B: 数据 | 最小值 | 数组/标量 |
| `SQRT(X)` | X: 数据 | 平方根 | 数组/标量 |
| `POW(X, N)` | X: 底数, N: 指数 | 幂运算 | 数组/标量 |
| `EXP(X)` | X: 数据 | 指数函数 e^x | 数组/标量 |
| `LOG(X)` | X: 数据 | 自然对数 ln(x) | 数组/标量 |
| `LN(X)` | X: 数据 | 自然对数(别名) | 数组/标量 |
| `LOG10(X)` | X: 数据 | 常用对数 log10(x) | 数组/标量 |
| `SIGN(X)` | X: 数据 | 符号函数 (-1, 0, 1) | 数组/标量 |
| `FLOOR(X)` | X: 数据 | 向下取整 | 数组/标量 |
| `CEIL(X)` | X: 数据 | 向上取整 | 数组/标量 |
| `ROUND(X)` | X: 数据 | 四舍五入 | 数组/标量 |
| `SIN(X)` | X: 数据(弧度) | 正弦函数 | 数组/标量 |
| `COS(X)` | X: 数据(弧度) | 余弦函数 | 数组/标量 |
| `TAN(X)` | X: 数据(弧度) | 正切函数 | 数组/标量 |
| `ASIN(X)` | X: 数据 | 反正弦 | 数组/标量 |
| `ACOS(X)` | X: 数据 | 反余弦 | 数组/标量 |
| `ATAN(X)` | X: 数据 | 反正切 | 数组/标量 |

### 高级查找类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `FINDHIGH(X, N, M, T)` | X: 数据, N: 周期, M: 第M个高点, T: 类型 | 查找N周期内第M个高点值 | 数组 |
| `FINDLOW(X, N, M, T)` | X: 数据, N: 周期, M: 第M个低点, T: 类型 | 查找N周期内第M个低点值 | 数组 |
| `TOPN(X, N, M)` | X: 数据, N: 周期, M: 前M个 | 取N周期内前M个最大值 | 数组 |
| `PEAK(X, N, M)` | X: 数据, N: ZigZag百分比, M: 第M个峰 | ZigZag高点 | 数组 |
| `TROUGH(X, N, M)` | X: 数据, N: ZigZag百分比, M: 第M个谷 | ZigZag低点 | 数组 |
| `PEAKBARS(X, N, M)` | X: 数据, N: 百分比, M: 第M个峰 | 距第M个峰的Bar数 | 数组 |
| `TROUGHBARS(X, N, M)` | X: 数据, N: 百分比, M: 第M个谷 | 距第M个谷的Bar数 | 数组 |
| `ZIGZAG(X, N)` | X: 数据, N: 百分比 | ZigZag指标 | 数组 |
| `DRAWNULL(X)` | X: 数据 | 空值标记（NaN） | 数组 |
| `CEILING(X)` | X: 数据 | 向上取整（同CEIL） | 数组 |

### 信号过滤类（文华财经兼容）

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `AUTOFILTER(COND, N)` | COND: 信号条件, N: 最小间隔 | 自动过滤连续信号，保留间隔≥N的信号 | 数组(0/1) |
| `CHECKSIG(BUY, SELL, MODE)` | BUY: 买信号, SELL: 卖信号, MODE: 确认模式 | 买卖信号交替确认 | 数组(0/1) |
| `MULTSIG(BUY, SELL, N, M)` | BUY: 买信号, SELL: 卖信号, N: 模式, M: 间隔 | 多信号过滤 | 数组(0/1) |
| `ENTERLONG(COND)` | COND: 条件 | 开多信号 | 数组(0/1) |
| `EXITLONG(COND)` | COND: 条件 | 平多信号 | 数组(0/1) |
| `ENTERSHORT(COND)` | COND: 条件 | 开空信号 | 数组(0/1) |
| `EXITSHORT(COND)` | COND: 条件 | 平空信号 | 数组(0/1) |
| `BUY(COND)` | 同ENTERLONG | 买入（别名） | 数组(0/1) |
| `SELL(COND)` | 同EXITLONG | 卖出（别名） | 数组(0/1) |

### 数组/序列操作类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `CUMSUM(X)` | X: 数据序列 | 累加求和 | 数组 |
| `CUM(X)` | 同CUMSUM | 累加（别名） | 数组 |
| `CUMMAX(X)` | X: 数据序列 | 累计最大值 | 数组 |
| `CUMMIN(X)` | X: 数据序列 | 累计最小值 | 数组 |
| `PERCENTILE(X, N, P)` | X: 数据, N: 周期, P: 百分位(0-100) | 滚动百分位数 | 数组 |
| `MEDIAN(X, N)` | X: 数据, N: 周期 | 滚动中位数 | 数组 |

### 高阶统计类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `SKEW(X, N)` | X: 数据, N: 周期 | 滚动偏度 | 数组 |
| `KURT(X, N)` | X: 数据, N: 周期 | 滚动峰度 | 数组 |
| `MODE(X, N)` | X: 数据, N: 周期 | 滚动众数 | 数组 |
| `SORT(X, N, DIR)` | X: 数据, N: 周期, DIR: 1=升序/0=降序 | 滚动排序取最值 | 数组 |
| `RANK(X, N)` | X: 数据, N: 周期 | 滚动排名百分比 | 数组 |

### 跨周期引用类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `PERIODTYPE()` | 无 | 当前周期类型(1=日/2=周/3=月) | 数组(常量) |
| `REFDATE(X, IDX)` | X: 数据, IDX: Bar索引 | 引用指定Bar位置的值 | 数组(常量) |

### 绘图函数扩展

| 命令 | 说明 | 语法 |
|------|------|------|
| `DRAWSL(C1, P1, SLOPE, LEN)` | 斜线绘制 | `DRAWSL(COND, PRICE, SLOPE, LENGTH), COLORRED` |
| `DRAWTEXT_FIX(X, Y, TEXT)` | 固定位置文字 | `DRAWTEXT_FIX(0.5, 0.9, '信号')` |
| `DRAWNUMBER(COND, PRICE, NUM)` | 数值绘制 | `DRAWNUMBER(COND, HIGH, CLOSE)` |
| `VERTLINE(COND)` | 垂直线 | `VERTLINE(CROSS(MA5, MA10))` |

### 高级指标类

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `ICHIMOKU_TENKAN(HIGH, LOW, N)` | N: 转换线周期(9) | 一目均衡图-转换线 | 数组 |
| `ICHIMOKU_KIJUN(HIGH, LOW, N)` | N: 基准线周期(26) | 一目均衡图-基准线 | 数组 |
| `SUPERTrend(HIGH, LOW, CLOSE, ATR_N, MULT)` | ATR_N: 14, MULT: 3.0 | 超级趋势指标 | 数组 |
| `VWAP(HIGH, LOW, CLOSE, VOLUME)` | - | 成交量加权平均价格 | 数组 |
| `DONCHIAN_WIDTH(HIGH, LOW, N)` | N: 周期(20) | 唐安奇通道宽度 | 数组 |

### 绘图与常量类

| 函数/关键字 | 说明 |
|------------|------|
| `DRAWICON(COND, PRICE, ICON)` | 满足条件时在PRICE位置绘制ICON图标 |
| `DRAWTEXT(COND, PRICE, TEXT)` | 满足条件时在PRICE位置绘制TEXT文字 |
| `DRAWKLINE(H, O, L, C)` | 绘制K线 |
| `STICKLINE(COND, P1, P2, W, E)` | 满足条件时在P1到P2绘制柱线 |
| `DRAWBAND(UP, COLOR1, DN, COLOR2)` | 在UP和DN之间绘制带状区域 |
| `COLORRED` | 红色 |
| `COLORGREEN` | 绿色 |
| `COLORBLUE` | 蓝色 |
| `COLORYELLOW` | 黄色 |
| `COLORWHITE` | 白色 |
| `COLORBLACK` | 黑色 |
| `LINETHICK(N)` | 设置线宽 |
| `NODRAW` | 不绘制 |
| `DRAWNULL` | 空值不绘制 |

---

## 参数系统

公式支持自定义参数，参数在公式头部声明：

```
PARAM N1 = 5, N2 = 10, N3 = 20;
```

或在公式执行时通过 `params` HashMap 传入参数值。

参数类型支持：
- 整数：`N = 14`
- 浮点数：`MULT = 2.5`
- 参数范围：`N: 2 ~ 200` (在GUI中限制输入范围)

---

## 使用示例

### Rust

```rust
use finkit::formula::{FormulaEngine, FormulaContext};
use ndarray::Array1;

let open = Array1::from_vec(vec![10.0, 10.2, 10.1, 10.5, 10.3]);
let high = Array1::from_vec(vec![10.5, 10.8, 10.6, 10.9, 10.7]);
let low = Array1::from_vec(vec![9.8, 10.0, 9.9, 10.2, 10.1]);
let close = Array1::from_vec(vec![10.3, 10.5, 10.2, 10.6, 10.4]);
let volume = Array1::from_vec(vec![1000.0, 1200.0, 800.0, 1500.0, 1100.0]);

let mut ctx = FormulaContext::new(open, high, low, close, volume, None);
let mut engine = FormulaEngine::new();

// 简单的均线金叉判断
let source = r#"
MA5 := MA(CLOSE, 5);
MA20 := MA(CLOSE, 20);
CROSS(MA5, MA20)
"#;

match engine.eval(&source, &mut ctx) {
    Ok(result) => println!("Result: {:?}", result),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Python

```python
from finkit import Indicators

open_prices = [10.0, 10.2, 10.1, 10.5, 10.3]
high_prices = [10.5, 10.8, 10.6, 10.9, 10.7]
low_prices = [9.8, 10.0, 9.9, 10.2, 10.1]
close_prices = [10.3, 10.5, 10.2, 10.6, 10.4]
volumes = [1000.0, 1200.0, 800.0, 1500.0, 1100.0]

# 验证公式
is_valid = Indicators.formula_validate("MA5 := MA(CLOSE, 5); MA5 > MA(CLOSE, 20)")
print(f"Formula valid: {is_valid}")

# 计算结果
result = Indicators.formula_eval(
    source="MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
    open=open_prices,
    high=high_prices,
    low=low_prices,
    close=close_prices,
    volume=volumes,
)
print(result)
```

### Node.js

```javascript
const { Indicators } = require('finkit');

const open = [10.0, 10.2, 10.1, 10.5, 10.3];
const high = [10.5, 10.8, 10.6, 10.9, 10.7];
const low = [9.8, 10.0, 9.9, 10.2, 10.1];
const close = [10.3, 10.5, 10.2, 10.6, 10.4];
const volume = [1000.0, 1200.0, 800.0, 1500.0, 1100.0];

// 验证
const isValid = Indicators.formulaValidate("MA5 := MA(CLOSE, 5)");
console.log("Valid:", isValid);

// 计算
const result = Indicators.formulaEval(
  "MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
  open, high, low, close, volume
);
console.log(result);
```

### Java

```java
import com.finkit.Indicators;
import java.util.Map;

public class FormulaExample {
    public static void main(String[] args) {
        double[] open = {10.0, 10.2, 10.1, 10.5, 10.3};
        double[] high = {10.5, 10.8, 10.6, 10.9, 10.7};
        double[] low = {9.8, 10.0, 9.9, 10.2, 10.1};
        double[] close = {10.3, 10.5, 10.2, 10.6, 10.4};
        double[] volume = {1000.0, 1200.0, 800.0, 1500.0, 1100.0};

        // 验证公式
        boolean valid = Indicators.formulaValidate("MA5 := MA(CLOSE, 5);");
        System.out.println("Valid: " + valid);

        // 计算
        Map<String, double[]> result = Indicators.formulaEval(
            "MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
            open, high, low, close, volume
        );
        for (Map.Entry<String, double[]> entry : result.entrySet()) {
            System.out.println(entry.getKey() + ": " + java.util.Arrays.toString(entry.getValue()));
        }
    }
}
```

### Go

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit"
)

func main() {
    open := []float64{10.0, 10.2, 10.1, 10.5, 10.3}
    high := []float64{10.5, 10.8, 10.6, 10.9, 10.7}
    low := []float64{9.8, 10.0, 9.9, 10.2, 10.1}
    close := []float64{10.3, 10.5, 10.2, 10.6, 10.4}
    volume := []float64{1000.0, 1200.0, 800.0, 1500.0, 1100.0}

    // 验证公式
    valid := ta.FormulaValidate("MA5 := MA(CLOSE, 5);")
    fmt.Println("Valid:", valid)

    // 计算
    result, err := ta.FormulaEval(
        "MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
        open, high, low, close, volume,
    )
    if err != nil {
        fmt.Println("Error:", err)
        return
    }
    for name, values := range result {
        fmt.Printf("%s: %v\n", name, values)
    }
}
```

### .NET (C#)

```csharp
using System;
using System.Collections.Generic;
using Finkit;

class Program
{
    static void Main()
    {
        double[] open = { 10.0, 10.2, 10.1, 10.5, 10.3 };
        double[] high = { 10.5, 10.8, 10.6, 10.9, 10.7 };
        double[] low = { 9.8, 10.0, 9.9, 10.2, 10.1 };
        double[] close = { 10.3, 10.5, 10.2, 10.6, 10.4 };
        double[] volume = { 1000.0, 1200.0, 800.0, 1500.0, 1100.0 };

        // 验证公式
        bool valid = Indicators.FormulaValidate("MA5 := MA(CLOSE, 5);");
        Console.WriteLine($"Valid: {valid}");

        // 计算
        var result = Indicators.FormulaEval(
            "MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
            open, high, low, close, volume
        );
        foreach (var kvp in result)
        {
            Console.WriteLine($"{kvp.Key}: [{string.Join(", ", kvp.Value)}]");
        }
    }
}
```

### C++

```cpp
#include <finkit.hpp>
#include <iostream>

int main() {
    std::vector<double> open = {10.0, 10.2, 10.1, 10.5, 10.3};
    std::vector<double> high = {10.5, 10.8, 10.6, 10.9, 10.7};
    std::vector<double> low = {9.8, 10.0, 9.9, 10.2, 10.1};
    std::vector<double> close = {10.3, 10.5, 10.2, 10.6, 10.4};
    std::vector<double> volume = {1000.0, 1200.0, 800.0, 1500.0, 1100.0};

    // 验证公式
    bool valid = ta::FormulaValidate("MA5 := MA(CLOSE, 5);");
    std::cout << "Valid: " << std::boolalpha << valid << std::endl;

    // 计算
    auto result = ta::FormulaEval(
        "MA5 := MA(CLOSE, 5); MA20 := MA(CLOSE, 20); CROSS(MA5, MA20)",
        open, high, low, close, volume
    );
    for (const auto& [name, values] : result) {
        std::cout << name << ": [";
        for (size_t i = 0; i < values.size(); ++i) {
            if (i > 0) std::cout << ", ";
            std::cout << values[i];
        }
        std::cout << "]" << std::endl;
    }

    return 0;
}
```

---

## 执行模式

公式引擎支持三种执行模式，适用于不同场景：

### AST 解释执行（默认）

```python
result = Indicators.formula_eval(source, open, high, low, close, volume)
```

直接遍历 AST 树进行求值，无需编译开销，适合单次执行和交互式开发。

### 字节码编译执行

```python
result = Indicators.formula_eval_bytecode(source, open, high, low, close, volume)
```

将公式编译为字节码后再由虚拟机执行，性能提升 3-5 倍。适合重复执行的公式。

**字节码编译流程**：
```
Source → Parser → AST → Bytecode Compiler → VM → Result
```

**优势**：
- 编译期类型检查
- 紧凑的指令格式
- 更少的运行时开销
- 执行 5-10 次后即可摊平编译开销

### 优化执行

```python
result = Indicators.formula_eval_optimized(source, open, high, low, close, volume)
```

在执行前应用以下优化通道：

1. **常量折叠** - 预计算常量表达式（如 `2 * 3` → `6`）
2. **死代码消除** - 移除未使用的变量赋值
3. **公共子表达式消除** - 缓存重复计算（如多次出现的 `EMA(CLOSE,12)`）
4. **循环不变量外提** - 将循环无关计算移出循环

优化后通常可减少 30-50% 的计算量，性能提升 5-10 倍。

### 调试模式

```python
result = Indicators.formula_eval_debug(source, open, high, low, close, volume)
```

返回执行结果和详细的调试信息：

```python
{
    "result": {
        "__result__": [...],
        "MA5": [...],
        "MA20": [...]
    },
    "debug": {
        "steps": ["Parsed AST", "Compiled bytecode", ...],
        "variables": {"MA5": "Array", "MA20": "Array"},
        "errors": []
    }
}
```

---

## 公式模板库

公式引擎内置了 309 个常用公式模板，覆盖技术指标、趋势分析、成交量分析等领域。

### 模板分类

| 分类 | 数量 | 说明 |
|------|------|------|
| 均线类 | 8 | MA、EMA、均线交叉等 |
| 震荡类 | 12 | RSI、KDJ、MACD、CCI 等 |
| 波动类 | 8 | 布林带、ATR、唐安奇通道等 |
| 成交量类 | 6 | 量价齐升、缩量回调、OBV 等 |
| 趋势类 | 6 | ADX、SAR、超级趋势等 |
| 策略类 | 10 | 综合策略、背离策略等 |
| 通达信经典 | 50+ | 筹码峰、龙虎榜、涨停捕捉等 |
| 同花顺智能选股 | 30+ | 智能选股、条件预警等 |
| 大智慧资金流向 | 20+ | 资金流向、板块分析等 |
| 形态类 | 5 | 红三兵、早晨之星等 |

### 获取模板

```python
# 获取特定模板
template = Indicators.formula_get_template("macd_golden_cross")
print(template["name"])        # "MACD金叉"
print(template["category"])    # "Oscillator"
print(template["description"]) # "DIF上穿DEA形成金叉"
print(template["formula"])     # 公式源码

# 搜索模板
results = Indicators.formula_search_templates("金叉")
for t in results:
    print(t["name"], t["description"])

# 列出所有分类
categories = Indicators.formula_list_categories()
for cat in categories:
    print(cat["category"], cat["count"])
```

---

## 经典公式示例

### MA均线金叉

```
MA5 := MA(CLOSE, 5);
MA10 := MA(CLOSE, 10);
MA20 := MA(CLOSE, 20);

BUY := CROSS(MA5, MA10) AND MA5 > MA20;
SELL := CROSS(MA10, MA5);

BUY
```

### MACD指标

```
DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
DEA := EMA(DIF, 9);
MACD := 2 * (DIF - DEA);

// MACD金叉
BUY := CROSS(DIF, DEA);
// MACD死叉
SELL := CROSS(DEA, DIF);

// 柱状图
MACD
```

### KDJ指标

```
RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
K := SMA(RSV, 3, 1);
D := SMA(K, 3, 1);
J := 3 * K - 2 * D;

// KDJ金叉
BUY := CROSS(K, D) AND K < 20;
// KDJ死叉
SELL := CROSS(D, K) AND K > 80;

J
```

### 布林带突破

```
MID := MA(CLOSE, 20);
UPPER := MID + 2 * STD(CLOSE, 20);
LOWER := MID - 2 * STD(CLOSE, 20);

// 突破上轨
BUY := CROSS(CLOSE, UPPER);
// 跌破下轨
SELL := CROSS(LOWER, CLOSE);
// 带宽
WIDTH := (UPPER - LOWER) / MID * 100;

UPPER
```

### RSI指标

```
RSI1 := SMA(MAX(CLOSE - REF(CLOSE, 1), 0), 14, 1) / SMA(ABS(CLOSE - REF(CLOSE, 1)), 14, 1) * 100;
RSI2 := SMA(MAX(CLOSE - REF(CLOSE, 1), 0), 6, 1) / SMA(ABS(CLOSE - REF(CLOSE, 1)), 6, 1) * 100;

// RSI超卖后金叉
BUY := CROSS(RSI2, RSI1) AND RSI2 < 30;
// RSI超买后死叉
SELL := CROSS(RSI1, RSI2) AND RSI1 > 70;

RSI1
```

### 成交量均线

```
VOLUME_MA5 := MA(VOLUME, 5);
VOLUME_MA20 := MA(VOLUME, 20);

// 放量
VOL_UP := VOLUME > VOLUME_MA5 * 2;
// 缩量
VOL_DOWN := VOLUME < VOLUME_MA5 * 0.5;
// 量价齐升
PRICE_VOL_UP := CLOSE > REF(CLOSE, 1) AND VOL_UP;

VOLUME
```

### 综合策略示例

```
// 参数
N1 := 5;
N2 := 10;
N3 := 20;

// 均线
MA5 := MA(CLOSE, N1);
MA10 := MA(CLOSE, N2);
MA20 := MA(CLOSE, N3);

// MACD
DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
DEA := EMA(DIF, 9);
MACD := 2 * (DIF - DEA);

// 条件
MA_COND := MA5 > MA10 AND MA10 > MA20;
MACD_COND := MACD > 0 AND DIF > DEA;
VOL_COND := VOLUME > MA(VOLUME, 5);

// 买入信号
BUY := CROSS(MA5, MA10) AND MACD_COND AND VOL_COND;

// 卖出信号
SELL := CROSS(MA10, MA5) OR MACD < 0;

BUY
```

---

## 通达信/同花顺兼容性

### 兼容性总结

| 平台 | 兼容度 | 说明 |
|------|--------|------|
| 通达信 (TDX) | 100% | 核心指标、绘图命令、时间函数、大盘引用均已支持 |
| 同花顺 (THS) | 96.3% | 语法高度重叠，支持 THS 特有别名 |
| 大智慧 (DZH) | 100% | 板块引用、资金流向、基础指标完整支持 |
| 东方财富 (EM) | 95% | DKCOL多空列、EM之字转向、成本分布、主力持仓 |
| 飞狐交易师 (FoxTrader) | 92% | 飞狐之字转向、交易信号、策略回测 |

### 新增时间/Bar函数

| 函数 | 说明 | 示例 |
|------|------|------|
| `DATE()` | 当前Bar日期 (YYYYMMDD) | `DATE()` |
| `TIME()` | 当前Bar时间 (HHMMSS) | `TIME()` |
| `YEAR()` | 年份 | `YEAR()` |
| `MONTH()` | 月份 (1-12) | `MONTH()` |
| `DAY()` | 日 (1-31) | `DAY()` |
| `HOUR()` | 小时 (0-23) | `HOUR()` |
| `MINUTE()` | 分钟 (0-59) | `MINUTE()` |
| `WEEKDAY()` | 星期几 (0=周日, 1-6) | `WEEKDAY()` |
| `CURRBARSCOUNT()` | 当前Bar到末尾的距离 | `CURRBARSCOUNT()` |
| `TOTALBARSCOUNT()` | 总Bar数 | `TOTALBARSCOUNT()` |
| `BARSSINCE(X)` | 条件成立到当前的Bar数 | `BARSSINCE(CROSS(MA5,MA10))` |
| `BARSSINCEN(X,N)` | 最近N周期内条件成立距今Bar数 | `BARSSINCEN(CLOSE>OPEN, 20)` |
| `BARSCOUNT(X)` | 数据的有效期长度 | `BARSCOUNT(CLOSE)` |
| `BARSTATUS()` | Bar位置 (0=中间,1=首,2=末) | `BARSTATUS()` |
| `ISLASTBAR()` | 是否最后一根Bar | `ISLASTBAR()` |
| `FROMOPEN()` | 当日开盘以来的Bar数 | `FROMOPEN()` |

### 新增数学/统计函数

| 函数 | 说明 | 示例 |
|------|------|------|
| `AVEDEV(X,N)` | 平均绝对偏差 | `AVEDEV(CLOSE, 20)` |
| `DEVSQ(X,N)` | 偏差平方和 | `DEVSQ(CLOSE, 20)` |
| `SLOPE(X,N)` | 线性回归斜率 | `SLOPE(CLOSE, 14)` |
| `FORCAST(X,N)` | 线性回归预测值 | `FORCAST(CLOSE, 14)` |
| `RANGE(A,B,C)` | 判断B<A<C | `RANGE(CLOSE, LOW, HIGH)` |
| `CONST(X)` | 取最后一个值为常量 | `CONST(MA(CLOSE,5))` |
| `SUMBARS(X,A)` | 累加到>=A的周期数 | `SUMBARS(VOL, 10000)` |
| `INTPART(X)` | 取整数部分 | `INTPART(CLOSE)` |
| `FRACPART(X)` | 取小数部分 | `FRACPART(CLOSE)` |
| `MOD(A,B)` | 取模 | `MOD(CLOSE, 10)` |
| `REVERSE(X)` | 反转序列 | `REVERSE(CLOSE)` |
| `TR()` | 真实波幅 (=TRANGE) | `TR()` |

### 大盘/财务/筹码数据接口

| 函数 | 说明 | 备注 |
|------|------|------|
| `INDEXC()` | 大盘收盘价 | 需提供 index_data |
| `INDEXO()` | 大盘开盘价 | 需提供 index_data |
| `INDEXH()` | 大盘最高价 | 需提供 index_data |
| `INDEXL()` | 大盘最低价 | 需提供 index_data |
| `INDEXV()` | 大盘成交量 | 需提供 index_data |
| `INDEXA()` | 大盘成交额 | 需提供 index_data |
| `CAPITAL()` | 流通股本 | 需提供 capital |
| `FINANCE(N)` | 财务数据 | 需提供 finance_data |
| `DYNAINFO(N)` | 动态数据 | 需提供 dynainfo |
| `WINNER(X)` | 获利盘比例 | 需提供 chip_data |
| `LWINNER(X,N)` | 近N日获利盘 | 需提供 chip_data |
| `COST(X)` | 成本分布 | 需提供 chip_data |

### 绘图命令

| 命令 | 说明 | 语法 |
|------|------|------|
| `DRAWLINE` | 连线 | `DRAWLINE(C1,P1,C2,P2,EX)` |
| `DRAWBAND` | 填充带 | `DRAWBAND(V1,C1,V2,C2)` |
| `DRAWKLINE` | 绘K线 | `DRAWKLINE(H,O,L,C)` |
| `DRAWRECTREL` | 相对矩形 | `DRAWRECTREL(X1,Y1,X2,Y2,COLOR)` |
| `FILLRGN` | 填充区域 | `FILLRGN(COND,P1,P2)` |
| `PARTLINE` | 分段线 | `PARTLINE(COND,P)` |
| `POLYLINE` | 折线 | `POLYLINE(COND,P)` |
| `DRAWGBK` | 背景 | `DRAWGBK(COND,COLOR)` |

### 输出样式修饰

| 修饰符 | 说明 | 示例 |
|--------|------|------|
| `COLORRED` 等 | 颜色 | `MA5:MA(C,5),COLORRED` |
| `LINETHICK1`-`9` | 线粗 | `MA5:MA(C,5),LINETHICK2` |
| `NOTEXT` | 不显示数值 | `MA5:MA(C,5),NOTEXT` |
| `LINESTICK` | 线柱 | `VOL:V,LINESTICK` |
| `COLORSTICK` | 彩色柱 | `MACD:DIF-DEA,COLORSTICK` |
| `CROSSDOT` | 交叉点 | `S:...,CROSSDOT` |

### TDX 别名映射

| TDX名称 | 映射到 |
|---------|--------|
| `PDI` | `PLUS_DI` |
| `MDI` | `MINUS_DI` |
| `MTM` | `MOM` |
| `VOL` | `VOLUME` (数据变量) |
| `TR` | `TRANGE` |

### 同花顺 (THS) 专有别名

| THS名称 | 说明 |
|---------|------|
| `CLOSE1` | 昨收价 (= REF(CLOSE,1)) |
| `OPEN1` | 昨开价 (= REF(OPEN,1)) |
| `HIGH1` | 昨最高价 (= REF(HIGH,1)) |
| `LOW1` | 昨最低价 (= REF(LOW,1)) |
| `VOL1` | 昨成交量 (= REF(VOLUME,1)) |

### 同花顺 (THS) 智能选股与预警函数

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `SMARTSELECT(COND, MODE)` | COND: 条件, MODE: 模式(0=全部, 1=首次, 2=连续首次) | 智能选股 | 数组(0/1) |
| `SELECTCOND(COND)` | COND: 条件 | 条件选股 | 数组(0/1) |
| `ALERT(COND, MSG)` | COND: 条件, MSG: 预警消息 | 条件预警 | 数组(0/1) |
| `ALERTONCE(COND, MSG)` | COND: 条件, MSG: 预警消息 | 单次预警 | 数组(0/1) |

### 大智慧 (DZH) 专有函数

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `BLOCKDATA(NAME, FIELD)` | NAME: 板块名, FIELD: 字段名 | 板块数据引用 | 数组/标量 |
| `BLOCKINDEX(NAME)` | NAME: 板块名 | 板块指数 | 数组 |
| `BLOCKAVG(NAME)` | NAME: 板块名 | 板块均价 | 数组 |
| `MONEYFLOW()` | 无 | 资金流向 | 数组 |
| `NETINFLOW()` | 无 | 净流入 | 数组 |
| `BIGORDER()` | 无 | 大单比例 | 数组 |
| `SMALLORDER()` | 无 | 小单比例 | 数组 |
| `MAININFLOW()` | 无 | 主力流入 | 数组 |
| `MAININFLOWPCT()` | 无 | 主力流入占比 | 数组 |
| `SUPERBIGORDER()` | 无 | 超大单 | 数组 |

### 东方财富(EM)专有函数

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `DKCOL()` | 无 | 多空列数据，返回买卖量差值 | 数组 |
| `EM_CROSS(A, B)` | A: 数据序列1, B: 数据序列2 | A上穿B时返回1，否则0 | 数组 |
| `EM_REF(NAME, N)` | NAME: 数据名称, N: 回溯期 | 引用外部数据序列 | 数组 |
| `EM_ZIG(K, N)` | K: 数据源, N: 转向阈值 | EM之字转向 | 数组 |
| `EM_TROUGH(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个谷 | EM之字谷值 | 数组 |
| `EM_PEAK(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个峰 | EM之字峰值 | 数组 |
| `EM_TROUGHBARS(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个谷 | EM之字谷值距离 | 数组 |
| `EM_PEAKBARS(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个峰 | EM之字峰值距离 | 数组 |
| `EM_COSTEX(PRICE, VOLUME)` | PRICE: 价格, VOLUME: 成交量 | 累积成本分布 | 数组 |
| `EM_ZLCCV()` | 无 | 主力持仓数据 | 数组 |

### 飞狐交易师(FoxTrader)专有函数

| 函数 | 参数 | 说明 | 返回 |
|------|------|------|------|
| `FOX_ZIG(K, N)` | K: 数据源, N: 转向阈值 | 飞狐之字转向 | 数组 |
| `FOX_TROUGH(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个谷 | 飞狐之字谷值 | 数组 |
| `FOX_PEAK(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个峰 | 飞狐之字峰值 | 数组 |
| `FOX_TROUGHBARS(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个谷 | 飞狐之字谷值距离 | 数组 |
| `FOX_PEAKBARS(K, N, M)` | K: 数据源, N: 转向阈值, M: 第M个峰 | 飞狐之字峰值距离 | 数组 |
| `FOX_BUY(COND, PRICE)` | COND: 条件, PRICE: 买入价 | 飞狐买入信号 | 数组 |
| `FOX_SELL(COND, PRICE)` | COND: 条件, PRICE: 卖出价 | 飞狐卖出信号 | 数组 |
| `FOX_TRADE_SIGNAL(BUY_COND, SELL_COND)` | BUY_COND: 买条件, SELL_COND: 卖条件 | 交易信号综合 | 数组 |
| `FOX_BACKTEST(BUY_COND, SELL_COND, PRICE)` | BUY_COND: 买条件, SELL_COND: 卖条件, PRICE: 价格 | 策略回测累计盈亏 | 数组 |
| `FOX_PROFIT_RATIO(BUY_COND, SELL_COND, PRICE)` | 同上 | 盈亏比 | 数组 |
| `FOX_WIN_RATE(BUY_COND, SELL_COND, PRICE)` | 同上 | 胜率 | 数组 |
| `FOX_MAX_DRAWDOWN(BUY_COND, SELL_COND, PRICE)` | 同上 | 最大回撤 | 数组 |
| `FOX_TRADE_COUNT(BUY_COND, SELL_COND)` | BUY_COND: 买条件, SELL_COND: 卖条件 | 交易次数 | 数组 |

### 东方财富数据注入

使用 `EmData` 结构体注入东方财富专有数据：

```python
# Python 示例
em_data = EmData()
em_data.dkcol_data["BUYVOL"] = array1
em_data.dkcol_data["SELLVOL"] = array2
em_data.dkcol_data["ZLCCV"] = array3
em_data.external_data["MA5"] = array4

ctx = FormulaContext(open, high, low, close, volume, None).with_em_data(em_data)
```

### 从通达信/同花顺迁移指南

1. **直接运行**：大部分公式可直接运行，无需修改
2. **数据变量**：`CLOSE/C`, `OPEN/O`, `HIGH/H`, `LOW/L`, `VOLUME/V/VOL` 均已支持
3. **函数调用**：时间函数需使用括号形式，如 `YEAR()` 而非 `YEAR`
4. **大盘数据**：需通过 `FormulaContext::with_index_data()` 提供大盘数据
5. **筹码函数**：`WINNER/COST` 等为占位实现，需自行注入数据
6. **多周期引用**：通过 `FormulaContext::with_period_data()` 提供跨周期数据，使用 `PERIODTYPE()` 和 `REFDATE()` 进行引用

### 从文华财经迁移指南

1. **信号函数**：`ENTERLONG/EXITLONG/ENTERSHORT/EXITSHORT` 完整支持，`BUY/SELL` 为别名
2. **自动过滤**：`AUTOFILTER(COND, N)` 对应文华的连续信号过滤机制
3. **多信号确认**：`CHECKSIG(BUY, SELL, MODE)` 对应文华的信号确认模式
4. **赋值语法**：支持 `:=`（隐藏变量）和 `:`（输出变量）两种写法

### 从 TradingView Pine Script 迁移指南

1. **均线函数**：`ta.sma()` → `MA()`, `ta.ema()` → `EMA()`
2. **交叉判断**：`ta.crossover()` → `CROSS()`, `ta.crossunder()` → `CROSSBELOW()`
3. **统计函数**：`ta.stdev()` → `STD()`, `ta.percentile_nearest_rank()` → `PERCENTILE()`
4. **累计函数**：`ta.cum()` → `CUMSUM()`, `math.max()` 历史 → `CUMMAX()`
5. **排名函数**：`ta.percentrank()` → `RANK()`
6. **注释语法**：Pine 的 `//` 注释直接兼容

### 从大智慧迁移指南

1. **基础语法**：大智慧公式语法与通达信高度一致，可直接运行
2. **赋值运算符**：支持 `=` 和 `:=` 两种赋值方式
3. **块注释**：支持 `{...}` 大智慧风格块注释
4. **数据引用**：`CLOSE`, `HIGH`, `LOW`, `OPEN`, `VOL` 均为标准变量名
5. **板块数据**：`BLOCKDATA(NAME, FIELD)` / `BLOCKINDEX(NAME)` / `BLOCKAVG(NAME)` 用于板块数据引用，需通过 `FormulaContext::with_block_data()` 注入板块数据
6. **资金流向**：`MONEYFLOW()` / `NETINFLOW()` / `BIGORDER()` / `MAININFLOW()` 等资金流向函数，需通过 `FormulaContext::with_money_flow_data()` 注入资金流向数据

---

## 语法兼容性

公式引擎支持多种语法风格，兼容各大平台的习惯写法：

### 赋值运算符

| 语法 | 说明 | 来源 |
|------|------|------|
| `A := EXPR` | 隐藏赋值（不输出） | 通达信/同花顺 |
| `A = EXPR` | 隐藏赋值 | 大智慧/文华 |
| `A: EXPR` | 输出赋值（显示在图表） | 通达信/同花顺 |

### 注释语法

| 语法 | 说明 | 来源 |
|------|------|------|
| `// comment` | 行注释 | 通达信/Pine Script |
| `/* comment */` | 块注释 | C风格 |
| `{ comment }` | 块注释 | 大智慧 |
| `# comment` | 行注释 | Python风格 |

### 字符串

| 语法 | 说明 |
|------|------|
| `"text"` | 双引号字符串 |
| `'text'` | 单引号字符串 |

---

## 多输出机制

公式引擎支持函数返回多个命名输出，适用于 MACD、KDJ 等多指标公式：

### 使用方式

```rust
use finkit::formula::{FormulaEngine, FormulaContext};

let mut engine = FormulaEngine::new();
let source = r#"
    DIF: EMA(CLOSE, 12) - EMA(CLOSE, 26);
    DEA: EMA(DIF, 9);
    MACD: (DIF - DEA) * 2
"#;

let result = engine.eval_multi(source, &mut ctx).unwrap();
// result.outputs["DIF"]  → DIF 线数组
// result.outputs["DEA"]  → DEA 线数组
// result.outputs["MACD"] → MACD 柱状图数组
```

输出变量使用 `:` 赋值（而非 `:=`），表示该变量需要在结果中输出。

### 典型多输出公式

```
{布林带三线}
MID: MA(CLOSE, 20);
UPPER: MID + 2 * STD(CLOSE, 20);
LOWER: MID - 2 * STD(CLOSE, 20)
```

```
{KDJ三线}
RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
K: SMA(RSV, 3, 1);
D: SMA(K, 3, 1);
J: 3 * K - 2 * D
```

---

## 跨周期引用

公式引擎支持在日线公式中引用周线/月线数据：

### 设置跨周期数据

```rust
use std::collections::HashMap;
use finkit::formula::types::{FormulaContext, PeriodData};

let weekly_data = PeriodData {
    open: Array1::from_vec(weekly_open),
    high: Array1::from_vec(weekly_high),
    low: Array1::from_vec(weekly_low),
    close: Array1::from_vec(weekly_close),
    volume: Array1::from_vec(weekly_vol),
};

let mut period_data = HashMap::new();
period_data.insert("WEEK".to_string(), weekly_data);

let ctx = FormulaContext::new(open, high, low, close, volume, None)
    .with_period_type(1)      // 1=日线
    .with_period_data(period_data);
```

### 引用函数

| 函数 | 说明 |
|------|------|
| `PERIODTYPE()` | 返回当前周期类型 (1=日/2=周/3=月) |
| `REFDATE(X, IDX)` | 引用指定Bar索引的值 |

---

## 性能优化

公式引擎提供三种优化模式，适用于不同场景：

### 惰性求值 (Lazy Evaluation)

自动分析 AST 依赖图，跳过不影响最终输出的中间变量计算：

```rust
let result = engine.eval_lazy(source, &mut ctx).unwrap();
```

适用于大型公式文件中只需要部分输出的场景，可减少 30-50% 不必要的计算。

### 增量计算 (Incremental Computation)

追加新数据后无需从头计算，利用编译缓存加速：

```rust
// 初始计算
let _ = engine.eval(source, &mut ctx).unwrap();

// 追加新 Bar
ctx.append_bar(open, high, low, close, volume);

// 增量重算
let result = engine.eval_incremental(source, &mut ctx).unwrap();
```

适用于实时行情推送场景，每次只需处理新增数据。

### 并行计算 (Parallel Computation)

分析 AST 中无依赖关系的语句，并行执行独立计算分支：

```rust
let result = engine.eval_parallel(source, &mut ctx).unwrap();
```

适用于多指标同时计算的场景（如 `A := MA(CLOSE,5); B := RSI(CLOSE,14); C := ATR(HIGH,LOW,CLOSE,14)`），当启用 `rayon` feature 时自动并行。

---

## 平台兼容性总览

| 平台 | 兼容度 | 核心能力 |
|------|--------|----------|
| 通达信 (TDX) | 100% | 核心指标、绘图命令、时间函数、大盘引用、PEAK/TROUGH/ZIGZAG |
| 同花顺 (THS) | 96.3% | 语法高度重叠，支持 THS 特有别名(CLOSE1/OPEN1/HIGH1/LOW1/VOL1) |
| 文华财经 | 90% | ENTERLONG/EXITLONG/AUTOFILTER/CHECKSIG/MULTSIG |
| 大智慧 (DZH) | 100% | 板块引用、资金流向、{注释}、= 赋值、基础指标完整支持 |
| 东方财富 (EM) | 95% | DKCOL多空列、EM_ZIG/PEAK/TROUGH之字转向、EM_COSTEX成本分布、EM_ZLCCV主力持仓 |
| 飞狐交易师 (FoxTrader) | 92% | FOX_ZIG/PEAK/TROUGH之字转向、FOX_BUY/SELL信号、FOX_BACKTEST回测、盈亏比/胜率/最大回撤 |
| TradingView Pine | ~60% | 核心函数映射可用，但语法结构差异较大需手动改写 |
