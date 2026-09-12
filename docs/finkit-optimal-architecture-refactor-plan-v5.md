# Finkit 最优化统一架构与重构方案 V5

> 状态：Approved Target Architecture / Implementation Baseline
>
> 基线：`feat/factor-research-architecture` / PR #29
>
> 评审基线 SHA：`eec0ab46bcdaac9bd6fcb5905aa950149e5e0651`
>
> 日期：2026-09-11
>
> 本文档作为后续重构的目标架构基线，优先级高于历史“继续扩指标/继续扩研究函数”的局部式演进方案。除兼容迁移外，新功能必须服从本文的模块所有权、依赖方向、执行主链和发布门禁。

---

## 0. 最终目标与边界

Finkit 的统一产品目标正式定义为：

> **统一量化计算内核 + Formula/Runtime + 因子研究引擎 + 可视化 + 多语言 SDK 的量化计算/研究基础设施。**

目标不是继续堆积独立 API，而是形成一套可复用、可组合、可增量、可缓存、可验证、可跨语言稳定发布的统一执行体系。

### 0.1 必须覆盖的五大能力域

1. **Quant Kernel**：指标、统计、收益、排名、回归、滚动计算、窗口、横截面、状态型算法的唯一数值内核。
2. **Formula / Runtime**：公式解析、DAG、ComputePlan、Buffer/State 生命周期、批量/流式/增量执行。
3. **Factor Research Engine**：Panel 数据、forward return、分层、IC、收益、换手、组合、验证、风险、事件、稳定性、多因子研究和报告。
4. **Visualization**：消费稳定的研究/计算 artifact，而不是重新计算研究逻辑。
5. **Multi-language SDK**：Rust 为语义 SSOT，向 Python / Node / Go / C/C++ / Java / .NET / iOS / Android / WASM 暴露统一版本化合同和高性能数据通道。

### 0.2 明确不做

Finkit 不直接拥有：

- 行情抓取、数据爬虫和数据供应商连接器；
- 券商登录、订单路由、实盘交易执行；
- OMS/EMS；
- 完整资管交易平台；
- 以 Python 生态对象为中心的第二套独立研究实现。

外部系统可把数据交给 Finkit，也可消费 Finkit 结果，但不反向污染 Finkit 核心计算边界。

---

## 1. 已确认的最优化架构决策

以下决策按最优化方案直接作为正式原则，不再保留为待确认项。

### D1. 一个语义，一个 canonical owner

同一个数学/指标/研究语义只能有一个真正算法实现。其他层只能 delegate / facade / adapter。

禁止：

```text
core 实现 A
factor-analysis 再实现 A
Python 再实现 A
WASM 再实现 A
```

允许：

```text
canonical kernel A
   ├─ core public facade
   ├─ Formula operator
   ├─ research service
   ├─ Python adapter
   └─ WASM adapter
```

### D2. Rust 是语义 SSOT，但不是所有层都塞进 core

Rust 持有计算和研究语义 SSOT；同时通过清晰模块边界避免 `core` 成为无限膨胀的“大一统 crate”。

先进行**逻辑所有权重构**，再依据编译时间、依赖图和增量构建数据决定是否物理拆 crate，不为了“看起来模块化”立即大拆 workspace。

### D3. ResearchPlan 必须成为真实执行计划

不得长期维持：

```text
ResearchPlan = 描述 DAG
FactorStudy::full_report = 另一套真实手工流程
```

最终必须统一为：

```text
ResearchRequest
    -> ResearchContext
    -> ResearchPlan
    -> ResearchExecutor
    -> ArtifactStore
    -> Report / Visualization / SDK
```

### D4. Batch / Streaming / Incremental 共用一套语义和执行器

三种模式的区别只能是数据可见范围、dirty range、状态恢复和 materialization 策略，不应成为三套算法体系。

### D5. Alphalens 采用“结果语义兼容”

目标是：

- 指标定义一致；
- 分组/去均值/分层语义一致；
- 输出指标含义一致；
- 关键结果可建立 golden parity。

不要求：

- 复制每一个历史 Python 对象；
- 复制所有 plotting API；
- 复制低效的数据布局；
- 在 Python 重新实现一套算法。

### D6. API 兼容一个发布周期

重构允许替换内部架构和改进 V2 公共合同，但旧公开 API 原则上保留至少一个发布周期的 deprecated compatibility facade。

兼容 facade 不允许持有独立算法体。

