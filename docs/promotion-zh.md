# Finkit 项目宣传文稿

## 推荐标题

**Finkit：把量化计算从“函数集合”升级为可复用的金融计算 Runtime**

备选：

- Finkit：一套 Rust 核心，连接指标、公式、因子、流式计算与研究分析
- Finkit：面向量化研究与实时分析的高性能多语言金融计算引擎
- 从 TA 指标到 Unified Runtime：Finkit 正在构建量化系统的计算底座

## 30 秒介绍

Finkit 是一个 Rust 驱动的量化金融计算引擎。它不只提供技术指标，还把 Formula、Factor DAG、Streaming、Feature Engineering、Research 与多语言绑定放到统一计算核心上。

它适合用于量化研究平台、行情分析服务、选股系统、因子平台、数据流水线、实时看板和金融 SDK。团队可以在一个核心里维护数值语义，再通过不同语言与产品形态复用，而不是在每个系统里重复实现同一套指标和统计逻辑。

当前正式发布版本为 v0.1.15；Unified Runtime、typed Artifact、DirtyRange 局部执行与扩展后的 Factor Research 正在 PR #29 中作为下一版本候选能力完成 same-SHA 门禁验证。

## 长篇宣传稿

很多量化项目最初都从几个指标函数开始。

SMA、EMA、RSI、MACD 很快就能算出来；再往后，团队开始加入公式引擎、选股逻辑、因子、特征工程、实时行情、回测统计和多语言服务。此时真正困难的往往不再是“还缺哪个指标”，而是同一套金融计算逻辑逐渐分裂成多套实现。

研究员在 Python 里写一套，实时服务在 Rust 或 Java 里重写一套，因子平台维护自己的 DAG，流式计算维护自己的状态，SDK 又复制一套包装逻辑。随着系统变大，结果会发生语义漂移，依赖关系被重复发现，缓存彼此孤立，局部数据修订也可能导致整段历史重新计算。

**Finkit 希望解决的是这个更底层的问题：把金融计算本身做成可以长期复用的基础设施。**

Finkit 以 Rust 作为 canonical compute core，把技术指标、Formula、Factor、Streaming、Feature 与 Research 能力围绕统一数值合同组织。Python、CLI 以及其他语言绑定是产品交付层，而不是算法的第二份实现。

对于重复使用的公式和因子，Finkit 不满足于“每次调用一个函数”。公式可以提前编译，因子依赖可以形成 `FactorPlan`，执行时复用已经验证过的依赖结构。下一版本候选架构进一步把 FactorPlan 接入 `UnifiedRuntime`，并使用 typed `ArtifactHash` 统一研究产物身份。

数据发生局部修改时，Finkit 也不会简单把接口命名为 `range` 就宣称支持增量执行。`DirtyRange` 会区分输入变化、受影响输出和历史 recompute window；只有完整依赖链都能证明 fixed-lookback、incremental、time-series safe 时，Runtime 才执行局部重算。横截面或动态 lookback 节点则自动回退到 full execution。

这体现了 Finkit 的核心工程观：**性能优化必须建立在正确性证明之上。**

在性能层面，Finkit 已经围绕 SIMD、borrowed/zero-copy 输入、caller-owned `_into` 输出、persistent compiled plan、streaming state、buffer reuse 与局部重算持续收敛。但项目不会把一台机器上的单次 benchmark 数字包装成普遍承诺，而是通过专用 performance regression、参考值/parity、Clippy、SSOT、跨语言构建和发布门禁来验证结果。

在产品层面，Finkit 的目标也已经从“技术分析库”扩大为一套 Quant Compute Runtime：你可以把它放在研究 Notebook 后面，也可以放进行情 API、选股服务、因子平台、实时分析服务或多语言 SDK 中。它不负责账户、订单和券商连接，而专注做好这些系统共同依赖的计算层。

对于只需要偶尔调用几个常见 TA 指标的用户，传统指标库可能已经足够；但对于需要长期维护研究、实时、因子与多语言产品的团队，Finkit 更关心的是如何让这些能力共享同一个核心，而不是继续增加新的重复实现。

## 面向不同用户的表达

### 面向量化研究员

你可以把 Finkit 看作一套比传统 TA 库更完整的研究计算底座：不仅有指标，还有 Formula、Feature、Label、Factor、统计、验证和 Factor Research 能力，并且这些模块在逐步共享同一个 Runtime。

### 面向工程团队

Finkit 的重点是减少跨模块、跨语言重复实现。算法和正确性合同集中在 Rust 核心，Formula/Factor 可以提前形成计划，Streaming 与 DirtyRange 负责增量更新，多语言绑定负责接入产品。

