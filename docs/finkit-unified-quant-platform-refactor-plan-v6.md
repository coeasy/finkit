# Finkit 统一量化基础设施架构与重构方案 V6

> 状态：Target Architecture / Refactor Baseline
>
> 基线分支：`feat/factor-research-architecture` / PR #29
>
> 基线 SHA：`d3096e4e78dd798cb129bb51fcdfc902b9e07358`
>
> 日期：2026-09-11
>
> 本文档以“替代 TA-Lib + 公式系统 + 因子系统 + 因子引擎 + 因子分析 + 量化计算 + 可视化 + 多语言支持”为统一核心目标，作为后续重构与功能建设的最高层架构基线。

---

## 1. 产品定位

Finkit 不再定义为单纯技术指标库，也不只是 Alphalens 的 Rust 复刻，而是：

> **面向多语言、多运行模式和研究场景的统一量化计算与因子研究基础设施。**

最终必须形成以下八大核心能力，且全部属于产品核心，而不是附属模块：

1. **TA-Lib 替代层**：完整覆盖 TA-Lib 指标语义、精度、边界行为，并在关键热路径达到或超过其性能。
2. **公式系统 Formula**：支持指标/数学/逻辑/条件/时序/横截面/因子表达式，编译为统一执行计划。
3. **因子系统 Factor System**：统一 Factor 定义、注册、组合、依赖、参数、元数据、生命周期、版本和可复现性。
4. **因子引擎 Factor Engine**：支持单因子/多因子、批量/流式/增量、DAG/CSE、缓存、并行、状态复用、跨因子公共中间结果复用。
5. **因子分析 Factor Research**：完成 Alphalens 类分析能力，并扩展多因子、验证、风险、事件、稳定性、组合与容量分析。
6. **量化计算 Quant Compute**：统一收益、风险、统计、回归、窗口、横截面、组合、金融数学、回测评价等通用计算能力。
7. **可视化 Visualization**：围绕统一 Artifact/Report 构建研究图表和高性能数据可视化，不重复计算研究逻辑。
8. **多语言支持 Multi-language SDK**：Rust 作为算法与语义 SSOT，对 Python / Node.js / Go / C/C++ / Java / .NET / WASM / iOS / Android 等提供一致合同和高性能数据路径。

项目明确不扩展为行情采集器、券商交易客户端、OMS/EMS 或完整资管交易平台。Finkit 负责“计算、研究、分析、表达、可视化与 SDK”，外部系统负责数据源与交易执行。

---

## 2. 总体设计原则

### 2.1 一个语义，一个实现

所有指标、统计、收益、回归、排序、量化和研究算法必须只有一个 canonical implementation。

```text
Canonical Kernel
  ├─ TA-Lib API facade
  ├─ Formula operator
  ├─ Factor operator
  ├─ Research service
  ├─ Visualization artifact
  └─ Language SDK adapter
```

严禁不同层复制算法。

### 2.2 Rust 为语义 SSOT

所有真正算法、研究语义、错误合同、核心数据模型均在 Rust 维护。语言绑定只能适配内存、类型、ABI 和调用习惯。

### 2.3 Formula、Factor 和 Research 共用 Runtime

FormulaEngine、FactorEngine 和 ResearchEngine 不再拥有互相独立的执行器，而统一复用：

```text
Semantic IR
   ↓
Logical DAG
   ↓
Dependency / CSE / Lifetime Analysis
   ↓
ExecutionPlan
   ↓
Executor
   ↓
Kernel + BufferArena + StateArena + ArtifactStore
```

差异只体现在节点语义、数据模型和输出 artifact。

### 2.4 Batch / Range / Last / Streaming / Incremental 统一

所有算法尽量支持统一执行模式：

- `eval_all`
- `eval_range`
- `eval_last`
- `append`
- `stream`

它们必须共用同一算法语义和状态定义，而不是五套实现。

### 2.5 数据和执行分离

数据层负责：

- 时间序列；
- OHLCV；
- Panel；
- Asset / Group / Universe；
- 缺失值；
- Calendar；
- Revision。

执行层只消费标准化视图，不持有数据采集语义。

### 2.6 报告和可视化不参与计算

Visualization 只能消费稳定 Artifact / Report / ViewModel，不能在图层里再算 IC、均值收益、最大回撤、分层或其它核心指标。

