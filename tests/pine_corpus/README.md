# Pine Script Corpus

公开 Pine Script indicator 语料回归集，用于验证 AlphaTA Pine 方言解析、映射与执行的兼容性。

## 目录结构

```
tests/pine_corpus/
├── README.md           # 本文件
├── manifest.json       # 语料状态清单
├── rsi.pine
├── macd.pine
├── bollinger_bands.pine
├── supertrend.pine
├── vwap.pine
└── ...                 # 26 个脚本
```

## 覆盖场景

| 类别 | 脚本 |
|------|------|
| 动量 | RSI, Stochastic, CCI, Williams %R, Momentum, ROC, TRIX |
| 趋势 | MACD, EMA Crossover, SMA, SuperTrend, ADX, Ichimoku, Parabolic SAR, Aroon |
| 波动率 | Bollinger Bands, ATR, Keltner Channels, Donchian Channels |
| 成交量 | VWAP, OBV, VWMA, Volume Profile |
| 价格变换 | Heikin Ashi |
| 跨周期 | cross_security (request.security) |

## manifest.json 字段

| 字段 | 说明 |
|------|------|
| `id` | 唯一标识 |
| `file` | `.pine` 文件名 |
| `status` | `partial` / `blocked` / `pass` |
| `parse_ok` | Pine 解析器是否通过 |
| `map_ok` | Pine → AlphaTA AST 映射是否通过 |
| `eval_ok` | 端到端求值是否通过 |
| `pass_rate` | 内置函数级通过率（0–1） |
| `builtin_functions` | 脚本涉及的内置函数列表 |

## 相关文档

- [Pine 兼容矩阵](../../docs/PINE_COMPAT_MATRIX.md)
- [Pine 文法](../../docs/formula/pine-grammar.md)
- [Pine → AlphaTA 迁移指南](../../docs/migration/pine-to-AlphaTA.md)
