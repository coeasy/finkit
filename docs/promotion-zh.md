# Finkit 项目宣传文稿

> 本文是面向 GitHub、技术社区、博客、公众号、Release Announcement 和项目介绍页面的宣传素材。技术事实以当前 README、文档与正式 Release 为准。

## 推荐标题

### 主标题

**Finkit：用 Rust 打造统一的量化金融计算引擎**

### 备选标题

- **从技术指标到因子研究：Finkit 正在把量化计算收敛到一个统一 Runtime**
- **一套 Rust 内核，多语言量化计算：Finkit 的产品化升级**
- **不只是 TA-Lib 替代：Finkit 想解决量化系统里的重复计算与语义漂移**
- **指标、公式、因子、流式、研究一次打通：Finkit 的统一计算架构**

## 30 秒项目介绍

Finkit 是一个以 Rust 为统一内核的高性能量化金融计算引擎。

它不是简单堆叠技术指标，而是试图把量化系统中长期分散的几条计算链路统一起来：

**技术指标 → Formula → Factor DAG → Streaming → Feature Engineering → Factor Research → 多语言 SDK。**

Finkit 的目标是让同一套金融计算语义只实现一次、验证一次、优化一次，再通过 Python、Rust、Node.js、Java、C/C++、Go、.NET、Android、iOS 和 WASM 等不同入口复用。

如果你正在搭建量化研究平台、行情分析服务、选股系统、因子服务、实时看板、数据流水线或跨语言金融 SDK，Finkit 希望成为其中稳定、可复用的计算底座。

## 正式宣传稿

在很多量化项目里，真正难维护的并不是“少几个指标”，而是同一套计算逻辑在不同系统里不断复制。

研究员在 Python 里写一版，生产服务在 Rust/C++ 里再写一版；前端或 Node 服务又有一套；实时计算和历史批量计算使用不同代码；Formula 有自己的缓存，Factor 有自己的 DAG，Feature 又维护一套统计函数。时间一长，性能问题、数值差异、边界行为和版本兼容开始一起出现。

**Finkit 想解决的正是这个问题。**

### 一个 Rust Core，而不是多套算法分支

Finkit 将 Rust 作为 canonical computation core。

指标、统计、收益率、回归、排序、风险等核心数学语义集中维护，多语言绑定负责“暴露能力”，而不是各自重新实现算法。

这样做带来的价值不只是性能，更重要的是：

- 同一输入在不同语言中保持一致语义；
- 修复一次 bug，不需要同步修改多套算法；
- 性能优化可以直接惠及多个绑定；
- CI 可以围绕统一内核建立真正的 parity 和回归门禁。

### 从函数调用升级到可复用执行计划

传统指标库通常是：调用一次函数，得到一次结果。

Finkit 在此基础上继续向“运行时”方向演进。

Formula 可以预编译，Factor 依赖图可以提前解析，执行顺序可以提前验证，中间缓冲区和状态可以复用。对于重复计算场景，不再每次从头解析、从头建图、从头分配。

这让 Finkit 更适合：

- 行情扫描；
- 批量因子计算；
- 长时间运行的分析服务；
- 高频重复请求；
- 实时流式更新；
- Notebook → 服务化迁移。

### 真正的 DirtyRange，而不是“名字叫增量”

Finkit 新一轮架构升级中，一个重要方向是 Unified Runtime + DirtyRange。

历史数据发生修订时，真正需要回答三个问题：

1. 哪些输入行变了？
2. 这些变化会影响未来多少输出？
3. 为了重新计算这些输出，需要向历史补多少 lookback？

Finkit 把这三件事分别建模为 input dirty、affected output 和 recompute window。

只有完整依赖链都能证明是 incremental、fixed-lookback、time-series safe 时，才允许局部重算。

如果出现 cross-sectional、动态 lookback 或无法确认的执行语义，Runtime 会回退到 full execution。

**性能优化不能以悄悄算错为代价。**

### 从技术分析扩展到 Factor Research

Finkit 已经拥有大量指标、Formula、Streaming、Feature、Risk、Backtest、Calendar 等基础能力。

新的 Factor Research 架构不会重新造一套研究框架，而是把这些已有能力重新组合：

```text
MarketFrame / external data
        ↓
Indicators / Formula / Factor
        ↓
Feature / Research data
        ↓
Prepare / Validate / Analyze
        ↓
Multi-Factor / Neutralization / Portfolio / Risk
        ↓
Research Artifact / Report
```

研究层重点覆盖：

- forward returns；
- quantile / group analysis；
- IC / rank IC；
- turnover / stability；
- neutralization；
- multi-factor；
- portfolio / attribution；
- risk / capacity；
- validation / no-lookahead；
- research report / artifact。

核心原则仍然是 reuse-first：收益率、回归、排序、分位数、风险、日历等能力必须复用 canonical kernel，而不是研究模块再复制一遍。

### 批量与实时计算都要

Finkit 同时重视 batch 与 streaming。

对于历史计算，批量 kernel 可以利用 SIMD、连续内存和 `_into` 输出复用。

对于实时计算，Streaming Indicator 保留状态，只处理新 Bar。

对于历史修订，DirtyRange 提供安全的局部重算路径。

三种执行模型不应该是三套不相关代码，而应该在相同数学语义下互相验证。

### 多语言不是“有绑定就算完成”

Finkit 的另一个产品化原则是：明确区分交付状态。