### 2.7 多语言合同统一

同一功能必须共享：

- schema/version；
- error envelope；
- precision semantics；
- missing-value semantics；
- warmup/lookback；
- enum identity；
- release version；
- golden corpus。

---

## 3. 最优目标架构

```text
+====================================================================+
|                         User / Application                         |
+====================================================================+
       |                 |                  |                 |
       v                 v                  v                 v
   Rust API          Python/Node         Go/Java/.NET       WASM/Mobile
       \                 |                  |                 /
        +----------------+------------------+----------------+
                                 |
+--------------------------------------------------------------------+
| L8 API / SDK Contract                                              |
| Versioned Request / Response / Error / Metadata / ArtifactRef      |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L7 Visualization & Report                                         |
| ReportBuilder / ViewModel / ChartSpec / Renderer / Export          |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L6 Factor Research Engine                                         |
| ResearchProfile -> ResearchPlan -> ResearchExecutor -> Report      |
| IC / Return / Turnover / Portfolio / Validation / Risk / Event ... |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L5 Factor Engine                                                   |
| FactorDefinition / FactorGraph / FactorPlan / FactorExecutor       |
| Registry / Metadata / Composition / Dependency / Batch/Incremental |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L4 Formula System                                                  |
| Parser -> AST -> Typed IR -> Logical DAG -> ExecutionPlan          |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L3 Unified Runtime                                                 |
| Planner / Executor / CSE / Scheduler / DirtyRange / Provenance     |
| BufferArena / StateArena / ArtifactStore / Parallelism             |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L2 Quant Services                                                  |
| Returns / Risk / Statistics / Regression / CrossSection / Finance  |
| Portfolio / Evaluation / Rolling / Transform / Feature             |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L1 TA-Lib Compatible Indicator Layer                               |
| Trend / Momentum / Volatility / Volume / Cycle / Price / Pattern   |
+--------------------------------------------------------------------+
                                 |
+--------------------------------------------------------------------+
| L0 Canonical Numeric Kernel                                       |
| Window / EMA / WMA / TR / ATR / DM / Rank / Quantile / SIMD ...   |
+--------------------------------------------------------------------+
```

这 9 层必须保持单向依赖。高层不得被低层反向依赖。

---

## 4. TA-Lib 替代体系

TA-Lib 替代仍然是 Finkit 的第一核心基础能力。

### 4.1 功能目标

必须建立完整 TA-Lib function manifest：

```text
function
category
parameters
lookback
warmup semantics
input count
output count
output names
NaN behavior
unstable-period behavior
precision tolerance
streaming support
into-buffer support
benchmark status
```

所有函数按 manifest 自动生成：

- Rust facade；
- Python binding；
- Node binding；
- 文档；
- parity tests；
- API registry；
- benchmark matrix。

### 4.2 内核目标

禁止每个指标自己实现 rolling/window/EMA/extrema。

Canonical primitive 优先包括：

- SMA / EMA / WMA / RMA；
- rolling sum / mean / var / stddev；
- rolling min/max/extrema visitor；
- true range；
- directional movement；
- rank / fractional rank；
- regression；
- covariance/correlation；
- quantile；
- stateful smoothing；
- sin/cos/cycle primitives。

指标层只负责组合这些 kernel。

### 4.3 验收门禁

必须同时满足：

- TA-Lib parity `failure = 0`；
- 无明显 warmup 偏差；
- 随机/极值/NaN/短序列 parity；
- 热路径无非必要 heap allocation；
- 关键指标相对性能不回退；
- `*_into` 与返回 Vec 语义一致；
- streaming/batch 同语义。

---

## 5. Formula 系统

Formula 是统一量化表达语言，而不是简单字符串 API。

### 5.1 编译链

```text
Formula Source
  ↓
Parser
  ↓
AST
  ↓
Name / Type / Shape Resolution
  ↓
Typed Formula IR
  ↓
Constant Fold
  ↓
Common Subexpression Elimination
  ↓
Logical DAG
  ↓
ExecutionPlan
```

字符串 operation 名称必须只在编译阶段解析；运行热路径只操作数字 ID。

### 5.2 IR 类型

至少支持：

