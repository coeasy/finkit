# Formula 引擎文档

Finkit 公式引擎（`finkit::formula`）：通用证券公式语言运行时，
兼容多券商产品语法（TDX/THS/DZH）与 Pine Script（部分）。

## 子文档

| 文档 | 内容 |
|------|------|
| [grammar.md](grammar.md) | 核心公式语法（TDX 风格：`MA`/`HHV`/`LLV`/`REF`/`CROSS`……） |
| [pine-grammar.md](pine-grammar.md) | Pine Script v5 子集语法与映射 |

## 引擎流水线

```
源码 → parser(pest grammar) → AST → 编译缓存(真 LRU) → 字节码 → 解释执行 / JIT → SIMD 指令
```

- 入口：`FormulaEngine::{compile/execute/eval}`。
- 缓存：`FormulaCache`，命中加速约 23x。
- 方言选择：`FormulaEngine::parse(src, dialect)`（TDX/THS/DZH/Pine）。
- 模板：`FormulaTemplates`（309 个经典指标）。
- 源码位置：`core/src/formula/`；调试器见 `FormulaDebugger`。

## 兼容性概览

| 方言 | 兼容度 |
|------|--------|
| TDX | ≈100% |
| THS（同花顺） | ≈96.3% |
| DZH（大智慧） | ≈100% |
| 文华 | ≈90% |
| Pine Script v5 | ≈60% |

> 更多实现细节见 [公式引擎](./grammar.md) 与 [Pine 语法](./pine-grammar.md)。