### D7. Report V2 采用 summary-first + artifact 引用

公共报告不再默认把所有大数组重复嵌入 JSON。

目标：

```text
ReportSummary
  + typed scalar/table summaries
  + ArtifactRef(s)
  + optional inline small artifacts
```

旧 V1 JSON 在兼容周期内继续可读取/生成。

---

## 2. 当前架构的核心问题

当前 workspace 已包含 `core`、`factor-analysis`、`visualization`、CLI、WASM 以及多类 FFI binding，能力覆盖很广，但结构上仍存在“功能先实现、主链后补”的历史痕迹。

### P0.1 ResearchPlan 不是执行 SSOT

`factor-analysis/src/orchestration.rs` 已经使用 core `ComputePlan` 表达 ResearchStage DAG，但它目前主要完成依赖排序和能力描述。

而标准研究入口仍在 `FactorStudy::full_report()` 中手工依次调用：

- forward returns；
- quantization；
- weights；
- factor returns；
- IC；
- mean IC；
- alpha/beta；
- quantile returns；
- turnover；
- rank autocorrelation；
- portfolio performance；
- report build。

这造成两个 workflow truth。

### P0.2 Artifact/Cache 语义不够强

现有 `ResearchCache<V>` 已经比早期 `Vec<f64>` 方案更进一步，`ResearchArtifactKey` 也包含 revision/fingerprint 信息，这是正确方向；但还缺：

- node/stage identity；
- output kind；
- schema/algorithm version；
- parameter hash；
- dependency artifact fingerprints；
- partial-range/materialization 信息；
- typed persistence policy。

因此它还不是完整的研究 ArtifactStore。

### P0.3 Incremental 与主执行器平行

`IncrementalForwardReturnEngine` 能正确跟踪 forward-return maturity，但仍是专用状态机。

它应该成为统一 executor 中 `ForwardReturns` node 的 incremental strategy，而不是研究系统旁边另一条执行通路。

### P0.4 高级研究功能“存在”但没有统一 profile/plan

多因子、Fama-MacBeth、validation、event、stability、mining、risk model、scenario 等能力已经大量存在，但当前并没有统一回答：

> 哪个研究请求需要执行哪些 stage？哪些 stage 可以共享 artifact？哪些是 optional profile？

这会导致功能越多，编排越分散。

### P0.5 FFI common 已承担高层业务合同

`ffi-common` 同时负责：

- error；
- registry；
- numeric conversion；
- golden vectors；
- factor research request/response；
- quantitative evaluation contract。

它已经不是纯 low-level FFI helper。继续扩展会造成边界混乱。

### P0.6 Visualization 尚未完全 artifact-driven

可视化必须消费稳定的 report/artifact contract。任何图表若直接重新计算因子研究算法，就会产生第三套语义实现。

### P0.7 当前发布门禁仍未闭合

评审基线 SHA 上，Workspace compile、Clippy、Memory/Performance、Docs、Python wheels、Multilang cross-platform 已通过；但仍存在：

- Format 失败；
- core tests 失败；
- Factor Research SSOT 失败；
- Visualization integration 失败；
- Go release 测试失败；
- 因此 Multilang release 未完成。

所以“架构设计继续推进”与“版本可以发布”必须严格分离。

---

## 3. 最优目标架构

### 3.1 总体分层

```text
+--------------------------------------------------------------+
| L8  User SDK / CLI / WASM / Mobile / Language Bindings       |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L7  Versioned API Contract                                   |
|     Request / Response / ErrorEnvelope / ArtifactRef         |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L6  Presentation                                              |
|     Report Builder / Visualization Adapter / Export          |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L5  Research Orchestration                                   |
|     ResearchProfile -> ResearchPlan -> ResearchExecutor      |
|                    -> ArtifactStore -> Provenance             |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L4  Research Services                                        |
|     Prepare / IC / Returns / Turnover / Portfolio / Risk     |
|     Multifactor / Validation / Event / Stability / Scenario  |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L3  Research Model & Policy                                  |
|     PanelIndex / ResearchFrame / ResearchPolicy / Universe   |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L2  Formula & Runtime                                        |
|     FormulaGraph / ComputePlan / ExecutionPlan               |
|     BufferArena / StateArena / DirtyRange / Scheduler        |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L1  Domain Operators                                         |
|     Indicators / Features / Transforms / Rolling / CrossSec   |
+-------------------------------+------------------------------+
                                |
+-------------------------------v------------------------------+
| L0  Canonical Quant Kernel                                   |
|     math / rank / return / regression / stats / window       |
+--------------------------------------------------------------+
```