- Scalar；
- Series；
- OHLCV Series；
- Boolean Series；
- CrossSection；
- Factor；
- Group；
- Matrix/Table；
- Stateful node。

### 5.3 Runtime 共用

Formula 不再拥有独立 Buffer 生命周期，而统一使用：

```text
BufferArena
StateArena
ArtifactStore
```

依赖分析在编译期计算 buffer lifetime 并复用 scratch。

---

## 6. 因子系统

必须把“因子函数”提升为真正的 Factor Model。

### 6.1 FactorDefinition

```text
FactorDefinition {
  id
  name
  version
  expression / kernel
  parameters
  inputs
  output_shape
  lookback
  warmup
  frequency
  universe_requirements
  group_requirements
  deterministic
  streaming_capability
  incremental_capability
  metadata
}
```

### 6.2 因子来源统一

Factor 可以来自：

- Formula；
- 内建 Rust kernel；
- 指标组合；
- 横截面 transform；
- 自定义注册 operator；
- 多因子组合。

无论来源如何，都转换成统一 FactorGraph。

### 6.3 Factor Registry

Registry 是唯一发现入口，负责：

- ID；
- 名称；
- aliases；
- version；
- 参数 schema；
- documentation；
- capability；
- provenance；
- deprecation。

多语言 SDK、CLI 和 docs 都从 registry 生成，不重复维护清单。

---

## 7. 因子引擎

FactorEngine 是项目核心执行层之一。

### 7.1 主链

```text
FactorRequest
   ↓
Resolve FactorDefinitions
   ↓
Build FactorGraph
   ↓
Merge common dependencies
   ↓
Compile FactorPlan
   ↓
FactorExecutor
   ↓
Kernel / Runtime
   ↓
FactorArtifacts
```

### 7.2 必须支持 compute-many

多因子计算不允许逐因子重复执行：

```text
MA(close, 20)
RSI(close, 14)
Factor A = MA20 / close
Factor B = MA20 - EMA20
Factor C = rank(RSI14)
```

其中 `MA20`、`close`、共享 rolling/window、公共中间结果必须只算一次。

### 7.3 增量计算

引入统一：

```text
DataRevision
DataDelta
DirtyRange
NodeState
```

Executor 根据节点 capability 自动选择：

- cache hit；
- append update；
- range recompute；
- full recompute。

彻底避免 append_bar O(n²) 和完整数组重复复制。

---

## 8. 因子分析 / Research Engine

Research 必须从“函数集合”收敛为 Plan + Executor。

### 8.1 标准研究主链

```text
ResearchRequest
  ↓
ResearchContext
  ↓
ResearchPolicy
  ↓
ResearchProfile
  ↓
ResearchPlan
  ↓
ResearchExecutor
  ↓
ArtifactStore
  ↓
ResearchReport
```

### 8.2 基础分析

标准 profile 必须覆盖：

- forward returns；
- quantization；
- factor weights；
- factor returns；
- information coefficient；
- mean IC / ICIR / HAC statistics；
- quantile returns；
- alpha/beta；
- turnover；
- rank autocorrelation；
- cumulative performance；
- cost/slippage；
- data quality。

### 8.3 高级分析

高级 profile 覆盖：

- multi-factor correlation；
- VIF；
- PCA；
- neutralization；
- Fama-MacBeth；
- event study；
- stability / factor health；
- candidate dedup / screening；
- Purged K-Fold；
- embargo；
- CPCV；
- walk-forward；
- bootstrap / permutation；
- multiple testing；
- factor risk model；
- attribution；
- scenario analysis；
- portfolio capacity。

### 8.4 ResearchProfile

建议内建：

- `Core`
- `Alphalens`
- `Portfolio`
- `Validation`
- `Risk`
- `Event`
- `Full`
- `Custom`

Profile 只决定 plan，不创建新的算法体系。

### 8.5 Alphalens 目标

坚持结果语义兼容：

- forward returns；
- quantile semantics；
- IC；
- mean returns by quantile；
- turnover；
- alpha/beta；
- cumulative return；
- group neutral / demeaned semantics。

建立 Python Alphalens golden parity dataset，但不复制其低效对象结构和 plotting 实现。

---

## 9. 统一量化计算层

量化计算不能散落在 indicator、research、binding 中。