项目把语言支持拆成：

- Source exists；
- CI validated；
- Package candidate；
- GitHub Release asset；
- Public registry package。

这意味着仓库里有 Node/Go/.NET/Android/iOS 代码，并不自动等于 npm/NuGet/Maven/Swift Package 已正式发布。

对用户来说，这比“支持十几种语言”一句口号更重要：**你可以准确知道哪些路径已经真正构建、测试、打包和发布。**

## Finkit 适合谁

### 量化研究员

希望把指标、Formula、Feature、Factor、Research Analysis 放在同一套语义和工具链里。

### 量化平台团队

希望构建选股、行情分析、因子服务、研究服务或计算 API，并减少重复算法实现。

### 实时行情产品

需要 Streaming Indicator、可复用执行计划和低开销更新机制。

### 数据与机器学习团队

需要金融特征、标签、rolling statistics、PCA、MI、CV、regime/stability 等基础能力。

### 多语言产品团队

希望同一套金融算法可以被 Python、Node、Java、C++、Go、.NET、移动端和浏览器调用。

## Finkit 与传统指标库有什么不同？

传统指标库最重要的问题通常是：

> “有没有这个指标？”

Finkit 更关注：

> “这个计算能不能被长期复用、重复执行、增量更新、跨语言交付、研究验证，并保持相同语义？”

所以项目重点不只是继续增加函数数量，而是持续收敛：

- canonical kernel；
- Unified Runtime；
- ComputePlan / FactorPlan；
- BufferArena；
- DirtyRange；
- Artifact identity / provenance；
- SSOT metadata；
- multi-language release gates。

## 当前版本

当前正式发布版本为 **v0.1.15**。

该版本的权威分发来源是 GitHub Release。已发布资产与其他语言的 CI/源码状态请以 README 与 language-bindings 文档为准。

正在开发的下一版本重点推进 Unified Runtime、typed Artifact、DirtyRange、Factor Research 与多语言交付链路。

## 我们坚持的工程原则

### 不用 mock 掩盖真实链路

发布门禁要验证真实构建、真实包、真实运行路径。

### 不拼接不同 SHA 的 CI 结果

只有同一个最终 SHA 的门禁全部绿色，才有资格作为发布证据。

### 不为了性能降低正确性

无法证明安全的 incremental execution 就 full fallback。

### 不重复造轮子

已有 canonical kernel 就复用，发现重复实现就收敛。

### 不让文档超前于发布事实

CI candidate 和 public package 是两件事。

## GitHub About 推荐描述

推荐使用：

> **Rust-powered quantitative finance engine for indicators, formulas, factors, streaming analytics, factor research, and multi-language SDKs.**

更短版本：

> **High-performance multi-language quantitative finance engine powered by Rust.**

中文一句话：

> **基于 Rust 的高性能多语言量化金融计算引擎，覆盖指标、公式、因子、流式分析与因子研究。**

## 社区发布短文案

### 版本 A：技术社区

Finkit 正在从“高性能技术指标库”升级为统一的量化金融计算引擎。

项目以 Rust 为 canonical core，继续打通 Indicators / Formula / Factor / Streaming / Feature / Factor Research，并通过 Python、Node、Java、C/C++、Go、.NET、移动端和 WASM 输出同一套数值语义。

新架构重点包括 Unified Runtime、typed Artifact、FactorPlan、DirtyRange 局部执行和 reuse-first Factor Research。增量计算只有在依赖链能够证明安全时才启用，否则自动 full fallback。

如果你在做量化研究平台、行情分析、选股、因子服务、实时计算或金融 SDK，欢迎关注和参与 Finkit。

### 版本 B：简短介绍

Finkit：一个 Rust 驱动的多语言量化金融计算引擎。

不仅有指标，还在统一 Formula、Factor DAG、Streaming、Feature Engineering 和 Factor Research。目标是让金融计算只实现一次、优化一次、验证一次，再跨语言复用。

### 版本 C：产品介绍

用一个 Rust Core，支撑从研究 Notebook 到生产分析服务。

Finkit 提供指标、公式、因子、流式、特征和研究计算，并持续通过 Unified Runtime、DirtyRange 和 typed Artifact 收敛重复执行与重复实现问题。

## 社交媒体超短文案

**Finkit = Rust Core + Indicators + Formula + Factor + Streaming + Research + Multi-language.**

不是只追求“更多指标”，而是把量化计算做成可复用、可验证、可增量、可跨语言交付的基础设施。

## 推荐配套链接

对外宣传时建议同时提供：

- GitHub 主仓库；
- README / 中文 README；
- Product Overview；
- Getting Started；
- Runtime & Factors；
- Factor Research Architecture；
- Release 页面；
- Benchmark 文档。

## 宣传时避免的表述

为了保证对外信息可信，不建议使用：

- “所有平台都已发布”——实际不同语言发布状态不同；
- “100% TA-Lib 完全兼容”——除非最终完整 parity gate 明确证明；
- “永远比 TA-Lib 快 X 倍”——性能依赖机器和 workload；
- “完整实盘交易平台”——Finkit 不负责 OMS/券商接入；
- “零拷贝所有路径”——只在明确 ownership/contiguous input 契约下成立；
- “所有因子都支持局部增量”——只有 range-safe 依赖链才能 DirtyRange 执行。

可信的产品宣传应该和代码、CI、Release 保持同一事实源。
