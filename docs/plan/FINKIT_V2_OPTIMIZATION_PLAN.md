# Finkit v2.0 优化升级方案

## 1. 项目定位

Finkit 定位为：

> 最完整的开源金融指标与因子计算基础库（Open Financial Indicator & Factor Engine）。

目标不是量化交易平台，而是提供类似 TA-Lib、通达信公式系统、TradingView Pine Script 的统一金融计算基础设施。

核心能力：

- 高性能金融指标计算
- 金融因子计算
- 主流交易终端公式兼容
- 自定义公式 DSL
- 多语言 SDK
- AI/MCP 调用能力

---

## 2. 总体架构

```
finkit
├── core-engine        C++ 高性能计算核心
├── indicator-engine   技术指标引擎
├── factor-engine      因子计算引擎
├── formula-engine     公式解析与执行
├── runtime-engine     优化执行运行时
├── bindings           多语言绑定
│   ├── python
│   ├── rust
│   ├── go
│   ├── java
│   ├── csharp
│   └── node
└── cli
```

---

## 3. 核心技术路线

采用 C++20 作为计算核心：

- SIMD 优化
- AVX2/AVX512 支持
- ARM NEON 支持
- 多线程并行
- 零拷贝数据交换
- 流式计算支持

上层通过语言绑定提供 SDK。

---

## 4. 指标计算体系

兼容 TA-Lib 分类。

### 趋势指标

- MA
- SMA
- EMA
- WMA
- HMA
- MACD
- ADX

### 动量指标

- RSI
- ROC
- MOM
- CCI
- KDJ

### 波动指标

- ATR
- Bollinger Bands
- STD
- VAR

### 成交量指标

- OBV
- VWAP
- MFI
- CMF

目标：500+金融指标。

---

## 5. 公式系统

实现统一 Formula Engine。

支持：

- 通达信公式
- 同花顺公式
- 东方财富公式
- TradingView Pine Script 子集
- Finkit DSL

执行流程：

```
Formula
 ↓
Lexer
 ↓
Parser
 ↓
AST
 ↓
Optimizer
 ↓
ByteCode VM
 ↓
Runtime
```

支持函数：

- MA
- EMA
- REF
- HHV
- LLV
- CROSS
- COUNT
- SUM
- IF
- BARSLAST

---

## 6. 因子计算引擎

支持：

### 动量因子

- RET5
- RET20
- RET60

### 波动因子

- Volatility
- ATR Ratio

### 价值因子

- PE
- PB
- PS
- EV/EBITDA

### 质量因子

- ROE
- ROA
- Profit Growth

支持自定义 Factor DSL。

---

## 7. 数据结构设计

统一核心对象：

### Candle

```
timestamp
open
high
low
close
volume
amount
```

### Series

统一时间序列结构。

### IndicatorResult

统一指标输出格式。

---

## 8. 多语言发布

目标：

Python:

```
pip install finkit
```

Rust:

```
cargo add finkit
```

Go:

```
go get github.com/coeasy/finkit
```

Java:

Maven package

C#:

NuGet package

Node:

npm install finkit

---

## 9. AI 扩展

提供 MCP Server：

```
calculate_indicator()
calculate_factor()
parse_formula()
```

支持 AI Agent 调用金融计算能力。

---

## 10. 版本规划

### v0.1

- C++ Core
- Python Binding
- 基础指标库

### v0.2

- TA-Lib 兼容
- 200+指标
- Benchmark体系

### v0.3

- 通达信公式兼容
- Formula VM

### v0.4

- Factor Engine
- Alpha因子体系

### v1.0

形成生产级开源金融计算基础库。

---

## 11. 与 AaaStock 的关系

保持独立：

```
AaaStock
 数据层

    ↓

Finkit
 指标与因子计算层

    ↓

策略 / AI / 回测系统
```

AaaStock 提供数据，Finkit 提供计算能力。

---

## 12. 长期目标

Finkit 最终成为：

> 金融领域的 LLVM：公式解析、编译优化、高性能执行、多语言调用。