建议按语义域形成 canonical services：

```text
quant/
  returns
  statistics
  regression
  ranking
  distribution
  rolling
  cross_section
  normalization
  risk
  performance
  portfolio
  finance
  validation
```

所有上层优先复用这些 service。

应重点统一：

- simple/log return；
- cumulative return；
- volatility；
- Sharpe / Sortino；
- max drawdown；
- VaR / CVaR；
- beta/alpha；
- covariance/correlation；
- regression；
- ranks/quantiles；
- HAC/Newey-West；
- turnover；
- transaction cost；
- portfolio evaluation。

滚动指标和全样本评价必须命名区分，例如：

```text
rolling_sortino_ratio
sortino_ratio
rolling_max_drawdown
max_drawdown
```

避免 SSOT ownership 再次冲突。

---

## 10. Runtime 最优化设计

Runtime 是整个项目能否真正统一的关键。

### 10.1 ExecutionPlan

ExecutionPlan 至少包含：

```text
NodeId
KernelId
Dependencies
InputBufferIds
OutputBufferIds
StateSlots
Lookback
DirtyRangePolicy
ParallelPolicy
MaterializationPolicy
Effect
```

### 10.2 BufferArena

负责短生命周期数值 buffer：

- 预分配；
- scratch reuse；
- liveness reuse；
- typed slice；
- alignment；
- SIMD-friendly layout。

### 10.3 StateArena

负责跨次执行状态：

- EMA state；
- rolling state；
- streaming indicators；
- incremental factor state；
- maturity state。

不能再和普通 scratch 混用。

### 10.4 ArtifactStore

负责可复用、可缓存、可报告的语义结果。

ArtifactKey 至少包含：

```text
data revision
node id
operation id/version
parameter fingerprint
semantics fingerprint
calendar fingerprint
universe fingerprint
dependency fingerprints
range/maturity
```

Artifact 类型包括：

- NumericSeries；
- BooleanSeries；
- PanelSeries；
- Matrix；
- Table；
- ScalarMap；
- QuantileResult；
- ICResult；
- PortfolioResult；
- ValidationResult；
- ReportSection。

---

## 11. 可视化核心架构

可视化作为核心能力，但必须和计算解耦。

统一链：

```text
Artifact / Report
   ↓
VisualizationModel
   ↓
ChartSpec
   ↓
Renderer
   ├─ HTML/SVG
   ├─ Canvas/WebGPU
   └─ export
```

重点图表覆盖：

- K-line / OHLC；
- indicator overlay；
- multi-panel indicator；
- factor quantile returns；
- IC time series/distribution；
- cumulative returns；
- turnover；
- factor heatmap；
- correlation matrix；
- event study；
- drawdown；
- risk contribution；
- scenario comparison。

Visualization tests 必须消费 golden artifact，禁止在测试 fixture 中绕开正式 Report/Artifact contract。

---

## 12. 多语言核心架构

多语言支持不是把函数逐个手写一遍，而是“统一描述 + 自动生成 + 少量语言优化”。

### 12.1 两条数据通路

**Control Plane**：

- JSON/serde；
- config；
- metadata；
- schema；
- errors；
- small reports。

**Data Plane**：

- contiguous arrays；
- typed buffers；
- ndarray/NumPy；
- TypedArray；
- pointer/length；
- owned/borrowed handle。

大数据禁止默认 JSON round-trip。

### 12.2 Python

优先级最高：

- NumPy input zero-copy/borrow；
- output 直接 ndarray；
- 消除 `Vec -> list -> ndarray`；
- GIL release；
- batch/compute-many；
- research API 与 Rust golden parity。

### 12.3 Node/WASM

- TypedArray；
- ArrayBuffer；
- memory ownership 明确；
- 浏览器和 Node API 语义一致。

### 12.4 Go/C/Java/.NET/Mobile

统一：

- ABI types；
- allocator ownership；
- enum mapping；
- panic boundary；
- ErrorEnvelope；
- version contract；
- packaged smoke test。

---

## 13. API 与版本策略

### 13.1 Public API 分层

建议明确：

```text
Stable API
Experimental API
Internal API
```

不得把 internal kernel 直接暴露后长期无法重构。

### 13.2 Compatibility

公共 breaking change：

