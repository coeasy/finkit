# Finkit 产品说明

## 一句话定位

**Finkit 是面向量化研究、实时分析和多语言产品集成的 Rust 金融计算引擎。**

它把技术指标、公式、因子 DAG、流式计算、特征工程、研究分析与多语言交付统一到同一套 canonical kernel、执行计划和正确性合同中，让团队可以把金融计算作为基础设施复用，而不是在每个应用里重新实现一遍。

当前正式发布版本为 **v0.1.15**。Unified Runtime、typed Artifact、DirtyRange 与 Factor Research 的进一步整合正在 PR #29 中作为下一版本候选能力验证。

## 产品使命

让量化系统中的“计算层”变得像数据库或数值库一样可复用、可验证、可嵌入。

Finkit 不试图替代研究平台、行情平台或交易系统，而是提供它们共同需要的底层能力：稳定数值语义、可复用执行计划、批量与实时计算、研究统计和多语言交付。

## 用户真正遇到的问题

一个团队从研究走向生产后，通常会出现这些问题：研究 Notebook 使用 Python 实现，实时服务又用 Rust/Java 重写；指标、公式、因子、特征工程分别维护自己的缓存和 DAG；相同 rolling statistics 在多个模块重复实现；每次调用都重新解析公式与发现依赖；局部数据修订触发全量重算；绑定代码存在但没有明确的发布和兼容状态；性能优化依赖单个函数 micro-benchmark，缺少端到端运行时合同。

这些问题的共同根因不是“算法不够多”，而是 **计算能力没有产品化为统一基础设施**。

## Finkit 的产品承诺

Finkit 希望提供四个稳定承诺：

1. **同一语义**：一个 canonical Rust 实现服务多个入口与语言，减少算法漂移。
2. **同一执行模型**：公式、因子、批量、局部和流式能力逐步收敛到共享 Runtime，而不是各自维护隐藏执行器。
3. **同一正确性边界**：对齐、warm-up、NaN、point-in-time、no-lookahead 和增量安全规则明确且可测试。
4. **同一验证体系**：编译、Clippy、SSOT、参考值、跨语言、性能和发布门禁形成连续证据链。

## 目标用户

| 用户角色 | 他们关心什么 | Finkit 提供什么 |
| --- | --- | --- |
| 量化研究员 | 快速验证公式、特征和因子，不想维护底层算法 | 指标、Formula、Feature、Factor、Research primitives |
| 量化开发工程师 | 研究结果如何稳定进入服务 | Rust core、compiled plan、Runtime、FFI/SDK |
| 数据/特征平台团队 | 大量 rolling/label/statistics 不重复实现 | canonical kernels、feature pipeline、批量执行 |
| 行情/分析产品团队 | 实时更新、选股、分析 API 的计算底座 | Streaming、Formula、FactorPlan、低开销运行时 |
| SDK/基础设施团队 | 多语言接口语义一致、升级可控 | 一个核心、多语言 delivery contracts |

## 六个产品价值支柱

### 1. Canonical Compute Core

Rust 是数值语义的单一来源。不同语言优先复用核心，而不是复制算法实现。

### 2. Reusable Plans

Formula 与 Factor 从“一次性函数调用”升级为可以提前编译、提前校验、持续复用的执行计划。重复请求不需要重复完成解析和依赖发现。

### 3. Batch + Streaming + Local Recompute

同一计算能力可以服务完整历史、逐 Bar 更新和局部修订。下一版本 Runtime 中，`DirtyRange` 不只是失效标签，而是实际执行范围；只有完整依赖链能够证明安全时才执行局部重算。

### 4. Research-Ready Building Blocks

Finkit 不停留在技术指标层，还提供或正在整合 transforms、labels、feature engineering、statistics、ranking、regression、validation、factor analysis、risk 与 lightweight backtest primitives。

### 5. Performance with Evidence

SIMD、zero-copy、borrowed input、`_into` output、persistent plans、streaming state、buffer reuse 等优化必须有 correctness/parity/performance evidence，而不是只靠宣传数字。

### 6. Multi-language Product Delivery

绑定层负责让产品接入不同生态，核心算法仍由 Rust 维护。Finkit 明确区分 source exists、CI validated、package candidate、release asset 和 public registry package。

## 产品能力模型

| 层级 | 能力 | 产品作用 |
| --- | --- | --- |
| Financial Compute Core | 指标、统计、回归、排序、数学内核 | 提供 canonical numerical semantics |
| Formula Engine | Parser、Compiler、Bytecode/JIT、缓存、range/last | 把终端式表达式变为可复用计划 |
| Factor Engine | Named factors、DAG、dependency validation、FactorPlan | 把因子依赖显式化和可重复执行化 |
| Unified Runtime | full/borrowed/range/into、typed artifacts、DirtyRange | 统一执行、状态与局部重算边界 |
| Streaming Engine | incremental state、one-bar update、batch parity | 支撑实时分析与低延迟更新 |
| Feature / Research | feature、label、CV、analysis、risk、report | 打通研究计算链路 |
| Language Delivery | Rust/Python/CLI/native/mobile/WASM | 把同一内核嵌入不同产品 |

