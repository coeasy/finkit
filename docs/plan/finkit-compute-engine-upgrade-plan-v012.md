# Finkit Compute Engine 全面升级优化方案（v0.1.2 基线）

- 日期：2026-09-02
- 基线：PR #13 clean head `18131aca5f782a86959855ff137a01afad524ab0`
- 分支：`feat/compute-runtime-factorplan-v012`
- 版本目标：继续稳定在 `0.1.2`，先完成架构收口、契约统一和性能基础设施，不提前扩版本号
- 产品定位：高性能、跨语言、跨终端兼容的金融指标、公式与因子计算基础库

## 1. 目标与边界

Finkit 当前已经具备 150+ Batch 指标、145 个 Streaming registry entries、公式引擎、Factor Engine、Runtime Contract、Function Registry、模式识别、Feature Engineering、多语言绑定、CLI/WASM 等能力。下一阶段不再以“继续堆指标数量”为主，而是把已有能力统一到可长期演进的计算架构。

本轮核心目标：

1. 建立统一 Compute IR，显式描述纯函数、副作用、lookback、stateful、streaming 等执行语义。
2. 将 `MarketFrame` / `SeriesView` 发展为跨 Indicator / Formula / Factor 的统一 Runtime 输入契约。
3. 建立 `FactorPlan`，把因子 DAG 校验、拓扑排序、raw input 依赖分析从每次执行前移到计划编译阶段。
4. 将 Function Registry 发展为真正 SSOT，为 CLI、文档、多语言 binding codegen、兼容矩阵提供统一元数据。
5. 继续减少 allocation、重复计算和多层数据复制，优先 Buffer reuse / DAG reuse / rolling primitive reuse，再推进 SIMD/JIT。
6. 建立 optimizer equivalence、Batch/Streaming equivalence、跨语言 golden fixture、memory/perf regression gate。

明确不做：

- 不把 Finkit 扩展成交易系统、数据平台或完整量化研究 OS。
- 不把回测、风险、可视化等可选能力反向耦合到最小核心安装。
- 不通过跳过测试、continue-on-error 或降低门禁来换取 CI 绿色。
- 不维护多套独立终端执行器；所有终端最终都收敛到 Canonical IR/Runtime。

## 2. 当前架构梳理

### 2.1 Kernel

当前 `core` 已覆盖：

- batch indicators
- math / SIMD kernels
- candlestick / chart patterns
- streaming indicators
- transforms
- risk
- selectors
- backtest
- sector / multi-period resonance
- feature engineering
- optional Polars / Rayon

底层 Kernel 应继续保持：

```text
slice / aligned series
        |
        v
parameter validation
        |
        v
rolling/scalar/SIMD kernel
        |
        v
caller-owned output buffer
```

核心原则：纯计算、低分配、无业务状态、可独立 benchmark。

### 2.2 Runtime

当前 `runtime.rs` 已存在：

- `SeriesView<'a>`
- `MarketFrame<'a>`
- `NanPolicy`
- `WarmupPolicy`
- 对齐校验
- allocation-free field alias lookup
- `Cow` zero-copy normalization fast path

目标是把它升级为所有高层引擎共用的输入边界：

```text
NumPy / Vec / FFI / WASM / Polars
              |
              v
      MarketFrame / SeriesView
              |
      +-------+--------+
      |       |        |
 Indicator  Formula  Factor
```

### 2.3 Formula Engine

当前执行链已包括：

```text
source -> parser -> AST -> optimizer
                     |
              +------+------+
              |             |
          AST executor   bytecode VM
                            |
                           JIT
              |             |
              +------v------+
                 buffer/SIMD
```

现有关键能力：

- compile cache
- bytecode cache
- persistent VM scratch
- `eval_into`
- `eval_range`
- `eval_last`
- borrowed/zero-copy simple formula input fast path
- sandbox
- template/debugger/drawing
- incremental/partial/lazy/parallel execution入口

近期 optimizer 回归说明 AST 不能同时承担“语法树”和“最终执行语义”。assignment/output/drawing 等节点具有可观察副作用，不能再依赖普通 AST DCE 的隐式判断。

### 2.4 Factor Engine

已有：

- `FactorContext`
- `FactorDefinition`
- `FactorRegistry`
- `FactorEngine`
- dependency recursion
- per-request memoization
- cycle detection
- `evaluate_many`
- direction-aware weighted composite

当前主要缺口：

- DAG 每次执行仍需重新递归确认。
- `FactorContext` 主要拥有 `Vec<f64>`，尚未与 `SeriesView`/`MarketFrame` 的 borrowed runtime 统一。
- 缺少显式 FactorPlan、topological execution order、raw input manifest、lookback aggregation 和 buffer lifetime 信息。

### 2.5 Function Registry

当前元数据已经包含：

- canonical name / alias
- category
- input shape
- params
- outputs
- lookback
- streaming
- deterministic

下一步要把 Registry 作为 SSOT，统一驱动：

