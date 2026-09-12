# Finkit 产品说明

## 一句话介绍

**Finkit 是一个以 Rust 为统一内核的高性能量化金融计算引擎，覆盖技术指标、公式、因子、流式分析、特征工程与因子研究，并通过多语言绑定向不同产品形态输出同一套数值语义。**

## 为什么需要 Finkit

很多量化项目最初只是需要几个指标函数，但随着业务发展，通常会出现几套彼此独立的计算系统：

- Python 研究代码一套实现，生产服务再实现一套；
- Formula、Factor、Feature、Streaming 分别维护自己的 DAG、缓存和中间结果；
- 每次请求重新解析表达式、重新发现依赖、重新申请临时缓冲区；
- “增量计算”只有接口名，底层仍然全量重算；
- 回归、收益率、排序、分位数、风险指标在不同模块重复实现；
- Node、Java、C++、Go、.NET 等绑定逐渐演变成独立算法分支，语义不断漂移。

Finkit 把这些问题统一看成一个“金融计算基础设施”问题。

核心原则是：

> **金融语义只实现一次，依赖图只编译一次，执行状态尽可能复用，再通过不同语言交付。**

## 产品边界

Finkit 专注计算与研究，不负责：

- OMS；
- 券商交易接口；
- 交易所 Gateway；
- 订单路由；
- 撮合；
- 账户管理；
- 一站式实盘交易系统。

这使项目能够把工程资源集中在：

- 数值正确性；
- 时间序列对齐；
- 执行计划复用；
- no-lookahead / point-in-time 研究安全；
- 低分配、高吞吐计算；
- 可复现 Artifact；
- 多语言一致性。

## 七层产品能力

### 1. 金融计算核心

Rust core 提供技术指标、统计、数学变换、K 线/图形模式、风险、收益率、日历、特征工程等 canonical kernel。

公开 API 可以有不同 facade，但底层算法不应在多个模块重复实现。

### 2. Formula Engine

Formula 用于表达 terminal-style 计算逻辑。用户可以将表达式编译成可复用计划，重复作用于对齐市场数据。

相关优化包括：

- 编译缓存；
- Bytecode / JIT；
- range / last evaluation；
- append-oriented execution；
- reusable buffer；
- borrowed / zero-copy input。

### 3. Factor Engine

Factor 是带显式依赖关系的命名计算节点。`FactorPlan` 会在执行前完成：

- 依赖解析；
- cycle 检查；
- 原始输入发现；
- 拓扑排序；
- stale plan 验证。

这样重复计算不需要每次重新发现依赖关系。

### 4. Unified Runtime

下一版本架构正在把可复用计算统一到一个 Runtime 边界。

核心概念：

- `ArtifactHash`：typed、确定性内容身份；
- `DirtyRange`：显式失效范围；
- `FactorPlan`：预编译因子执行图；
- retained materialization：保留上次输出用于局部更新；
- conservative fallback：无法证明局部执行正确时全量计算。

DirtyRange 被拆成三个不同概念：

1. **input dirty range**：原始输入真正变化的行；
2. **affected output range**：依赖传播后可能变化的输出行；
3. **recompute range**：为了正确计算这些输出，需要向历史补齐的输入窗口。

这个设计解决一个常见错误：只向过去补 lookback，却没有把输入修改对未来 rolling 输出的影响向后传播。

### 5. Streaming Engine

对于自然适合逐 Bar 更新的计算，Finkit 使用 retained state 避免每个新 Bar 都重新计算整段历史。

批量路径和流式路径通过一致性测试约束，避免历史回放与实时计算逐渐产生不同结果。

### 6. Feature / Research Layer

项目已经提供并继续整合：

- lag / lead / diff / pct_change；
- rolling stats；
- normalization；
- labels / triple barrier；
- mutual information；
- feature selection；
- PCA；
- Purged KFold / Embargo / CPCV；
- regime / stability；
- risk / lightweight backtest；
- Factor Research / Alphalens-style analysis；
- multi-factor / portfolio / attribution / capacity / monitoring。

研究层遵循 reuse-first 原则，不重新实现 research 专用的 return / regression / rank / risk 内核。

### 7. Multi-language Delivery

Rust 是语义中心，多语言绑定只是交付方式。

目标平台包括：

- Python；
- Rust；
- CLI；
- Node.js；
- Java/JNI；
- C/C++；
- Go/CGO；
- .NET；
- Android；
- iOS；
- WebAssembly。

不同语言的公开发布状态不同。Finkit 明确区分 source exists、CI validated、package candidate、GitHub Release asset 和 public registry package。

## 目标用户

### 量化研究员

用一套计算引擎完成指标、公式、特征、标签、因子、研究统计与研究结果生成。

### 量化平台工程师

把 Finkit 作为选股、信号、行情分析、研究服务或数据平台的底层计算组件，而不是把算法逻辑散落到业务服务中。

### 实时分析团队

使用 Streaming retained state 和可复用 Formula/Factor Plan 处理高频重复计算。

### SDK / 产品团队

在 Rust 中维护核心算法，再面向 Python、JVM、Node、C++、Go、.NET、移动端和浏览器提供本地化 API。

### ML / 数据工程团队

利用金融特征、标签、统计、验证和研究 Artifact 构建可复现的数据集与特征流水线。

