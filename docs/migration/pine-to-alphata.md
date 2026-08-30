# Pine Script → AlphaTA 迁移指南

本文档帮助将 TradingView Pine Script v5 指标迁移到 AlphaTA 公式引擎（含 Pine 方言与原生 AlphaTA/TDX 公式）。

---

## 快速开始

### 方式一：Pine 方言（推荐用于现有 Pine 脚本）

```rust
use alphata_core::formula::{parse_formula_with_dialect, FormulaDialect};

let source = r#"//@version=5
indicator("RSI")
length = input(14)
rsi = ta.rsi(close, length)
plot(rsi)
"#;

let ast = parse_formula_with_dialect(source, FormulaDialect::Pine)?;
```

### 方式二：改写为 AlphaTA/TDX 公式

```text
N:=14;
RSI_VAL:RSI(CLOSE,N);
```

---

## 常见函数对照表

### ta.* 技术分析（已映射）

| Pine Script | AlphaTA | 说明 |
|-------------|--------|------|
| `ta.sma(src, len)` | `SMA(src, len)` | 简单移动平均 |
| `ta.ema(src, len)` | `EMA(src, len)` | 指数移动平均 |
| `ta.rsi(src, len)` | `RSI(src, len)` | 相对强弱指数 |
| `ta.macd(src, fast, slow, sig)` | `MACD` → DIF/DEA/MACD | 多返回值 |
| `ta.atr(len)` | `ATR(len)` | 平均真实波幅 |
| `ta.stoch(close, high, low, len)` | `STOCH` → K/D | 随机指标 |
| `ta.cci(src, len)` | `CCI(src, len)` | 商品通道指数 |
| `ta.bb(src, len, mult)` | `BOLL` → MID/UP/DN | 布林带 |

### math.* 数学函数（已映射）

| Pine Script | AlphaTA |
|-------------|--------|
| `math.abs(x)` | `ABS(x)` |
| `math.log(x)` | `LOG(x)` |
| `math.max(a, b)` | `MAX(a, b)` |
| `math.min(a, b)` | `MIN(a, b)` |
| `math.pow(a, b)` | `POW(a, b)` |
| `math.sqrt(x)` | `SQRT(x)` |

### NA 处理（已映射）

| Pine Script | AlphaTA 等价 |
|-------------|-------------|
| `na(x)` | `ISNA(x)` |
| `nz(x, y)` | `IF(ISNA(x), y, x)` |
| `fixnan(x)` | `FIXNAN(x)` |

### 跨周期（部分映射）

| Pine Script | AlphaTA | 状态 |
|-------------|--------|------|
| `request.security(sym, tf, expr)` | `SECURITY(...)` | ⚠️ 解析可用；不重绘语义未实现 |

### ta.* 尚未映射（需手写 AlphaTA 或等待后续版本）

| Pine Script | AlphaTA 替代建议 |
|-------------|-----------------|
| `ta.supertrend(f, len)` | 使用 AlphaTA `SUPERTREND` 指标 API（若有）或手写 |
| `ta.vwap(src)` | `VWAP` 内置函数 |
| `ta.wpr(len)` | `WILLR(high, low, close, len)` |
| `ta.obv` | `OBV` |
| `ta.dmi(len, len)` | `DMI` / `ADX` 系列 |
| `ta.sar(start, inc, max)` | `SAR` |
| `ta.highest(src, len)` | `HHV(src, len)` |
| `ta.lowest(src, len)` | `LLV(src, len)` |
| `ta.mom(src, len)` | `MOM(src, len)` |
| `ta.roc(src, len)` | `ROC(src, len)` |
| `ta.aroon(len)` | `AROON` |
| `ta.trix(src, len)` | `TRIX` |
| `ta.vwma(src, len)` | 手写 `SUM(src*vol,len)/SUM(vol,len)` |

### 内置变量对照

| Pine Script | AlphaTA |
|-------------|--------|
| `open` | `OPEN` |
| `high` | `HIGH` |
| `low` | `LOW` |
| `close` | `CLOSE` |
| `volume` | `VOLUME` |
| `hl2` | `(HIGH+LOW)/2` |
| `hlc3` | `(HIGH+LOW+CLOSE)/3` |
| `ohlc4` | `(OPEN+HIGH+LOW+CLOSE)/4` |

