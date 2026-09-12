# Finkit 品牌与媒体素材

本文用于统一 Finkit 在 GitHub、官网、技术社区、发布公告、媒体介绍和演讲中的产品表达。所有文案都应遵守一个原则：**已发布能力与开发中候选能力必须明确区分，性能与兼容性只使用可验证证据。**

## 品牌核心

**中文定位**

> 面向量化研究、实时分析与多语言产品集成的高性能金融计算引擎。

**英文定位**

> A Rust-powered quantitative finance compute engine for research, realtime analytics, factor workflows, and multi-language products.

**产品类别**

> Quant Compute Runtime / Financial Compute Engine

**核心价值句**

> 一个 Rust 核心统一指标、公式、因子、流式计算、研究分析与多语言交付。

## 10 秒 Elevator Pitch

> Finkit 是 Rust 驱动的 Quant Compute Runtime，让研究、实时分析、因子和多语言 SDK 共享同一套金融计算核心。

## 30 秒 Elevator Pitch

> Finkit 不只是技术指标库。它把指标、Formula、Factor DAG、Streaming、Feature Engineering、Research 和多语言绑定组织在一个 canonical Rust core 上。团队可以提前编译公式和因子依赖，复用统一数值语义，并在安全时使用局部重算，而不是为 Notebook、服务端和 SDK 重复维护多套算法。

## 2 分钟产品介绍

> 很多量化系统一开始只有少量指标函数，但随着研究、实时服务、因子平台和多语言 SDK 增长，同一套计算会被复制到多个地方，最终出现结果漂移、重复解析、缓存分裂和全量重算。
>
> Finkit 想把这个问题向下收敛到计算基础设施层。Rust 维护 canonical numerical kernels；Formula 与 Factor 使用可复用计划；Streaming 负责逐 Bar 更新；Unified Runtime 逐步承接 full、borrowed、range 和 into 执行；Feature 与 Factor Research 复用同一统计和依赖体系；不同语言只负责产品交付，不重新实现算法。
>
> 当前正式版本是 v0.1.15。Unified Runtime、typed ArtifactHash、DirtyRange 局部执行和更完整的 Factor Research 仍属于下一版本候选能力，只有 same-SHA 全门禁通过后才会进入发布判断。

## 官网 Hero

### 中文

**标题**

> 一套计算核心，连接量化研究与实时分析。

**副标题**

> 用 Rust 统一指标、Formula、Factor DAG、Streaming、Research 和多语言产品交付，让金融计算成为可复用基础设施。

**CTA**

> 查看快速开始 · 阅读产品说明 · 查看架构

### English

**Headline**

> One compute core for quantitative research and realtime analytics.

**Subheadline**

> Unify indicators, formulas, factor graphs, streaming computation, research analytics, and multi-language delivery on a canonical Rust core.

**CTA**

> Get started · Explore the product · Read the architecture

## GitHub About

**推荐英文长版**

> Rust-powered quantitative finance compute engine for indicators, formulas, factor graphs, streaming analytics, research workflows, and multi-language products.

**推荐英文短版**

> Quant compute runtime for research, realtime analytics, factors, and multi-language SDKs.

**推荐中文**

> Rust 驱动的量化金融计算引擎，统一指标、公式、因子、流式分析、研究工作流与多语言交付。

## 技术社区发布稿

### 长版开头

> 量化项目扩展到一定规模以后，最难维护的往往不是某个指标，而是同一套计算逻辑在 Notebook、服务端、实时链路、因子平台和 SDK 中被重复实现。Finkit 想解决的就是这个问题：用一个 canonical Rust core 把金融计算收敛成可以长期复用、验证和嵌入的 Runtime。

### 中版

> Finkit 正在从高性能指标库继续收敛为 Quant Compute Runtime。指标、Formula、Factor、Streaming、Feature 和 Research 围绕同一个 Rust 核心组织，并通过 reusable plans、zero-copy、SIMD、typed artifacts 与安全的 DirtyRange 执行减少重复工作。当前正式版本为 v0.1.15；下一版本 Runtime/Research 架构仍在 PR #29 完成 same-SHA 验证。

### 短版

> Finkit：一个 Rust 核心统一指标、Formula、Factor、Streaming、Research 和多语言 SDK。

## 面向不同受众的主信息

| 受众 | 首要信息 | 次要信息 |
| --- | --- | --- |
| 量化研究员 | 不只算指标，还能复用 Formula/Feature/Factor/Research | 减少 Notebook 到生产的语义差异 |
| 量化工程师 | 一个核心 + reusable plans + shared runtime | zero-copy、streaming、DirtyRange、FFI |
| 数据平台团队 | 复用 rolling/statistics/labels，而非重复实现 | 批量执行与研究数据链路 |
| 产品团队 | 可作为独立行情/分析/因子计算底座 | 不绑定券商或交易框架 |
| SDK 团队 | 多语言共享数值语义 | 发布状态和兼容合同明确 |
| 开源贡献者 | 关注 correctness、reuse、performance evidence | 不以 API 数量代替架构质量 |