### 3.2 依赖铁律

只允许向下依赖：

```text
SDK -> Contract -> Presentation/Research -> Runtime -> Kernel
```

禁止：

- core/kernel 依赖 factor-analysis；
- core 依赖 visualization；
- research service 依赖某个语言 binding；
- visualization 直接调用语言 binding；
- SDK 内实现研究算法；
- Formula parser 层持有数值算法副本。

---

## 4. Canonical Quant Kernel 重构

目标：所有可复用数值语义有明确 owner，并支持 batch/into/stateful 复用。

### 4.1 Canonical owner 分类

建议逻辑所有权：

```text
math/
  statistics
  rank
  regression
  information
  reduction
  numeric

returns/
  arithmetic/log return
  forward/backward return primitive

window/
  extrema
  moments
  welford
  weighted
  rolling regression

indicators/
  只做领域组合与 TA-Lib 语义

features/
  只做 feature 语义组合
```

### 4.2 Kernel API 统一形态

每类适用算法尽量形成三层：

```text
compute(input) -> owned output
compute_into(input, &mut output)
State::update(value) / append(...)
```

并保证三者调用相同 canonical logic。

### 4.3 热路径约束

- 热循环禁止字符串 dispatch；
- 热循环禁止重复 operation lookup；
- 可预分配路径禁止每次创建临时 Vec；
- Python 数值输入优先 zero-copy/borrowed view；
- Formula compute-many 必须复用公共子表达式；
- rolling/extrema/WMA/Welford 等共享统一 kernel；
- SIMD 只用于 benchmark 能证明收益、并有正确性 fallback 的路径。

### 4.4 SSOT Gate 升级

现有 Research SSOT gate 应扩大为 canonical ownership gate：

1. 注册 semantic operation；
2. 注册 canonical owner path；
3. facade 可声明 delegate target；
4. 自动扫描同名/同语义算法体；
5. 禁止 facade 内出现独立循环算法实现；
6. 新增 kernel 必须声明 owner。

---

## 5. Formula / Runtime 统一设计

Formula/Runtime 是整个项目的执行基础设施，不只是字符串公式执行器。

### 5.1 编译阶段

```text
Formula / Program / Research Node
        ↓
Semantic IR
        ↓
Dependency DAG
        ↓
CSE
        ↓
Lifetime analysis
        ↓
ExecutionPlan
```

编译期完成：

- operation 解析；
- 参数归一化；
- 类型/shape 检查；
- lookback 分析；
- 公共子表达式消除；
- state slot 分配；
- scratch 生命周期分析；
- BufferArena slot 复用；
- streaming/incremental capability 标注。

### 5.2 执行阶段

热路径只看到：

```text
KernelId
BufferId
StateSlot
Range
Parameters
```

而不是字符串名称和动态解析。

### 5.3 BufferArena 与 StateArena 必须分离

- `BufferArena`：短生命周期 scratch / materialized vectors；
- `StateArena`：EMA、ATR、rolling state、streaming state 等跨调用状态；
- ArtifactStore：研究级、可复用、可缓存输出。

三者不能混成一个“万能 cache”。

### 5.4 Unified execution modes

统一接口概念：

```text
execute_full(plan, data)
execute_range(plan, data, range)
execute_last(plan, data)
append(plan, delta)
recompute_dirty(plan, dirty_range)
```

它们共享同一个 plan 和 kernel registry。

---

## 6. Factor Research Engine 最优设计

### 6.1 新的核心对象

最终建议形成：

```text
ResearchRequestV2
ResearchContext
ResearchPolicy
ResearchProfile
ResearchPlan
ResearchExecutor
ResearchArtifactStore
StudyProvenance
FactorStudyReportV2
```

### 6.2 ResearchRequestV2

Request 只表达用户意图，不携带执行细节。

建议字段组：

- schema version；
- dataset/panel identity；
- factor columns；
- price/return inputs；
- horizon；
- universe/group；
- quantization policy；
- neutralization policy；
- weighting policy；
- execution lag；
- evaluation policy；
- research profile；
- requested outputs；
- optional deterministic seed。

### 6.3 ResearchContext

统一承载执行上下文：

```text
ResearchContext {
  frame/view,
  revision,
  calendar,
  universe,
  policy,
  provenance,
  executor resources,
}
```