### 面向产品团队

如果你的产品需要行情计算、技术分析、选股、因子、研究统计或实时看板，Finkit 可以作为独立的计算服务或嵌入式引擎，而无需把产品绑定到某个券商或交易框架。

### 面向开源社区

Finkit 欢迎围绕指标 parity、Runtime、Streaming、Factor Research、多语言 SDK、benchmark 与文档参与贡献。项目更重视可复用实现和可验证门禁，而不是简单堆叠 API 数量。

## GitHub About 推荐描述

英文推荐：

> Rust-powered quantitative finance compute engine for indicators, formulas, factor graphs, streaming analytics, research workflows, and multi-language products.

短版：

> Quant compute runtime for research, realtime analytics, factors, and multi-language SDKs.

中文版：

> Rust 驱动的量化金融计算引擎，统一指标、公式、因子、流式分析、研究工作流与多语言交付。

## 官网 / README Hero 推荐文案

**标题**

> One compute core for quantitative research and realtime analytics.

**中文标题**

> 一套计算核心，连接量化研究与实时分析。

**副标题**

> Finkit uses a canonical Rust core to unify indicators, formulas, factor graphs, streaming computation, research analytics, and multi-language delivery.

**中文副标题**

> Finkit 以统一 Rust 核心连接指标、公式、因子 DAG、流式计算、研究分析和多语言产品交付。

## 发布公告模板

### 社区版本 A：技术导向

Finkit 正在从高性能技术分析库进一步收敛为统一 Quant Compute Runtime。下一版本候选架构把 typed Artifact、FactorPlan、Unified Runtime 与真正的 DirtyRange 局部执行连到同一主链，同时继续保持跨语言、正确性和性能门禁。当前正式版本仍为 v0.1.15，新架构会在 same-SHA 全矩阵验证完成后再进入发布判断。

### 社区版本 B：产品导向

如果你的量化项目已经出现“研究代码一套、实时服务一套、因子平台一套、SDK 又一套”的问题，Finkit 想做的是把这些计算重新收回一个核心。它不仅计算指标，也提供公式、因子、Streaming、Feature、Research 和多语言接入能力，让金融计算成为可复用基础设施。

### 社区版本 C：简短版

Finkit：Rust 驱动的 Quant Compute Runtime。一个核心统一指标、Formula、Factor、Streaming、Research 和多语言 SDK。

## 社交媒体短文案

**极短版**

> Finkit = Rust Quant Compute Runtime：指标 + Formula + Factor + Streaming + Research + Multi-language。

**产品版**

> 不想再为 Notebook、服务端和 SDK 重写三遍同一个指标？Finkit 用一个 Rust 核心统一金融计算语义，并把 Formula、Factor、Streaming 与 Research 接到同一执行体系。

**工程版**

> Finkit 正在把 FactorPlan、typed Artifact 与 DirtyRange 接入 Unified Runtime：只有依赖链证明安全时才局部执行，不能证明就 full fallback。性能优化先服从正确性。

## 可以宣传的事实与需要避免的说法

可以明确说明：Finkit 使用 Rust 核心；提供指标、Formula、Streaming、Feature、Factor 等能力；v0.1.15 是当前正式发布版本；下一版本 Runtime/Research 能力正在 PR #29 验证；Python/Rust/CLI 与其他绑定存在不同发布状态；项目通过 CI、正确性和性能门禁验证核心路径。

在没有对应公开证据前，不应宣传“所有语言均已在公共 registry 发布”“所有指标都比 TA-Lib 快”“完全替代 Alphalens”“所有工作负载都支持增量执行”或固定延迟/吞吐承诺。

## 推荐 CTA

- **想快速开始**：阅读 Getting Started，先完成第一个指标或 Formula 计算。
- **想评估产品适配度**：阅读《Finkit 产品说明》。
- **想了解 Runtime 重构**：阅读 Runtime 与 Factor / Factor Research Architecture。
- **想对外介绍项目**：直接使用《品牌与媒体素材》。
- **想贡献代码**：从 correctness、performance、bindings 或 docs 门禁开始。

## 相关链接

- [README 中文版](../README.zh-CN.md)
- [产品说明](product-overview-zh.md)
- [品牌与媒体素材](media-kit-zh.md)
- [完整文档索引](README.md)
- [Runtime 与 Factor](runtime-and-factors.md)
- [Factor Research 架构](factor-research-architecture.md)
- [多语言绑定](language-bindings.md)