## 竞争定位表达

### 对传统 TA 库

不要说“替代所有 TA 库”。推荐说：

> Finkit 保留数组输入 → 指标输出的经典能力，但进一步覆盖 Formula、Factor DAG、Streaming、Feature/Research、统一 Runtime 和多语言交付，更适合需要长期维护计算基础设施的场景。

### 对纯 Python 研究栈

不要说“Python 不适合量化”。推荐说：

> Python 继续负责高效率研究和编排，Finkit 提供可复用的底层 Rust 计算层，让性能热点、数值语义和生产迁移更容易统一。

### 对完整交易框架

不要说“Finkit 是交易系统”。推荐说：

> Finkit 专注计算层，可以嵌入研究平台、交易框架或行情服务，但不管理账户、订单、撮合和券商连接。

## FAQ / 常见疑问

### Finkit 和 TA-Lib 是什么关系？

Finkit 覆盖大量传统技术分析能力，也持续通过参考值/parity 与性能测试验证实现。但产品目标不仅是指标 API，还包括 Formula、Factor、Streaming、Feature/Research 与统一 Runtime。

### Finkit 是 Alphalens 的替代品吗？

不应简单这样描述。Finkit 正在扩展 Alphalens-style 因子研究能力，但更强调与自己的 FactorPlan、统计内核、Runtime 和多语言计算底座复用。

### 所有绑定都发布了吗？

不是。Finkit 明确区分源码存在、CI 验证、package candidate、GitHub Release asset 与 public registry package。准确状态以 `docs/language-bindings.md` 为准。

### 所有因子都能 DirtyRange 局部执行吗？

不能。只有依赖链能证明 incremental、fixed-lookback、time-series safe 时才能局部执行；否则必须 full fallback。

### Finkit 可以直接实盘交易吗？

Finkit 不是 OMS 或 broker gateway。它提供可以被实盘系统调用的计算和研究引擎。

## 可直接使用的社交文案

**X / 微博短版**

> Finkit = Rust Quant Compute Runtime。一个核心统一指标、Formula、Factor、Streaming、Research 和多语言 SDK。目标不是再堆一套函数，而是让金融计算成为可复用基础设施。

**开发者社区版**

> 当 Notebook、服务端、因子平台和 SDK 都在重写同一套指标时，问题已经不是“缺函数”，而是缺统一计算层。Finkit 用 canonical Rust core + reusable plans + shared runtime 收敛这条链路。

**下一版本预告版**

> Finkit 下一版本候选架构正在把 typed ArtifactHash、FactorPlan、UnifiedRuntime 与真正的 DirtyRange 局部执行接到同一主链。只有能证明安全的依赖链才增量执行，其他情况 full fallback。

## 演讲 / PPT 开场句

> 我们不缺计算 RSI 的函数，我们缺的是让研究、实时、因子和 SDK 共享同一个 RSI 语义的基础设施。

> Finkit 的核心问题不是“还能加多少指标”，而是“同一个计算能不能只实现一次、验证一次、在所有场景复用”。

## 文案风格规则

推荐使用：统一核心、canonical kernel、reusable plan、correctness contract、safe local recompute、multi-language delivery、research/realtime continuity。

避免使用未经证据支持的绝对化表达，如“最快”“完全兼容所有系统”“所有指标性能第一”“零误差”“所有语言均已发布”“100% 增量执行”。

当功能位于开发分支时使用“下一版本候选”“正在验证”“PR 中已实现并等待门禁”等表述，不写成当前正式发行能力。

## 事实基线

- 当前正式发布：v0.1.15。
- 核心语言：Rust。
- 稳定核心：Technical Analysis、Formula、Streaming、Feature 与已有 Factor 能力。
- 下一版本候选：Unified Runtime、typed Artifact、DirtyRange local execution、扩展 Factor Research。
- 多语言状态：按 `docs/language-bindings.md` 分层说明。
- 性能表述：以仓库 benchmark / performance regression evidence 为准，不承诺跨机器固定数字。

## 相关文档

- [产品说明](product-overview-zh.md)
- [项目宣传文稿](promotion-zh.md)
- [README 中文版](../README.zh-CN.md)
- [Runtime 与 Factor](runtime-and-factors.md)
- [Factor Research 架构](factor-research-architecture.md)
- [多语言绑定](language-bindings.md)