`FactorStudy::new` 最终只负责建立/验证 context，不执行大规模计算。

### 6.4 ResearchPolicy

当前分散在 QuantizeConfig、WeightConfig、EvaluationConfig、mode、lag 等位置的语义应统一进入 policy tree。

例如：

```text
ResearchPolicy
  ├─ ReturnPolicy
  ├─ QuantilePolicy
  ├─ GroupPolicy
  ├─ NeutralizationPolicy
  ├─ WeightPolicy
  ├─ ExecutionPolicy
  ├─ CostPolicy
  ├─ RiskPolicy
  └─ CompatibilityPolicy
```

好处：

- fingerprint 稳定；
- cache key 可重现；
- FFI 合同更清楚；
- profile 可组合；
- 避免参数在不同函数层漂移。

### 6.5 ResearchProfile

不要让“full_report”无限膨胀。

提供标准 profile：

```text
Core
  ForwardReturns + Quantize + IC + FactorReturns + Turnover

Alphalens
  Core + Alphalens semantic outputs

Portfolio
  Core + Weights + Overlapping Portfolio + Cost + Performance

Validation
  Core + Purged/Embargo/CPCV/WalkForward + inference

Risk
  Core + RiskModel + Attribution + Scenario

Full
  union of stable supported profiles

Custom
  explicit requested stages
```

### 6.6 ResearchPlan

Plan 必须真正决定执行：

```text
Profile/RequestedOutputs
        ↓
Stage expansion
        ↓
Dependency closure
        ↓
Capability annotation
        ↓
ComputePlan compile
        ↓
ResearchExecutionPlan
```

Stage 不应只保存 `kind + id + dependencies`，还要逐步增加：

- canonical operation id；
- parameter fingerprint；
- output artifact type；
- cacheability；
- incremental strategy；
- memory estimate；
- deterministic flag；
- version。

### 6.7 ResearchExecutor

这是 V5 架构的中心。

职责：

1. 按 plan 执行；
2. 查询 ArtifactStore；
3. 计算 dependency closure；
4. 进行 dirty-range propagation；
5. 选择 full/range/incremental strategy；
6. materialize typed artifacts；
7. 收集 provenance/timing/diagnostics；
8. 构建 report 所需 artifact set。

必须避免把所有逻辑再次塞入一个巨大 `match ResearchStageKind`。每个 stage 使用注册的 executor/operator。

---

## 7. Typed Research ArtifactStore

### 7.1 Key

建议：

```text
ArtifactKey {
  data_revision,
  data_fingerprint,
  plan_fingerprint,
  stage_id,
  operation_id,
  parameter_fingerprint,
  policy_fingerprint,
  calendar_fingerprint,
  universe_fingerprint,
  algorithm_version,
  schema_version,
}
```

### 7.2 Value

不要退回万能 `Vec<f64>`。

采用 typed enum/trait-object/typed handle 中的安全方案，例如：

```text
ResearchArtifact
  NumericSeries
  PanelSeries
  Quantiles
  Weights
  ForwardReturns
  InformationCoefficient
  RegressionResult
  Turnover
  PortfolioResult
  RiskModel
  ScenarioResult
  Table
  ReportSection
```

大对象通过 `ArtifactId/ArtifactRef` 引用。

### 7.3 生命周期

区分：

- Ephemeral：仅当前执行；
- Session：同一 ResearchContext 复用；
- Revision：同一数据 revision 复用；
- Persistent：可序列化持久化。

### 7.4 缓存策略

第一阶段保持 deterministic bounded cache，不要过早引入复杂分布式 cache。

优先保证：

- key 正确；
- 不错误复用；
- revision invalidation 正确；
- dependency invalidation 正确；
- memory budget 可控。

然后再优化 LRU/size-aware/persistence。

---

## 8. Incremental / Streaming 统一

### 8.1 DirtyRange 是核心概念

数据变化应转化为：

```text
DataDelta
   -> Revision++
   -> DirtyRange / DirtyAssets / DirtyColumns
   -> dependency propagation
   -> selective recompute
```

### 8.2 Forward return maturity

保留现有 `IncrementalForwardReturnEngine` 的成熟逻辑，但把它下沉为 ForwardReturns stage 的状态实现。

对 append：

- 新 session 只成熟对应 horizon 的历史 origin；
- 不重算整个 panel；
- downstream IC/portfolio 只更新受到影响的 date/range。

对 replace/backfill：