```text
Function Registry
       |
       +--> Formula validation
       +--> CLI help
       +--> docs generation
       +--> compatibility matrix
       +--> Python/TS/Java/C#/Go/C schema
       +--> planner capabilities
```

## 3. 目标架构

```text
                   Input Adapters
         NumPy / Vec / FFI / WASM / Polars
                         |
                         v
                MarketFrame / SeriesView
                         |
                         v
                 +------------------+
                 |    Compute IR     |
                 |------------------|
                 | operation        |
                 | inputs/outputs   |
                 | lookback         |
                 | purity/effect    |
                 | deterministic    |
                 | stateful         |
                 | streaming        |
                 | nan/warmup       |
                 +---------+--------+
                           |
          +----------------+----------------+
          |                |                |
          v                v                v
      Batch Backend   Streaming Backend   SIMD Backend
          |                |                |
          +----------------+----------------+
                           |
                      Buffer Arena
                           |
            +--------------+--------------+
            |              |              |
          Formula        Factor       Direct Indicator
            |              |              |
            +--------------+--------------+
                           |
                    Multi-language SDK
```

## 4. Compute IR 设计

### 4.1 基础元数据

建议新增：

```rust
pub struct ComputeNodeId(pub usize);

pub enum ComputeEffect {
    Pure,
    WriteVariable(String),
    EmitOutput(String),
    Draw,
    Stateful,
}

pub struct ComputeCapabilities {
    pub deterministic: bool,
    pub streaming: bool,
    pub stateful: bool,
    pub lookback: LookbackRequirement,
    pub effect: ComputeEffect,
}

pub struct ComputeNode {
    pub id: ComputeNodeId,
    pub operation: String,
    pub dependencies: Vec<ComputeNodeId>,
    pub capabilities: ComputeCapabilities,
}
```

执行优化必须遵循 effect/purity，而不是只根据“最终表达式是否引用”判断可删除性。

### 4.2 DAG validation

`ComputePlan::compile` 负责：

1. NodeId 唯一性。
2. dependency 存在性。
3. cycle detection。
4. stable topological order。
5. 最大 lookback 聚合。
6. 是否支持 streaming 的整体能力推导。
7. effectful node retention。

### 4.3 Formula integration

后续将：

```text
Formula AST
   |
   v
Semantic analysis
   |
   v
Compute IR
   |
   +--> optimizer
   +--> bytecode
   +--> incremental planner
```

assignment：`WriteVariable(name)`

output：`EmitOutput(name)`

drawing：`Draw`

这样 DCE/CSE/LICM 都必须显式尊重 effect。

## 5. Unified Runtime 设计

### 5.1 Borrowed-first

继续保持：

```rust
SeriesView<'a> { name, values: &'a [f64] }
MarketFrame<'a> { open, high, low, close, volume, ... }
```

新增统一执行策略：

```rust
pub struct ExecutionPolicy {
    pub nan: NanPolicy,
    pub warmup: WarmupPolicy,
}
```

高层 Planner 不复制输入；只有确实需要 normalizing/mutable intermediate 时才进入 owned buffer。

### 5.2 Buffer Arena

优先实现可复用 scratch/result buffer：

- keyed by length/capacity
- caller-owned output 优先
- 生命周期按 DAG last-use 释放
- multi-output function 可一次申请连续 buffer group
- streaming state 与 batch scratch 分离

性能优化顺序固定为：

```text
allocation elimination
-> duplicate compute elimination
-> O(n*k) to O(n)
-> buffer reuse
-> kernel fusion
-> SIMD
-> parallel
-> JIT
```

## 6. FactorPlan 设计

### 6.1 Compile 阶段

```text
FactorRegistry + targets
        |
        v
validate identifiers
        |
        v
resolve dependencies
        |
        v
cycle detection
        |
        v
topological sort
        |
        v
raw input manifest
        |
        v
FactorPlan
```

`FactorPlan` 至少包含：

- targets
- stable execution order
- required raw inputs
- dependency graph
- future lookback metadata slot

### 6.2 Execute 阶段

第一阶段保持完全兼容现有 `FactorEngine`：

```text
plan.validate_context(context)
        |
        v
engine.evaluate_many(targets, context)
```

第二阶段再把 `FactorEngine` 内部改为直接按 plan 顺序执行，彻底消除重复 dependency traversal。

### 6.3 Borrowed Factor Context

后续增加：

```rust
BorrowedFactorContext<'a>
```

底层持有 `SeriesView<'a>` / `MarketFrame<'a>`，并提供与现有 owned `FactorContext` 的兼容适配层，最终让 Python NumPy / C FFI / Polars 输入不需要为了进入 Factor Engine 再复制一套 Vec。

## 7. Registry SSOT / Schema Driven SDK

增加机器可读 API schema：

```text
registry -> canonical schema
             |
             +--> docs JSON
             +--> CLI metadata
             +--> Python signature generator
             +--> TypeScript declarations
             +--> Java/C#/Go wrappers
             +--> C header metadata
```

手写 binding 只保留：