1. 新 API 先上线；
2. 旧 API 变 compatibility facade；
3. 保留至少一个发布周期；
4. 标记 deprecated；
5. 最终移除。

Compatibility facade 绝不能保留旧算法实现。

### 13.3 Version SSOT

workspace version 作为唯一版本源，生成：

- Cargo crates；
- Python；
- npm；
- Go metadata；
- .NET；
- Java；
- Android/iOS；
- docs；
- release assets。

---

## 14. 当前最需要纠正的结构问题

### P0

1. **TA-Lib/quant canonical ownership 仍需完全锁定**。
2. **Formula/Factor/Research 尚未全部共用一个真实 Executor**。
3. **ResearchPlan 还不是 `full_report()` 的唯一执行来源**。
4. **Incremental 仍有专用旁路状态机，应纳入统一 DirtyRange Runtime**。
5. **ArtifactStore 尚未成为统一 typed cache/output 层**。
6. **当前 PR #29 仍有真实 CI 红灯，不能宣称发布完成**。

### P1

1. FactorDefinition / FactorGraph / FactorRegistry 需要形成更明确的一等模型。
2. compute-many 公共依赖复用必须从 Formula 扩展到 Factor Engine。
3. 横截面 date/group traversal 应统一 kernel，减少重复排序/分组/分配。
4. Report V2 应避免大数组全部 JSON 内嵌。
5. Visualization 应彻底 artifact-driven。
6. 各语言 binding 需要生成式 SSOT 和 golden parity。

### P2

1. 根据真实 compile graph 再决定是否物理拆 core crate。
2. 引入更细粒度 SIMD/parallel scheduler。
3. 更高级的 factor catalog、plugin/custom operator 能力。
4. 持久化 Artifact cache。
5. GPU compute 仅在 benchmark 证明有效后引入，避免架构先行过度设计。

---

## 15. 推荐重构实施路线

## R0：恢复当前真实门禁

先修当前 branch 的真实问题：

- Format；
- core tests；
- Research SSOT；
- Visualization integration；
- Go release test；
- 最终 same-SHA 全绿。

禁止降低阈值、跳过测试、改成 allow-failure。

## R1：Canonical Quant Kernel / TA-Lib SSOT

- 建立 operation ownership registry；
- 清除重复 rank/regression/risk/window 实现；
- 统一 rolling/stat/extrema/smoothing kernels；
- 所有 TA-Lib facade 改 delegate；
- parity = 0；
- benchmark gate 固化。

## R2：统一 Formula Runtime

- Typed IR；
- KernelId；
- CSE；
- liveness；
- BufferArena/StateArena；
- eval_all/range/last/streaming 共用 ExecutionPlan。

## R3：Factor System 一等化

- FactorDefinition；
- FactorRegistry；
- FactorGraph；
- FactorPlan；
- Factor metadata/version/provenance；
- Formula Factor 与 native Factor 统一。

## R4：Factor Engine

- compute-many；
- dependency merge；
- cross-factor CSE；
- incremental；
- DirtyRange；
- cache reuse；
- batch/streaming 同语义。

## R5：ResearchExecutor

- ResearchProfile；
- ResearchPolicy；
- ResearchPlan；
- ResearchExecutor；
- `FactorStudy::full_report()` 退化为 facade；
- advanced services 全部通过 plan 组合。

## R6：Typed ArtifactStore

- typed artifact；
- revision key；
- dependency fingerprint；
- partial materialization；
- cache；
- incremental invalidation；
- provenance。

## R7：Factor Analysis 完整化

- Alphalens parity corpus；
- IC/turnover/return/alpha-beta golden；
- multifactor；
- validation；
- event；
- risk；
- scenario；
- capacity；
- stability/mining。

## R8：Visualization V2

- Report/Artifact -> ViewModel；
- ChartSpec；
- HTML/SVG/WebGPU renderer；
- large-series downsampling/display-only optimization；
- 不重复研究计算。

## R9：Multi-language SDK V2

- schema generator；
- registry generator；
- ErrorEnvelope；
- control/data plane 分离；
- Python zero-copy；
- Node TypedArray；
- 全 binding golden parity；
- package smoke tests。

## R10：最终架构收敛

依据性能、编译时间和依赖图决定：