- 精确计算影响范围；
- 无法安全局部更新的 stage 显式 fallback full recompute；
- fallback 是 executor 策略，不由业务调用方手工决定。

### 8.3 Stateful correctness gate

所有增量实现必须满足：

```text
full(data_after_update)
==
incremental(data_before + delta)
```

在容差和 NaN 规则下建立 property/golden test。

---

## 9. Research Services 内部重构

高级研究功能保留，但取消“模块存在即算接入”的判断标准。

### 9.1 Cross-sectional execution primitive

IC、rank、quantile、neutralization、weights、Fama-MacBeth 等反复做 date/group 切片。

建立统一：

```text
CrossSectionView
SegmentIndex
GroupSlice
DateSlice
CrossSectionExecutor
```

目标：

- 一次构建 segment index；
- 多分析共享；
- 降低排序/分配；
- group/date 语义统一。

### 9.2 Validation 内部分层

将“大 validation 模块”逻辑拆成：

```text
validation/
  split
  purge
  embargo
  cpcv
  walk_forward
  bootstrap
  permutation
  multiple_testing
```

先逻辑拆模块，是否拆 crate 以后再定。

### 9.3 Risk / attribution / scenario

统一数据模型：

```text
ExposureMatrix
CovarianceModel
FactorReturnSeries
SpecificRisk
PortfolioExposure
ScenarioShock
AttributionResult
```

避免 risk model 和 scenario 各自定义重复矩阵语义。

---

## 10. Report / Visualization 最优边界

### 10.1 Report V2

建议结构：

```text
FactorStudyReportV2
  metadata
  provenance
  quality_summary
  factor_summary
  ic_summary
  return_summary
  turnover_summary
  performance_summary
  optional risk_summary
  optional validation_summary
  artifacts[]
  diagnostics[]
```

大数组不默认全部 inline。

### 10.2 ArtifactRef

```text
ArtifactRef {
  id,
  kind,
  rows,
  columns,
  dtype,
  content_hash,
  storage,
}
```

FFI JSON 可以仅返回 summary + refs；本地 SDK 可以提供 typed accessor 拉取 artifact。

### 10.3 Visualization

Visualization 只能做：

```text
Report/Artifact
  -> ViewModel
  -> ChartSpec
  -> renderer/export
```

禁止：

```text
Visualization
  -> 重算 IC / factor return / quantile
```

这样 HTML/WebGPU/未来 GUI 都不会产生计算语义漂移。

---

## 11. Multi-language SDK 最优化方案

### 11.1 重新定义共享 contract 层

将高层版本化合同从低层 `ffi-common` 概念中独立出来。

逻辑结构建议：

```text
api-contract
  research request/response
  error envelope
  artifact metadata
  schema/version

ffi-common
  panic/error mapping
  primitive type conversion
  buffer ownership
  registry bridge
```

物理 crate 是否立即拆分可在 R6 决定；先保证逻辑依赖方向。

### 11.2 两条数据通道

**Control Plane**：JSON/serde，适合：

- request config；
- report summary；
- error；
- metadata。

**Data Plane**：typed/zero-copy/low-copy buffer，适合：

- large arrays；
- matrix；
- panel series；
- artifact materialization。

不要用超大 JSON 作为所有数据的永久最低公分母。

### 11.3 SDK 一致性

所有语言共享：

- schema version；
- error code；
- operation semantics；
- golden corpus；
- artifact fingerprint；
- compatibility tests。

语言层只做 ergonomic wrapper。

### 11.4 Python

重点：

- NumPy borrowed/zero-copy input；
- 避免 `Vec -> list -> ndarray`；
- ndarray 输出尽量一次 ownership transfer/copy；
- factor report 提供 Pythonic facade，但底层仍走 Rust SSOT。

### 11.5 Node/WASM

- TypedArray 通道；
- 避免 JSON 承载大 numeric arrays；
- browser/server 行为使用相同 schema。

### 11.6 Go/C/.NET/Java/Mobile

- 统一 ABI ownership；
- 明确谁分配、谁释放；
- panic 永不跨 FFI；
- enum/width ABI 显式化；
- packaged candidate 必须在 clean environment smoke test。

---

## 12. 性能架构

性能优化必须围绕“统一执行链”，不是单函数 benchmark 竞赛。

### 12.1 四级性能目标

**L0 Kernel**

- canonical hot kernels；
- no avoidable allocation；
- SIMD correctness fallback。

**L1 Plan**

