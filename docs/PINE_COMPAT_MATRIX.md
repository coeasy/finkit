# Pine Script 兼容矩阵

> 基于 `tests/pine_corpus/` 26 个公开 indicator 语料的回归基线。  
> 最后更新：2026-06-23

## 总览

| 指标 | 数值 |
|------|------|
| 语料脚本数 | 26 |
| 解析通过 | 18 / 26 (69%) |
| 映射通过 | 12 / 26 (46%) |
| 求值通过 | 8 / 26 (31%) |
| 整体通过率 | **31%** |

## 脚本 × 内置函数 × 通过率 × 结论

| 脚本 | 关键内置函数 | 通过率 | 结论 |
|------|-------------|--------|------|
| rsi | `ta.rsi`, `input`, `plot`, `hline` | 75% | ✅ 核心指标可用；绘图元数据忽略 |
| macd | `ta.macd`, `input`, `plot` | 70% | ✅ 多返回值 MACD 已映射 |
| bollinger_bands | `ta.bb`, `fill`, `plot` | 70% | ✅ 布林带三轨映射成功 |
| supertrend | `ta.supertrend` | 20% | ❌ `ta.supertrend` 未实现 |
| vwap | `ta.vwap` | 20% | ❌ `ta.vwap` 未实现 |
| volume_profile | `for`, `volume[i]`, `close[i]` | 50% | ⚠️ 循环可解析；series 索引语义不完整 |
| stochastic | `ta.stoch`, `ta.sma` | 55% | ⚠️ 嵌套调用部分通过 |
| cci | `ta.cci`, `hlc3` | 75% | ✅ CCI 可用 |
| atr | `ta.atr` | 80% | ✅ ATR 可用 |
| ema_crossover | `ta.ema` | 80% | ✅ 双 EMA 可用 |
| sma_overlay | `ta.sma` | 85% | ✅ SMA 可用 |
| williams_r | `ta.wpr` | 20% | ❌ `ta.wpr` 未实现 |
| obv | `ta.obv` | 20% | ❌ `ta.obv` 未实现 |
| adx | `ta.dmi` | 15% | ❌ `ta.dmi` 多返回值未实现 |
| ichimoku | `ta.lowest`, `ta.highest`, 用户函数 | 10% | ❌ 用户函数 + 极值函数未支持 |
| parabolic_sar | `ta.sar` | 20% | ❌ `ta.sar` 未实现 |
| keltner_channels | `ta.ema`, `ta.atr` | 65% | ⚠️ 组合指标部分可用 |
| donchian_channels | `ta.highest`, `ta.lowest`, `math.avg` | 25% | ❌ 极值函数未映射 |
| momentum | `ta.mom` | 20% | ❌ `ta.mom` 未实现 |
| roc | `ta.roc` | 20% | ❌ `ta.roc` 未实现 |
| aroon | `ta.aroon` | 15% | ❌ `ta.aroon` 未实现 |
| trix | `ta.trix` | 20% | ❌ `ta.trix` 未实现 |
| vwma | `ta.vwma` | 20% | ❌ `ta.vwma` 未实现 |
| heikin_ashi | `nz`, `ta.ema`, `math.max/min`, `[1]` | 45% | ⚠️ 历史引用 + nz 部分支持 |
| macd_histogram | `ta.macd`, 颜色三元组 | 65% | ⚠️ plot 颜色参数未完整 |
| cross_security | `request.security`, `syminfo.tickerid` | 10% | ❌ syminfo 内置变量未支持 |

## 内置函数映射通过率

基于 `core/src/formula/pine/builtin_table.rs` 当前实现：

| 命名空间 | 函数 | Finkit 映射 | 语料命中 | 状态 |
|----------|------|-------------|----------|------|
| ta | sma | SMA | sma_overlay, stochastic | ✅ |
| ta | ema | EMA | ema_crossover, keltner, heikin_ashi | ✅ |
| ta | rsi | RSI | rsi | ✅ |
| ta | macd | MACD (DIF/DEA/MACD) | macd, macd_histogram | ✅ |
| ta | atr | ATR | atr, keltner_channels | ✅ |
| ta | stoch | STOCH (K/D) | stochastic | ⚠️ 嵌套调用 |
| ta | cci | CCI | cci | ✅ |
| ta | bb | BOLL (MID/UP/DN) | bollinger_bands | ✅ |
| ta | supertrend | — | supertrend | ❌ |
| ta | vwap | — | vwap | ❌ |
| ta | wpr | — | williams_r | ❌ |
| ta | obv | — | obv | ❌ |
| ta | dmi | — | adx | ❌ |
| ta | sar | — | parabolic_sar | ❌ |
| ta | highest | — | donchian, ichimoku | ❌ |
| ta | lowest | — | donchian, ichimoku | ❌ |
| ta | mom | — | momentum | ❌ |
| ta | roc | — | roc | ❌ |
| ta | aroon | — | aroon | ❌ |
| ta | trix | — | trix | ❌ |
| ta | vwma | — | vwma | ❌ |
| math | abs/log/max/min/pow/sqrt | 同名 | heikin_ashi, ichimoku | ✅ |
| — | nz/na/fixnan | IF/ISNA/FIXNAN | heikin_ashi | ⚠️ |
| request | security | SECURITY | cross_security | ⚠️ 解析受限 |

## 明确不支持清单

以下 Pine Script 特性**当前不在支持范围内**，语料中遇到将标记为 `blocked`：

| 特性 | 说明 |
|------|------|
| `strategy()` | 策略脚本入口；仅支持 `indicator()` / `study()` |
| Repaint semantics | `request.security` 不重绘、`barmerge` lookahead 等行为未实现 |
| 自定义类型 | `type MyType`、UDT 字段、方法调用 |
| `alertcondition()` | 警报条件声明与触发 |
| `library` | 库导入与导出 |
| `var` 跨 bar 持久化 | `var`/`varip` 声明可解析，运行时语义不完整 |
| `line`/`label`/`box` 绘图对象 | 命名空间在文法中预留，运行时未实现 |
| `array.*` 动态数组 | 数组命名空间预留，无运行时 |
| `syminfo.*` / `timeframe.*` | 品种与周期元信息内置变量 |
| `plot.style_*` 完整样式 | 仅解析 `plot()` 调用，样式枚举不完整 |
| `color.new` / `color.*` 完整色板 | 颜色表达式部分解析 |

## 使用方式

```bash
# 验证语料目录
test -d tests/pine_corpus

# 查看 manifest
python -c "import json; m=json.load(open('tests/pine_corpus/manifest.json')); print(m['summary'])"

# Pine 方言解析冒烟（Rust）
cargo test -p finkit dialect_tests -- --nocapture
```

## 相关文件

- 语料目录：`tests/pine_corpus/`
- Pine 解析器：`core/src/formula/pine/parser.rs`
- 内置映射表：`core/src/formula/pine/builtin_table.rs`
- 方言入口：`core/src/formula/mod.rs` (`FormulaDialect::Pine`)