- 是否拆 `finkit-kernel`；
- 是否拆 `finkit-runtime`；
- 是否拆 `finkit-factor`；
- 是否拆 `finkit-research`；
- 是否保留当前 crate 物理布局。

原则是：**先统一逻辑边界，后决定物理 crate 边界。**

---

## 16. 主体链路最终 Definition of Done

### TA-Lib 链

```text
TA-Lib operation
 -> canonical kernel
 -> Rust facade
 -> generated language facade
 -> parity test
 -> benchmark gate
```

全部贯通。

### Formula 链

```text
Formula
 -> AST
 -> Typed IR
 -> DAG
 -> ExecutionPlan
 -> Executor
 -> Artifact
```

全部贯通。

### Factor 链

```text
FactorDefinition
 -> FactorGraph
 -> FactorPlan
 -> FactorExecutor
 -> ArtifactStore
```

全部贯通。

### Research 链

```text
ResearchRequest
 -> Context/Policy/Profile
 -> ResearchPlan
 -> ResearchExecutor
 -> ArtifactStore
 -> Report
```

全部贯通。

### Visualization 链

```text
Artifact/Report
 -> ViewModel
 -> ChartSpec
 -> Renderer
```

全部贯通。

### SDK 链

```text
Registry/Schema
 -> generated binding surface
 -> language adapter
 -> native Runtime/Research
 -> versioned response
```

全部贯通。

只有以上所有链路同时成立，才能定义为“核心架构完全实现”。

---

## 17. 最终发布门禁

发布必须在**同一个最终 SHA**上同时满足：

1. `cargo fmt --check`；
2. workspace compile；
3. Clippy `-D warnings`；
4. core tests；
5. Formula/runtime tests；
6. Factor system/engine tests；
7. Factor research tests；
8. TA-Lib parity failure = 0；
9. hot-path performance regression gate；
10. memory/allocation gate；
11. visualization integration；
12. Python wheels；
13. Node package；
14. Go package；
15. C/C++；
16. Java；
17. .NET；
18. WASM；
19. iOS；
20. Android；
21. docs；
22. package smoke tests；
23. version contract；
24. SSOT ownership gate；
25. golden cross-language parity。

禁止：

- 不同 SHA 拼绿色结果；
- 跳过失败平台；
- 降低性能阈值掩盖回归；
- duplicate algorithm facade；
- CI 自动修改源码后把修改当测试结果；
- mock 替代核心功能验证。

---

## 18. 重构优先级结论

后续不再以“新增多少函数”为主要进度指标，而按以下优先级推进：

```text
P0  正确性 / parity / CI green
 ↓
P1  Canonical Kernel SSOT
 ↓
P2  Unified Runtime
 ↓
P3  Formula System
 ↓
P4  Factor System + Factor Engine
 ↓
P5  Factor Research Engine
 ↓
P6  Artifact / Incremental / Cache
 ↓
P7  Visualization
 ↓
P8  Multi-language SDK
 ↓
P9  极致性能和物理 crate 收敛
```

尤其禁止继续在 Research、Formula、Binding 或 Visualization 中新增独立算法实现。

---

## 19. 最终目标状态

完成 V6 重构后，Finkit 应具有以下特征：

- TA-Lib 可以作为兼容标准，而不是内部架构依赖；
- Formula 是统一表达方式之一；
- Factor 是一等计算对象；
- FactorEngine 能自动复用跨因子公共 DAG；
- ResearchEngine 通过 ResearchPlan 真正执行；
- batch / range / last / streaming / incremental 共用同一语义；
- 所有缓存和报告建立在 typed ArtifactStore 上；
- Visualization 只消费 artifact；
- 所有语言共享同一 Rust 语义；
- Python 等高吞吐场景有低复制/零复制通道；
- 所有核心链路有 golden/invariant/performance gate；
- 任何新能力必须有明确 canonical owner；
- 发布只接受同一 SHA 的真实全绿结果。

最终形成：

> **一个量化计算内核、一套统一 Runtime、一套 Formula/Factor 表达体系、一个 Factor Engine、一个 Factor Research Engine、一套 Visualization Artifact 模型和一套跨语言稳定合同。**

这应作为 Finkit 后续版本演进、架构评审、PR 拆分和发布验收的统一基线。