- CSE；
- fused traversals；
- buffer reuse；
- cross-section segmentation reuse。

**L2 Research**

- artifact reuse；
- incremental materialization；
- only-requested-output execution。

**L3 SDK**

- low-copy input/output；
- avoid serialization bottleneck。

### 12.2 必须保留的 benchmark 类别

- TA-Lib parity + performance；
- kernel microbench；
- compute-many DAG bench；
- eval_range/eval_last；
- append/streaming；
- research panel end-to-end；
- full vs incremental；
- Python NumPy ingress/egress；
- FFI large-array overhead；
- memory peak/alloc count。

### 12.3 性能门禁原则

禁止为了 CI 通过：

- 放宽已有阈值来掩盖真实 regression；
- skip benchmark；
- 特判 CI 环境绕过；
- 换成不等价算法。

阈值若确实需要调整，必须有 benchmark 基线变更说明和证据。

---

## 13. 正确性、可重复性与 Provenance

### 13.1 StudyProvenance V2

至少包含：

- library version；
- schema version；
- algorithm versions；
- plan fingerprint；
- data fingerprint/revision；
- factor/universe/calendar fingerprints；
- policy/config fingerprint；
- seed；
- compatibility mode。

### 13.2 Determinism

默认研究 pipeline 必须 deterministic。

需要随机性的 bootstrap/permutation/mining 必须：

- seed 可指定；
- seed 写入 provenance；
- 测试固定 seed。

### 13.3 ErrorEnvelope

统一所有入口：

```text
code
message
stage/node
recoverability
optional details
```

公开 FFI 不泄漏 Rust panic 和实现细节。

---

## 14. CI / Release 架构

### 14.1 CI 不允许修改生产源码并 push

历史 one-shot fixer 只能作为临时诊断工具。

长期规则：

```text
Developer commit
    -> CI verify
    -> fail with exact evidence / pass
```

不能：

```text
CI mutates production source
    -> commits
    -> pushes
```

### 14.2 必需门禁分组

**Source gates**

- rustfmt；
- generated registry；
- SSOT ownership；
- schema consistency；
- version consistency。

**Correctness gates**

- core；
- formula/runtime；
- factor research；
- incremental equivalence；
- visualization contract；
- FFI golden corpus。

**Quality gates**

- Clippy `-D warnings`；
- docs；
- audit；
- fuzz/property suites（按 cadence）。

**Performance gates**

- zero-allocation contract；
- relative performance；
- selected hard hot-path thresholds。

**Packaging gates**

- Rust package；
- Python wheels；
- Node；
- Go；
- C/C++；
- Java；
- .NET；
- WASM；
- iOS；
- Android。

### 14.3 Same-SHA Gate

发布的唯一判据：

> **所有 required gates 必须在同一个最终 SHA 上通过。**

禁止把不同提交上的绿色结果拼成“整体绿色”。

---

## 15. 兼容迁移策略

### 15.1 Rust API

新 architecture API 稳定后：

- 旧 `FactorStudy::full_report()` 作为 deprecated facade；
- 内部改为构建默认 `ResearchProfile::Full` 并调用 ResearchExecutor；
- facade 保留一个发布周期；
- 下一主版本再考虑移除。

### 15.2 Request/Report Schema

- V1 request 继续接收；
- V2 request 作为 canonical；
- Report V1 保留兼容 serializer；
- Report V2 成为默认新接口；
- schema migration 必须有 golden samples。

### 15.3 Alphalens compatibility

compat 层只能映射：

```text
Alphalens-like request
  -> canonical ResearchPolicy/Profile
  -> ResearchExecutor
  -> compatibility-shaped response
```

不能出现独立计算分支。

---

## 16. 建议目录/模块结构

不要求一次物理拆 crate，先按以下逻辑边界收敛。

```text
core/
  math/
  returns/
  window/
  indicators/
  features/
  formula/
  compute/
  runtime/
  streaming/

factor-analysis/
  model/
    panel
    frame
    universe
    policy
  prepare/
  services/
    factor
    multifactor
    portfolio
    validation
    event
    stability
    mining
    risk
    scenario
  execution/
    profile
    plan
    executor
    artifact
    cache
    incremental
    provenance
  report/
  compat/
  api/

visualization/
  adapter/
  view_model/
  renderer/
  export/

ffi/
  contract/       # 先逻辑边界，可后续独立 crate
  ffi-common/
  *-binding/
```

---