## 典型场景

### 场景一：选股 / 行情扫描器

把常用公式和因子提前编译，对大量股票反复执行，减少解析与依赖发现开销，并保持不同语言服务之间的语义一致。

### 场景二：因子研究服务

从 MarketFrame 生成因子/特征，计算 forward returns，执行 point-in-time/no-lookahead 验证，完成分组收益、IC、多因子、组合、风险和报告分析。

### 场景三：实时行情分析

新增 Bar 使用 streaming state；历史数据发生修订时，对满足 range-safe 条件的计划使用 DirtyRange 局部重算。

### 场景四：量化数据流水线

复用 returns、statistics、regression、rank、quantile、calendar 等 canonical kernel 生成 ML 特征和研究数据集。

### 场景五：多语言金融 SDK

算法只在 Rust 内核中维护，各语言只负责适配、打包和生态集成，降低长期维护成本。

## 架构图

```text
                      Market / External Data
                               │
                               ▼
                      MarketFrame / contracts
                               │
       ┌───────────────────────┼──────────────────────┐
       ▼                       ▼                      ▼
  Indicators                 Formula               Factors
       │                       │                      │
       └──────────────┬────────┴─────────────┬────────┘
                      ▼                      ▼
              canonical kernels       Compute/Factor Plan
                      │                      │
                      └───────────┬──────────┘
                                  ▼
                           Unified Runtime
                  full / borrowed / range / into
                                  │
                 ┌────────────────┼────────────────┐
                 ▼                ▼                ▼
             Streaming         Feature          Research
                 │                │                │
                 └────────────────┴────────┬───────┘
                                          ▼
                    Rust / Python / CLI / native SDK / WASM
```

## 设计原则

### Single Source of Truth

支持的函数、指标、版本和错误码尽可能由机器可读 registry/schema 生成。文档不再维护容易漂移的硬编码数量。

### Reuse First

新增功能优先复用现有 math、returns、calendar、risk、graph、cache、buffer infrastructure。

### Explicit Contracts

warm-up、NaN、alignment、lookback、stateful、deterministic、effect、ownership 都是正式契约，而不是实现偶然行为。

### Correct Incremental Execution

只有整个依赖链都可以证明 range-safe 时才执行局部重算。未知语义、动态窗口、横截面依赖自动 full fallback。

### No Lookahead

任何预测性因子研究都必须尊重 point-in-time 信息边界。

### Multi-language Parity

绑定负责暴露核心，不复制算法。

## 性能策略

Finkit 的优化目标覆盖整个执行路径：

- SIMD kernel；
- `_into` caller-owned buffer；
- borrowed / zero-copy input；
- persistent compiled formula；
- FactorPlan dependency reuse；
- streaming retained state；
- BufferArena / scratch reuse；
- DirtyRange local execution；
- fused / multi-period kernel；
- CI performance regression gate。

性能结果依赖 CPU、编译器、数据规模、内存布局、feature 和语言绑定开销，因此应在目标环境重新 benchmark。

## 质量与发布门禁

Finkit 使用多层门禁保证发布质量：

- unit/integration tests；
- batch/streaming parity；
- selected reference / TA-Lib comparison；
- formula semantic contract；
- no-lookahead tests；
- rustfmt / Clippy / workspace；
- version/SSOT/docs link；
- no-std / feature matrix；
- Python ABI3 wheels；
- multi-language packaging/runtime smoke；
- benchmark / performance regression。

不同 SHA 的绿色结果不能拼成 same-SHA green；runner 没有实际执行 steps 的结果也不能当作有效门禁证据。

## 当前发布与下一版本

### v0.1.15

当前正式发布版本是 v0.1.15。GitHub Release 是该版本权威发行来源，包含经过验证的 Python ABI3 wheels、Rust crate asset、Linux x86_64 CLI 和校验文件。

### 下一版本

当前 Factor Research 分支正在继续完成：

- typed Artifact identity；
- Unified Runtime；
- FactorPlan runtime integration；
- DirtyRange correctness；
- Research Artifact / provenance / cache；
- Factor Research / multi-factor / portfolio / report；
- 同 SHA 多平台、多语言发布门禁。

这些开发中能力不会被倒灌成 v0.1.15 已发布能力。

## Roadmap

近期重点：

1. 继续将 Formula / Factor / Research 执行边界收敛到 Unified Runtime；
2. 完成 DirtyRange 语义、性能与 fallback 门禁；
3. 完成 Factor Research prepare/analyze/report 主体链路；
4. 清除 statistics / regression / returns / rank / quantile / risk 重复实现；
5. 完成 typed Artifact provenance / cache；
6. 完成同 SHA multi-platform / multi-language 交付门禁；
7. 完善产品文档、案例、示例和包发现体验。

Finkit 的长期目标不是成为“函数最多”的金融库，而是成为 **可信、可复用、高性能、跨语言的量化金融计算基础设施**。

## 延伸阅读

- [快速开始](getting-started.md)
- [Runtime 与 Factor](runtime-and-factors.md)
- [Factor Research 架构](factor-research-architecture.md)
- [Formula Runtime](formula-runtime.md)
- [多语言绑定](language-bindings.md)
- [开发与 CI](development.md)
- [性能结果](benchmark-results.md)
- [宣传文稿](promotion-zh.md)