---

## 迁移步骤

### 1. 评估兼容性

对照 [PINE_COMPAT_MATRIX.md](../PINE_COMPAT_MATRIX.md)，确认脚本使用的内置函数是否在映射表中。

可将脚本放入 `tests/pine_corpus/` 并更新 `manifest.json` 做回归跟踪。

### 2. 替换不支持的入口

```pine
// Pine — 不支持
strategy("My Strategy", overlay=true)

// 改用
indicator("My Indicator", overlay=true)
```

策略逻辑需在 AlphaTA 应用层实现，而非公式引擎内。

### 3. 替换警报与绘图对象

```pine
// 不支持
alertcondition(condition, title="Alert")
line.new(x1, y1, x2, y2)

// 绘图改用 AlphaTA 原生指令（TDX 方言）
DRAWTEXT(condition, CLOSE, "BUY");
STICKLINE(condition, OPEN, CLOSE, 2, 1);
```

### 4. 处理历史引用

```pine
// Pine
prevClose = close[1]

// AlphaTA
PREV_CLOSE := REF(CLOSE, 1);
```

Pine 方言可解析 `[1]` 语法，但复杂历史引用建议改写为 `REF()`。

### 5. 处理多返回值

```pine
// Pine
[macdLine, signalLine, histLine] = ta.macd(close, 12, 26, 9)

// AlphaTA（分变量赋值或指标 API）
DIF := EMA(CLOSE,12) - EMA(CLOSE,26);
DEA := EMA(DIF, 9);
MACD := 2 * (DIF - DEA);
```

### 6. 选择方言

| 场景 | 推荐 |
|------|------|
| 现有 Pine 脚本、快速验证 | `FormulaDialect::Pine` |
| 生产环境、国内行情软件习惯 | AlphaTA/TDX 公式 |
| 跨平台 CI 回归 | `tests/pine_corpus/` + `tests/formula_corpus/` |

---

## 限制说明（不支持的特性清单）

以下特性**无法**通过 Pine 方言直接使用，必须迁移或在上层应用实现：

| 特性 | 影响 | 迁移建议 |
|------|------|----------|
| `strategy()` | 无法编译策略脚本 | 应用层回测框架 |
| Repaint semantics | `request.security` 行为与 TV 不一致 | 避免 lookahead；用 AlphaTA 跨周期 API |
| 自定义类型 (UDT) | `type` / 方法调用失败 | 拆为独立变量或 Rust 结构体 |
| `alertcondition()` | 无警报触发 | 应用层条件监控 |
| `library` / `import` | 无法引用外部库 | 内联函数或 Rust 模块 |
| `syminfo.*` | 品种元信息不可用 | 从行情上下文注入 |
| `line`/`label`/`box` | 对象绘图不可用 | TDX `DRAWTEXT` 等 |
| `array.*` | 动态数组不可用 | 固定长度或 Rust 侧处理 |
| `var` 跨 bar 状态 | 持久化语义不完整 | `REF` + 显式状态变量 |
| `plot.style_*` 完整样式 | 样式不生效 | 可视化层配置 |
| `color.new()` 等 | 颜色表达式不完整 | 硬编码或可视化层 |

---

## 示例：RSI 完整迁移

### Pine Script 原版

```pine
//@version=5
indicator("RSI", overlay=false)
length = input(14, "RSI Length")
rsi = ta.rsi(close, length)
plot(rsi, "RSI")
hline(70)
hline(30)
```

### AlphaTA TDX 公式

```text
N:=14;
RSI_VAL:RSI(CLOSE, N);
```

### Pine 方言（无需改写）

```rust
parse_formula_with_dialect(pine_source, FormulaDialect::Pine)?;
```

---

## 相关资源

- [Pine 文法规范](../formula/pine-grammar.md)
- [Pine 兼容矩阵](../PINE_COMPAT_MATRIX.md)
- [AlphaTA 公式文法](../formula/grammar.md)
- [公式语料回归集](../../tests/formula_corpus/README.md)
- [Pine 语料回归集](../../tests/pine_corpus/README.md)
- 内置映射实现：`core/src/formula/pine/builtin_table.rs`