## 17. 分阶段实施路线

## R0 — 恢复真实绿色基线

**目标**：在开始大规模重构前获得可信基线。

必须处理当前真实红灯：

1. rustfmt；
2. Research SSOT；
3. core HT_SINE/实际 core test failure；
4. visualization integration；
5. Go release binding；
6. 同一 SHA 重跑所有 required gates。

**DoD**：PR 保持 Draft，最终基线 SHA 全部门禁绿。

---

## R1 — Canonical ownership 收敛

**目标**：彻底消除重复算法归属。

任务：

1. 建立 semantic operation registry；
2. 标注 canonical owner；
3. rank/regression/returns/risk/rolling 等统一；
4. facade 只 delegate；
5. SSOT gate 覆盖 core/research/FFI/WASM；
6. 为 canonical kernels 补 `*_into` / stateful 统一路径。

**DoD**：新增相同语义实现时 CI 可自动阻断。

---

## R2 — ResearchExecutor 成为唯一执行主链

**目标**：消灭 ResearchPlan 与 `full_report()` 双流程。

任务：

1. 引入 ResearchContext；
2. 引入 ResearchPolicy；
3. 引入 ResearchProfile；
4. 扩展 ResearchStageSpec；
5. 实现 stage registry/operator；
6. 实现 ResearchExecutor；
7. `FactorStudy::full_report()` 改为 facade；
8. Core/Portfolio/Alphalens/Full profiles 建立 E2E test。

**DoD**：没有任何标准 report 需要手工重复编排研究阶段。

---

## R3 — Typed ArtifactStore + Provenance

**目标**：统一 cache/materialization/report 输入。

任务：

1. ArtifactKey V2；
2. typed ResearchArtifact；
3. ArtifactRef；
4. memory budget；
5. revision invalidation；
6. dependency fingerprint；
7. provenance V2；
8. executor cache hit/miss diagnostics。

**DoD**：所有 research node 输出都可被统一识别、缓存、引用、追踪。

---

## R4 — Full / Range / Incremental 统一

**目标**：增量执行不再是旁路。

任务：

1. DataDelta/DirtyRange；
2. incremental forward-return engine 接入 stage；
3. dirty dependency propagation；
4. IC/returns/turnover 可局部更新部分先落地；
5. 不安全 stage 自动 full fallback；
6. full == incremental equivalence gate；
7. append 性能基准。

**DoD**：批量与增量共享 ResearchPlan/Executor/ArtifactStore。

---

## R5 — Cross-section / Runtime 性能收敛

**目标**：减少研究层重复扫描和分配。

任务：

1. SegmentIndex/CrossSectionView；
2. date/group traversal reuse；
3. CSE；
4. scratch lifetime reuse；
5. compute-many/research stage sharing；
6. buffer/state/artifact 三层资源模型；
7. end-to-end memory/performance benchmark。

**DoD**：优化结果通过真实 panel benchmark 证明，而不仅是 microbench。

---

## R6 — Report V2 + Visualization artifact-driven

**目标**：报告、可视化与计算解耦。

任务：

1. ReportSummary；
2. ArtifactRef；
3. V1 compatibility serializer；
4. visualization view model；
5. renderer 仅消费 report/artifact；
6. HTML/GPU/export integration gate；
7. 大报告体积 benchmark。

**DoD**：Visualization 无研究算法所有权。

---

## R7 — API Contract + Multi-language SDK V2

**目标**：统一 control plane 与 data plane。

任务：

1. 逻辑拆分 api-contract / ffi-common；
2. versioned request/response/error；
3. typed artifact access；
4. Python NumPy low-copy；
5. Node TypedArray；
6. C ABI ownership；
7. Go/.NET/Java/mobile packaged smoke tests；
8. cross-language golden corpus。

**DoD**：所有 SDK 对相同输入产生同一语义结果和统一错误码。

---

## R8 — 高级研究服务 Profile 化

**目标**：把现有高级能力全部纳入统一研究计划，而不是继续旁挂。

范围：

- multifactor；
- neutralization；
- Fama-MacBeth；
- Purged/Embargo/CPCV/WalkForward；
- bootstrap/permutation/multiple testing；
- event；
- stability/health；
- mining/dedup；
- risk model；
- attribution；
- scenario。

**DoD**：所有公开高级研究入口都可以映射到 Profile/Custom Plan。

---

## R9 — 物理 crate 拆分评估与发布收敛