- native memory conversion
- error mapping
- loader/platform packaging
- language-specific ergonomics

禁止继续人工维护同一指标的 8 套参数默认值和文档。

## 8. Formula Compatibility Matrix

兼容性不再使用模糊“支持某终端”表述，改为生成能力矩阵：

| Capability | Finkit | TDX | THS | EastMoney | Pine |
|---|---|---|---|---|---|
| MA/EMA/REF | native | yes | subset | subset | adapter |
| assignment | native | yes | common | common | different |
| output modifiers | native | yes | partial | partial | different |
| drawing | native | common | partial | partial | partial |
| parameter declarations | native | yes | partial | partial | different |

每个单元格必须绑定 golden fixture。

目录建议：

```text
core/tests/formula_compat/
  tdx/
  ths/
  eastmoney/
  pine/
```

验收维度：parser、semantic、NaN、warmup、lookback、incremental、drawing/output。

## 9. 性能专项

### 9.1 第一优先级

- 核心 indicator 全量 `_into()`。
- multi-output caller-owned buffer。
- rolling sum/min/max/variance primitive 统一复用。
- Formula/Factor 共用 Buffer Arena。
- persistent plan / bytecode / scratch。
- multi-factor DAG CSE。

### 9.2 当前 watch-list

优先继续优化 README benchmark 中相对 TA-Lib 较弱的：

- AROON
- MFI
- WILLR
- WMA
- STOCHF
- AD/ADOSC

先 profile 算法复杂度和 allocation，再决定 SIMD；不做“为 SIMD 而 SIMD”。

### 9.3 JIT 定位

JIT 保持实验/高级 backend，不作为默认性能宣传主线。只有满足以下条件才扩展：

- compile amortization 明确
- 热公式重复执行次数足够
- 比 optimized bytecode 有稳定收益
- fallback 语义一致

## 10. 测试与 CI 门禁

新增/加强：

1. Optimizer equivalence：unoptimized == optimized execution。
2. AST == Bytecode == incremental == range/last。
3. Batch == Streaming final value。
4. Formula terminal golden fixtures。
5. FactorPlan order / cycle / missing input / cache behavior。
6. FFI panic isolation / pointer ownership / length contract。
7. property tests / parser fuzz / FFI fuzz。
8. Miri/Sanitizer 针对 unsafe 边界。
9. memory regression gate。
10. performance regression gate。

任何门禁失败都修真实实现，不通过 skip/continue-on-error 隐藏。

## 11. 多语言发布闭环

版本保持 `0.1.2`，在正式 release 前完成：

```text
main CI green
-> tag v0.1.2
-> Rust crate
-> CLI binaries
-> Python ABI3 wheels
-> Node packages
-> C artifacts
-> WASM package
-> Java/.NET/Go packaging verification
-> install smoke tests
-> SHA256/provenance
-> GitHub Release
```

当前 public release 仍停留在 v0.1.0，因此本轮架构优化不能替代 v0.1.2 正式发布闭环。

## 12. 分阶段实施顺序

### Phase A - 当前立即执行

- 新增 Compute IR 基础类型与 DAG compile/validate。
- 新增 FactorPlan，支持 stable topological order 和 raw input manifest。
- 建立 FunctionSpec -> ComputeCapabilities 映射。
- 增加相关 unit tests。
- 文档落盘并接入主文档索引。

### Phase B

- BorrowedFactorContext / MarketFrame adapter。
- FactorEngine plan-based execution。
- Buffer Arena。
- Formula semantic analysis -> Compute IR。

### Phase C

- Registry canonical schema。
- SDK codegen scaffold。
- compatibility matrix generator + golden fixtures。

### Phase D

- rolling primitive / kernel fusion。
- memory/perf gates。
- watch-list indicators 定向优化。

### Phase E

- merge/release 收口。
- v0.1.2 全工件 smoke test。

## 13. 完成标准

本轮全面升级不以“新增文件完成”为验收，而以以下条件为准：

- ComputePlan 能检测重复 NodeId、未知依赖和 cycle，并给出稳定拓扑顺序。
- effect/purity 成为优化元数据的一等公民。
- Function Registry 能提供 Compute capability 映射。
- FactorPlan 能在执行前完成 DAG 编译并明确 raw inputs。
- 新增 API 保持现有 FactorEngine / FormulaEngine 向后兼容。
- `cargo fmt --all --check` 通过。
- `cargo check --workspace --all-targets --locked` 通过。
- `cargo test -p finkit --locked` 通过。
- Clippy / Docs / Version consistency / Python wheels 不回退。

## 14. 后续原则

Finkit 后续所有新增功能应先回答三个问题：

1. 它属于 Kernel、Runtime、Compute Engine、Compatibility 还是 SDK 哪一层？
2. 它是否可以由 Registry/Compute IR 描述，而不是增加一条特例执行路径？
3. 它是否增加复制、重复计算、语义分叉或跨语言维护成本？

只有通过这三个约束，项目才能从“功能很多的 Rust 指标库”真正升级为稳定的金融计算基础设施。