## 典型采用路径

### 路径 A：研究团队

从 Python/Rust 的指标和 Formula 开始，随后复用 Feature、Factor 和 Research 能力。目标是先减少研究代码中的重复数学实现，再逐步把可稳定的 plan 迁入服务。

### 路径 B：行情与分析服务

把高频调用的公式和因子预编译，使用 borrowed/zero-copy 输入和 caller-owned output；实时链路使用 Streaming，需要历史数据修订时再使用安全的 DirtyRange 重算。

### 路径 C：因子平台

使用 FactorPlan 固化依赖关系，Research 层复用同一因子定义与 canonical statistics，避免“计算因子”和“研究因子”成为两个系统。

### 路径 D：多语言产品

把算法和语义留在 Rust 核心，在 Python、Node、Java、C/C++、Go、.NET、移动端或 WASM 侧只做符合各语言习惯的 API 封装和数据传递。

## 为什么不是只用传统 TA 库？

传统 TA 库最擅长的是“输入数组，得到指标数组”。Finkit 保留这一能力，但产品目标更宽：可编译公式、依赖 DAG、Streaming、Feature/Research、统一 Runtime、多语言交付和增量正确性边界。

如果你的需求只是偶尔计算几个常见指标，一个成熟 TA 库可能已经足够；如果你需要长期维护一个研究/分析产品，并希望批量、实时、因子、研究和 SDK 共享计算底座，Finkit 的价值会更明显。

## 为什么不是自己维护一套 Python 研究栈？

Python 非常适合研究和编排，但随着算法数量、吞吐量、实时要求和多语言接入增长，纯 Python 团队往往需要处理性能热点、重复实现、GIL/对象开销以及生产迁移问题。Finkit 的定位不是替代 Python，而是让 Python 调用更稳定的底层计算 Runtime。

## 与完整交易框架的关系

Finkit 不管理账户、订单、撮合、券商连接或交易状态机。它可以作为交易框架或研究平台的底层计算引擎，但不会把计算核心绑定到某个 broker/exchange。

## 正确性模型

产品级计算不仅要求“函数有返回值”，还需要明确以下合同：

- bars 按 oldest → newest 排列；
- OHLCV 和关联字段必须保持对齐；
- rolling 结果保留长度并显式表达 warm-up；
- 多输出组合使用联合有效性规则；
- borrowed 数据在同步执行期间不得被非法修改；
- 预测研究必须 point-in-time / no-lookahead；
- 增量执行必须证明依赖链安全，否则 full fallback；
- artifact 身份必须稳定、typed、可复现。

## 性能模型

Finkit 不把“某个函数在某台机器上的 ns/bar”作为产品承诺。性能工程关注的是完整路径：减少解析、依赖发现、分配、复制和无效重算，并用 dedicated benchmark / relative performance gates 验证退化。

普通 unit test 只负责功能和灾难性性能异常，不应使用过窄的共享 runner 单次墙钟阈值制造随机红灯。

## 成熟度与采用建议

**可以直接采用的稳定核心**：指标、Formula、Streaming、Feature 基础能力、已有 Factor 能力，以及 v0.1.15 已验证的发布资产。

**适合试用与集成验证的下一版本能力**：Unified Runtime、typed Artifact、DirtyRange local recompute、扩展后的 Factor Research。它们已进入 PR #29 的真实 CI/跨语言门禁，但在最终 same-SHA 全绿前不应描述为正式发布能力。

**需要按绑定状态单独确认的能力**：Node、Java、C/C++、Go、.NET、Android、iOS、WASM 的公开包发行。源码存在或 CI 可构建，不等价于已经发布到公共 registry。

## 产品原则

Finkit 的长期演进遵循四条原则：

- **Reuse before rewrite**：优先复用 canonical kernel 与计划，不重复实现数学逻辑。
- **Correctness before cleverness**：优化必须服从结果语义和 no-lookahead 安全。
- **Plans before repeated work**：能提前编译和验证的工作不应每次请求重复执行。
- **Evidence before claims**：性能、兼容性和发布状态都以 CI、benchmark 和 release asset 为依据。

## 产品边界

Finkit 不做 OMS、Broker API、Exchange Gateway、Matching Engine 或 turnkey live-trading platform。它专注于 **金融计算、因子研究、实时分析和跨语言运行时**。

## 下一阶段产品方向

下一阶段不是简单“继续加指标”，而是继续提高复用率和产品闭环：Unified Runtime 全面收口、FactorPlan/Research 共享执行计划、DirtyRange 进入更多安全内核、artifact provenance/缓存完善、多因子研究与报告流程稳定化，以及多语言发布合同标准化。

## 相关文档

- [README 中文版](../README.zh-CN.md)
- [宣传文稿](promotion-zh.md)
- [品牌与媒体素材](media-kit-zh.md)
- [Runtime 与 Factor](runtime-and-factors.md)
- [Factor Research 架构](factor-research-architecture.md)
- [多语言绑定](language-bindings.md)
- [完整使用指南](usage.md)
- [性能基准](benchmark-results.md)