只有在 R1-R8 逻辑边界稳定后评估物理拆 crate。

评估依据：

- full build time；
- incremental build time；
- dependency fan-out；
- binary size；
- feature coupling；
- release independence；
- API stability。

可能拆分但不预设必须拆分：

```text
finkit-kernel
finkit-runtime
finkit-research
finkit-api-contract
```

如果数据证明拆分只增加版本/发布复杂度，则保持现有 crate 物理结构。

---

## 18. 每阶段强制检查方法

每一阶段至少做三轮检查：

### 第一轮：结构检查

- 模块依赖是否单向；
- 是否新增孤儿逻辑；
- 是否出现重复 owner；
- public API 是否绕开 executor。

### 第二轮：主体链检查

从真实入口追踪：

```text
SDK/CLI
 -> Request
 -> Context
 -> Plan
 -> Executor
 -> Kernel/Service
 -> Artifact
 -> Report
 -> Visualization/Response
```

逐节点确认输入、输出、error、revision、provenance。

### 第三轮：行为与门禁检查

- correctness；
- parity；
- incremental equivalence；
- performance；
- memory；
- package smoke test；
- same-SHA green。

任何“模块单测绿色但主链未接入”的功能，不视为完成。

---

## 19. 最终 Definition of Done

只有同时满足以下条件，才能认定 Finkit 的统一架构改造完成。

### Architecture

- 一个 canonical operation 只有一个算法 owner；
- ResearchPlan 是真实执行 SSOT；
- ResearchExecutor 是统一研究执行入口；
- Batch/Range/Streaming/Incremental 共用底层执行语义；
- ArtifactStore 统一研究中间结果；
- Visualization 只消费 artifact/report；
- SDK 不实现独立算法。

### Function

- 技术指标与 Formula 主链完整；
- Factor Core/Portfolio/Alphalens/Validation/Risk 等标准 profile 可执行；
- Advanced research services 不存在“已实现但无法从统一 plan 到达”的孤儿能力；
- 所有 public API 都能追踪到 canonical owner。

### Correctness

- TA-Lib parity gate 达标；
- Alphalens golden semantic parity 达标；
- full/incremental equivalence 达标；
- cross-language golden parity 达标；
- 无隐藏 panic 穿越 FFI。

### Performance

- 热路径不退化；
- compute-many CSE 有实际收益；
- append 不出现 O(n²) 全量复制；
- Python 数值路径无不必要 `Vec -> list -> ndarray`；
- 大型 research report 不被 JSON 复制放大。

### Release

- format / lint / docs / audit 全绿；
- core / runtime / research / visualization 全绿；
- Python / Node / Go / C/C++ / Java / .NET / WASM / iOS / Android package gates 全绿；
- required gates 全部在**同一最终 SHA** 上成功；
- 不降低阈值、不跳过测试、不依赖 CI 自动修改源码。

---

## 20. 禁止回归的反模式

后续 PR 一律禁止：

1. 为新语言 binding 再写一份指标/研究算法；
2. 为 Alphalens compat 建第二套计算链；
3. 在 visualization 重算 IC/收益/quantile；
4. 新增独立 cache 绕开 ArtifactStore；
5. 新增独立 incremental engine 不接 ResearchExecutor；
6. 继续扩大 `full_report()` 手工流程；
7. 用字符串 dispatch 进入热循环；
8. 为通过 CI 放宽真实性能/正确性门禁；
9. CI 修改并 push 生产源码；
10. 不做 E2E 接入就把“模块存在”标记为功能完成。

---

## 21. 下一轮实际实施顺序

严格按以下顺序推进，避免同时大改导致无法定位 regression：

```text
1. R0 当前真实红灯收敛
2. R1 canonical ownership / SSOT
3. R2 ResearchExecutor + Profile + Policy
4. R3 Typed ArtifactStore + Provenance
5. R4 Incremental/DirtyRange 接入统一 executor
6. R5 Runtime/Cross-section 性能收敛
7. R6 Report V2 + Visualization
8. R7 Multi-language SDK V2
9. R8 Advanced research 全面编排接入
10. R9 是否物理拆 crate 的数据化决策
```

优先级原则：

> **先修断链和双主链，再做缓存；先统一语义，再做性能；先保证 Rust canonical，再扩 SDK；先让所有功能可由统一 plan 到达，再新增研究能力。**

这将使 Finkit 从“功能覆盖很广的多模块库”收敛为真正统一的量化计算/研究基础设施。
