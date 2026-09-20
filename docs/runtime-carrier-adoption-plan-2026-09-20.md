# crates/\* 接入生产：功能缺口分析与迁移规划

日期：2026-09-20
状态：**R1–R4 全部完成；平行轨道（5 个 crate）已删除 —— 收敛完成（见 §6）**
上游决策：用户 2026-09-20 选择「接入生产，作为 Runtime 载体」
相关文档：[improvement-plan-2026-09-20.md](improvement-plan-2026-09-20.md) §P2-1、[architecture-gap-assessment-2026-09-20.md](architecture-gap-assessment-2026-09-20.md):118

---

## 0. 结论摘要

先给结论，再给证据。

1. **平行轨道（`crates/finkit-{array,series,math,factor,runtime}`）声称有 4 项能力是 core 没有的**。
   其中 3 项确为缺口并已接入生产，第 ③ 项实测**已被 core 覆盖**（详见 §6.3）：
   - ① `Factor` 对象边界 + `FactorProvider` 工厂 + **带类型的参数校验** → 已落地（R2，`core/src/factor_provider.rs`）；
   - ② 手写**声明式因子图** API（不写公式字符串也能构图）→ 已落地（R3，`core/src/factor_graph.rs`）；
   - ③ ~~`QuantSeries` 的**符号 + 时间戳 + 严格递增校验**~~ → **core 早已有更强实现**：`data_contract::TemporalSeries` 在构造期校验名称/长度/单调时间戳并提供 `align_to` 对齐策略，`FrameKey` 校验 symbol 与 timeframe —— 无需接入；
   - ④ 以 `(symbol, factor, params, time_range)` 为键的**结果缓存形状** → 已落地（R4，`operation::OperationResultCache`）。

2. **但它的 `Executor` / `Scheduler` / `FactorRegistry` / kernel / 指标实现 5 项已被 core 超越，不能当 Runtime 载体**——`core/src/unified_executor.rs` 的 `UnifiedExecutor` + `KernelDispatcher` + `BufferArena` 在每一个维度上都更强，且已通过差分验证。**把载体换成平行轨道会让已绿的路径回退。**

3. **因此「接入生产」的正确落法**是：载体仍是 `UnifiedExecutor`（已验证），平行轨道**降级为它的声明式前端**——即把上面 4 项真缺口实现为「构图 / 构造 / 缓存」三层，向下 lowering 到现有的 `HotExecutionPlan`，而不是替换它。

> 换句话说：**crate 接入生产 = 是；crate 当执行引擎 = 否。** 下面第 1 节给出逐项证据，第 2 节给出 4 个阶段的可验证规划。

---

## 1. 具体有哪些功能未接入

### 1.1 平行轨道的完整能力清单

逐文件清点（共 1392 LOC / 9 个测试）：

| 能力 | 位置 | 说明 |
|---|---|---|
| `FloatArray` | `crates/finkit-array/src/lib.rs:6` | `Vec<f64>` 的薄包装，26 LOC |
| `QuantSeries` | `crates/finkit-series/src/lib.rs:6` | `symbol + timestamps + values` + `is_valid()`（等长 + 严格递增） |
| `ema` / `rolling_mean` / `rolling_variance` / `rolling_std` | `crates/finkit-math/src/` | 4 个数学 kernel |
| `trait Factor` | `crates/finkit-factor/src/lib.rs:21` | `name()` + `compute(&QuantSeries) -> FactorResult` |
| `FactorResult`（结构体） | `crates/finkit-factor/src/result.rs:7` | `factor_name` + 主 `series` + `outputs: BTreeMap<String, QuantSeries>` |
| `Sma` / `Ema` / `Rsi` / `Macd` | `crates/finkit-factor/src/{sma,ema,rsi,macd}.rs` | 4 个指标实现 |
| `FactorProvider` / `FactorFactory` | `crates/finkit-runtime/src/factory.rs:97` | `name()` + `create(params) -> DynFactor` |
| `FactorFactoryError` | `crates/finkit-runtime/src/factory.rs:13` | **InvalidParameter / UnknownParameter / DuplicateParameter** |
| `FactorFactoryRequest` | `crates/finkit-runtime/src/factory.rs:49` | 有序参数 + `canonical_params()`（排序后稳定串） |
| `FactorRegistry` | `crates/finkit-runtime/src/registry.rs:10` | 名字 → `Arc<dyn FactorProvider>` |
| `Scheduler::topological_order` | `crates/finkit-runtime/src/scheduler.rs:31` | 拓扑排序 + Duplicate/Missing/Cycle 错误 |
| `FactorGraph` / `FactorNode` / `ExecutionPlan` | `crates/finkit-runtime/src/graph.rs:35`、`lib.rs:29` | 手写声明式图 |
| `FactorCache` / `FactorCacheKey` / `CacheStats` | `crates/finkit-runtime/src/cache.rs:13`、`cache_key.rs:4` | `get_or_compute` + 命中/未命中统计 |
| `Executor` | `crates/finkit-runtime/src/executor.rs:55` | plan + registry + cache，按拓扑序逐节点执行 |
| `FactorConfig` / `FactorOutput` | `crates/finkit-runtime/src/{config,output}.rs` | 配置 + 输出别名 |

### 1.2 判定：真缺口 vs core 已超越

#### A. core 已超越 —— **不能作为载体**（5 项，逐项给出反证）

| 平行轨道能力 | core 对应实现 | 判定依据 |
|---|---|---|
| `Executor`（逐节点、每节点 `Vec<f64>`、输出 `clone()`） | `UnifiedExecutor<D>`（`core/src/unified_executor.rs:191`）+ `KernelDispatcher`（:78）+ `BufferArena` | core 有缓冲区复用、kernel 分派、`execute_range` / `execute_last`、`reset` / `rebind`、`buffer_stats`；且已差分验证 |
| 执行语义 | `ArtifactHash`（`core/src/unified_runtime.rs:24`）、`DirtyRange`（:69）、`RuntimeExecutionMode`（:180）、`RuntimeExecutionTrace`（:196） | core 有**增量/脏区**执行与可观测轨迹，平行轨道完全没有 |
| `Scheduler::topological_order`（含 Cycle 检测） | `ComputePlanError::DependencyCycle(Vec<ComputeNodeId>)`（`core/src/compute.rs:158`） | core 的 Kahn 环检测（`compute.rs:262`）会**回传具体环路径**（`:523` DFS 抽取），比平行轨道只报 `Cycle` 更强 |
| `FactorResult.outputs`（多输出） | `OutputLayout`（`core/src/execution_plan.rs:233`）+ `FormulaHotPlan::outputs()` / `FormulaOutputBinding`（`core/src/formula/hot_plan.rs`，2026-09-20 新增） | 多输出**已经具备**，且带 `level_marker` 语义（Pine `hline`） |
| `FactorCache` / `FactorCacheKey` | `OperationCacheKey`（`core/src/operation.rs:531`）、`CompositeCache`（`composite.rs`）、`FormulaCache`（`formula/compiler.rs:29`） | core 的键含 `dialect` + `FrameKey` + **`data_revision: u64`**（O(1)，不哈希缓冲区）；平行轨道的 `time_range: String` 更脆弱 |
| `ema` / `rolling_*`（4 个 kernel） | `core/src/formula/functions.rs`（7431 行）+ `core/src/indicators/`（37 个模块）+ `registry.rs` 111 条 `FunctionSpec` | 平行轨道是这 354 个批量函数的子集 |
| `Sma` / `Ema` / `Rsi` / `Macd` | 同上，且过 TA-Lib 201/201 parity | 平行轨道的 4 个实现是 core 的真子集 |
| `QuantSeries` 的输入模型 | `MarketFrame<'a>`（`core/src/runtime.rs:139`）+ `NanPolicy`（:12）+ `WarmupPolicy`（:24） | core 的输入模型是**多字段 OHLCV + NaN/预热策略**，比单序列更强 |

#### B. core 确实缺失 —— **值得接入**（4 项）

| # | 能力 | 平行轨道位置 | core 现状（反证） | 价值 |
|---|---|---|---|---|
| ① | `Factor` 对象边界 + `FactorProvider` 工厂 + **带类型的参数校验** | `factory.rs:13`、`:97` | `FactorDefinition`（`core/src/factors.rs:307`）只有 `name / dependencies / kind / direction / compute: FactorFn`，**没有参数概念，也没有参数校验** | 外部调用方（Python/Node）可以用声明式请求构造因子，拿到 `InvalidParameter / UnknownParameter / DuplicateParameter` 三种结构化错误，而不是笼统的失败 |
| ② | 手写**声明式因子图** | `graph.rs:35`、`lib.rs:29` | core 全库无 `add_node` / `depends_on` 式构图 API（grep 0 命中）；计划只能从公式源串生成 | 不写公式字符串即可组合因子（`SMA(3) → EMA(2)`），这是 pandas-ta 风格组合 API（P2-3）的前置 |
| ③ | `QuantSeries`：符号 + 时间戳 + **严格递增校验** | `series/src/lib.rs:6`、`:42` | `MarketFrame` 是 OHLCV 多字段；无「单序列 + symbol」类型 | 缓存键与错误信息需要 symbol；`is_valid()` 是廉价的前置校验 |
| ④ | `(symbol, factor, params, time_range)` 形状的结果缓存 | `cache_key.rs:4`、`cache.rs:41` | 三套缓存都不是这个键形状 | 需要**重新基底**到 `OperationCacheKey`（见 §2 阶段 R4），而非照搬 |

#### C. 一项语义冲突（必须在规划里解决）

平行轨道的 `FactorResult.series` 采用**预热裁剪**语义：`SMA(3)` 输入 10 点 → 输出 8 点，时间戳 `[2..9]`（`crates/finkit-factor/src/lib.rs:43` 的测试断言了这一点）。

core 的 `apply_warmup`（`core/src/runtime.rs:273`）+ `WarmupPolicy` 采用**保持长度 + 填充**语义。

两者**不兼容**：直接接入会让「同一公式在两条路径上长度不同」。这是本次接入最大的语义风险，必须在阶段 R2 明确二选一（建议：以 core 的 `WarmupPolicy` 为准，`QuantSeries` 只作为**输入端**类型，不作为输出端类型）。

---

## 2. 需要如何规划

### 2.1 总原则

- **载体不变**：执行仍是 `UnifiedExecutor` + `FormulaKernelDispatcher` + `HotExecutionPlan`。平行轨道不接管执行。
- **只搬真缺口**：§1.2-B 的 4 项；§1.2-A 的 5 项**不搬**，只把平行轨道对应文件降级/删除。
- **每阶段独立可验证**，且**不得**让 `cargo test -p finkit`（基线 3976 passed / 0 failed）与差分门禁回退。
- **命名冲突先解决**：core 的 `FactorResult<T> = Result<T, FactorError>`（`core/src/factors.rs:17`）与平行轨道的 `FactorResult` 结构体同名异义。搬入前必须先改名（建议 `FactorOutputSet`），否则会在同一个 crate 内撞名。

### 2.2 阶段划分

#### 阶段 R1 —— 定边界与去重（无功能变更，纯收敛）

- 在 `docs/` 记录本决策：载体 = `UnifiedExecutor`，平行轨道 = 声明式前端。
- 把 §1.2-A 的 5 项在平行轨道中的实现**标注为 superseded**，并给出 core 对应实现的位置（便于后续删除）。
- **验收**：文档落地；`cargo check --workspace` 仍 0；无代码行为变化。

#### 阶段 R2 —— 搬「构造层」：`FactorProvider` + 参数校验（真缺口 ①）

- 在 core 新增 `FactorProvider` / `FactorFactoryRequest` / `FactorFactoryError`，`canonical_params()` 原样保留（它是缓存键的稳定性基础）。
- 与既有 `FactorDefinition` 的关系：`FactorDefinition` 是**已注册的因子**；`FactorProvider` 是**按参数构造因子实例**的工厂。二者是组合关系，不是替代关系。
- **必须解决** §1.2-C 的预热语义冲突：`QuantSeries` 只做**输入端**；输出端统一走 core 的 `WarmupPolicy`。
- **验收**：新增用例覆盖 `InvalidParameter` / `UnknownParameter` / `DuplicateParameter` 三条错误路径；`canonical_params()` 对参数顺序不敏感（`a=1,b=2` ≡ `b=2,a=1`）。

#### 阶段 R3 —— 搬「构图层」：声明式因子图（真缺口 ②）

- 新增 `FactorGraph` / `FactorNode`，但**向下 lowering 到现有 compute IR**：复用 `ComputeNodeId`、`cse_plan`、`prune_unreachable`、`HotExecutionPlan::compile_with_parameters`。
- **不要**搬平行轨道的 `Scheduler`：环检测直接用 `ComputePlanError::DependencyCycle`（`core/src/compute.rs:158`），它更强。
- **不要**搬「每节点最多 1 个依赖」的限制（`executor.rs:112` 的 `MultipleDependencies`）——core 的 kernel 是 N 元输入的，这是能力回退。
- **验收**：`SMA(3) → EMA(2)` 这类图，经图 API 与经等价公式字符串两条路径，**逐元素一致**（沿用 `core/tests/formula_plan_differential.rs` 的差分口径）。

#### 阶段 R4 —— 搬「缓存层」：结果缓存（真缺口 ④）

- 缓存键**重新基底**到 `OperationCacheKey` 的形状（`dialect` + `FrameKey` + `data_revision`），**不要**照搬 `time_range: String`。
- 保留 `CacheStats`（hits/misses）与 `get_or_compute` 这两个有用的抽象。
- 与 `OperationCache` / `CompositeCache` 的关系需先裁决：**优先扩展现有缓存**，只有在键形状确实无法表达时才新增第四套（避免制造新的双轨）。
- **验收**：命中/未命中统计正确；`data_revision` 变化必须导致 miss（这是正确性而非性能要求）。

### 2.3 平行轨道本身的处置（与 R1–R4 并行）

| crate | 处置 | 理由 |
|---|---|---|
| `finkit-array` | **删除** | `FloatArray` 被 core 的 `BufferArena` / `ndarray` 完全覆盖，26 LOC 无独立价值 |
| `finkit-math` | **删除** | 4 个 kernel 是 `formula/functions.rs` 的真子集 |
| `finkit-series` | **合并后删除** | `QuantSeries` 的 symbol + 校验迁入 core（阶段 R2），其余丢弃 |
| `finkit-factor` | **合并后删除** | `trait Factor` 与 4 个指标实现迁入/对齐 core；`FactorResult` 改名后并入 |
| `finkit-runtime` | **部分合并后删除** | 只搬 ①④ 两项（+ ②的 `FactorGraph` 骨架）；`Executor` / `Scheduler` / `Registry` / `Factories` 不搬 |

> 全部删除都在 git 历史中可恢复；每个 crate 的删除与对应阶段的合并**同一个提交**完成，保证可回滚。

### 2.4 建议顺序与理由

R1 → R2 → R3 → R4。理由：

- R2 的 `canonical_params()` 是 R4 缓存键的输入，必须先有；
- R3 的构图 API 需要 R2 的 provider 边界来解析节点；
- R4 依赖前三者的键与实例；
- 每一阶段结束都应保持 `cargo test -p finkit` 全绿，随时可停。

---

## 3. 风险与回滚

| 风险 | 影响 | 缓解 |
|---|---|---|
| **预热语义冲突**（§1.2-C） | 同一公式两条路径长度不同，差分门禁会红 | R2 明确 `QuantSeries` 仅作输入端；输出端统一 `WarmupPolicy` |
| **`FactorResult` 撞名** | 同 crate 内同名异义，编译期混乱 | R2 搬入前先改名 `FactorOutputSet` |
| **制造第四套缓存** | 新的双轨，与本轮收敛目标相反 | R4 优先扩展 `OperationCache`；新增需单独论证 |
| **误把平行轨道当载体** | 回退已差分验证的执行路径 | R1 先把边界写进文档；载体不变是硬约束 |
| 平行轨道测试依赖裁剪语义 | 删除 crate 时连带删除 9 个测试 | 迁入的用例按 core 语义重写，不照抄断言 |

回滚：每个阶段独立提交；删除类操作集中在 R2–R4 的对应提交内，`git revert` 单个提交即可恢复。

---

## 4. 验收（整体）

1. `cargo test -p finkit` 不低于基线 3976 passed / 0 failed；
2. `cargo check --workspace` 退出码 0；
3. `core/tests/formula_plan_differential.rs` 全绿（构图路径与公式路径逐元素一致）；
4. 参数校验三条错误路径有用例；
5. `workspace.members` 中 5 个平行 crate 全部移除，且无任何残留引用；
6. `docs/` 只保留本文件 + `improvement-plan-2026-09-20.md` 作为权威基线。

---

## 5. 执行记录

### R1 已完成（无功能变更）

- 决策落文档：本文件 + `improvement-plan-2026-09-20.md` §P2-1。
- 5 个 crate 的 `lib.rs` 顶部标注各自「adopted / superseded」判定与 core 对应实现位置；
  `finkit-runtime` 的标注含逐模块对照表（`factory`/`graph`/`cache` 采用，`executor`/`scheduler`/`registry`/`factories` 被超越）。
- 验收：`RUSTFLAGS="-D warnings" cargo check -p finkit-array -p finkit-series -p finkit-math -p finkit-factor -p finkit-runtime --all-targets` → **exit 0，无警告**。

**R1 附带修复（既有缺陷，与本次改动无关）**：CI 的 `workspace-check` job
（`cargo check --workspace --all-targets` + `RUSTFLAGS="-D warnings"`）**本来就是红的**。
`core/tests/common/golden_loader.rs` 被 3 个测试目标 `mod common;` 引入，
但 `DEFAULT_TOLERANCE` 只被 `golden_example.rs` 使用，于是另外两个目标触发
`-D dead-code` → `error: could not compile finkit (test "talib_coverage_matrix")`。
该文件自身的约定是给这类共享测试支撑项加 `#[allow(dead_code)]` 并写明原因
（见同文件第 49、88 行），`DEFAULT_TOLERANCE` 只是漏了。已按同一约定补上；
三个目标现在都能在 `-D warnings` 下编译通过。

### R2 已实现（真缺口 ①）

新增 `core/src/factor_provider.rs`（在 `lib.rs` 中以 `pub mod factor_provider` 挂载，`#[cfg(feature = "std")]`）：

| 项目 | 说明 |
|---|---|
| `FactorFactoryError` | 三种结构化错误：`InvalidParameter` / `UnknownParameter` / `DuplicateParameter` |
| `FactorFactoryRequest` | 有序参数表（保持 `Vec` 而非 map，**重复参数才能被检出**）+ `try_params_map()` + `canonical_params()` |
| `FactorProvider` | `name()` + `create(&params) -> Result<FactorDefinition, FactorFactoryError>` |
| `FactorProviderRegistry` | 名字（大小写不敏感）→ `Arc<dyn FactorProvider>`，`create(&request)` |
| `positive_usize` / `reject_unknown` | 供 provider 复用的两个校验助手 |

**两处刻意的偏离**（对应 §1.2-A 的既有结论）：

1. **不搬 `FactorResult` 多输出结构体**。多输出已由 `execution_plan::OutputLayout` +
   `formula::FormulaHotPlan::outputs()` 表达；搬过来还会与 `factors::FactorResult<T>`（`Result` 别名）撞名。
   这样 §2.1 提到的「改名 `FactorOutputSet`」就不需要了——**通过不引入重复类型来消除冲突**。
2. **不搬预热裁剪语义**。预热统一由 `runtime::WarmupPolicy` 负责（保持长度）；再引入一套长度不同的约定，
   会让同一公式在不同路径上产生不同长度。

**与既有 `FactorDefinition` 的关系**是组合而非替代：`FactorDefinition` 是**已注册的因子**，
`FactorProvider` 是**按参数构造因子**的工厂。provider 负责把参数集编码进返回的
`FactorDefinition::name`（如 `IDENTITY(period=5)`），使同一 provider 的不同参数化在注册表里可区分。

**一个可观察的行为**（已写进 `create` 的文档）：provider **先解析、后校验请求**，
因此名字未知时即使参数也畸形，报的仍是 `UnknownProvider`。

**验收**：`cargo test -p finkit --lib factor_provider` → **6 passed / 0 failed**，覆盖
三条错误路径（`invalid_parameter_is_rejected` / `unknown_parameter_is_rejected` /
`duplicate_parameter_is_rejected`）、`canonical_params_is_order_independent`
（`a=1,b=2` ≡ `b=2,a=1`）、`unknown_provider_is_rejected`，以及
`registry_constructs_a_runnable_definition`（构造出的定义经 `FactorRegistry` + `FactorEngine`
真实求值通过）。

### R3 已实现（真缺口 ②）

新增 `core/src/factor_graph.rs`（`lib.rs` 中以
`#[cfg(all(feature = "std", feature = "formula"))] pub mod factor_graph` 挂载）。
**没有引入第二套执行引擎**：`FactorGraph::build` 把图 lowering 成
`AstNode::Statements` 块后交给 `FormulaHotPlan::compile`，因此图与等价公式串共用同一套
lowering / CSE / DCE / kernel dispatcher。这一点由文件末尾的差分测试守住。

| 项目 | 说明 |
|---|---|
| `FactorOperation` | `Call { function }` / `Binary(BinaryOperator)` / `Constant` |
| `FactorNode` | `id` + `operation` + 输入列表 + 数值参数；builder：`new` / `binary` / `constant` / `input` / `param` |
| `FactorGraph` | `declare_input(s)` / `add_node` / `build(primary)` / `to_ast(primary)` / `used_inputs` |
| `FactorGraphError` | `DuplicateNode` / `UnknownPrimary` / `UnknownInput` / `InputArity` / `UnexpectedParams` / `ParamArity` / `Cycle` / `Plan` |
| `FactorGraphPlan` | `plan()` / `order()` / `primary()` / `external_inputs()` / `node_index(id)` |

**实现过程中发现的两条硬约束**（都写进了模块文档，因为从 AST 上看不出来）：

1. **节点必须用 `Assignment` 定义，不能只发 `Output`。**
   `hot_plan::lower_formula_plumbing` 只把 `ASSIGN:` 节点记进 `local_writes`；
   `VARIABLE:<id>` 只有在定义是 assignment 时才会被解析成别名，否则会被
   `execution_plan::InputLayout` 判成**外部输入**——plan 能编译成功，却在执行时索要一个
   与你节点同名的数据序列。所以 lowering 是「先 `Assignment` 定义全部节点，再为每个节点
   发一个 `Output` 使其可按 id 寻址，最后以裸 `Variable(primary)` 声明结果」。
   最后一句必须是裸引用而不是 primary 自己的 `Output`：拓扑序并不保证 primary 在末尾
   （没有任何节点依赖它的节点可能排在它前面）。
2. **算术必须走 `BINARY:*`，不能走 `CALL:ADD/SUB/MULT/DIV`。**
   `unified_dispatch` 只派发 `BINARY:Add|Sub|Mul|Div|...`；`CALL:SUB` 不在派发表里，
   所以把 `fast - slow` 建模成对注册表 `SUB` 函数的调用会**编译通过然后执行失败**。
   `FactorOperation::Binary` 把这个区分显式化，而不是从函数名去猜。

**顺带的一处去重**：`compute_ir::canonical_name` 从私有改为 `pub(crate)`。
公式层把每个变量名 trim + 大写，直接构造 AST 的前端必须用同一条规则，否则
`fast` / `FAST` 会在 plan 里变成同一个变量而互相覆盖——现在二者在 `add_node` 阶段就报
`DuplicateNode`。

**验收**：`cargo test -p finkit --lib factor_graph` → **10 passed / 0 failed**。其中
`graph_and_equivalent_formula_produce_the_same_series` 是差分门：
`(EMA(CLOSE,12)-EMA(CLOSE,26))/CLOSE` 分别由图与等价公式串编译执行，逐点比对
（NaN 视为相等，因为预热段两侧都是 NaN），容差 1e-12。
另有 `every_node_series_is_addressable_by_id`（校验发布出来的 `diff` 通道确实等于
`fast - slow`，而不只是长度对）、`declaration_order_does_not_affect_the_result`、
`a_cycle_is_reported_by_node_name`、`an_undeclared_input_is_rejected_rather_than_becoming_a_data_series`、
`node_kind_arity_is_enforced`、`ids_that_differ_only_in_case_are_the_same_node` 等。

### R4 已实现（真缺口 ④）

**先裁决「要不要新增第四套缓存」：不新增。** 读代码后发现 R4 的真实缺口比计划里写的更窄、也更有意思：

1. **缓存键的形状 core 早就有了。** `operation::OperationCacheKey` 已经是
   `operation` + `request` + `dialect` + `frame: FrameKey` + `data_revision: u64`，正是 R4 要求的形状。
   平行轨道的 `FactorCacheKey { symbol, factor_name, params, time_range }` **一个字段都不需要搬**：
   `symbol` → `frame.symbol`，`factor_name` → `request`，`time_range` → `data_revision`
   （这正是 R4 说的「不要照搬 `time_range: String`」）。
2. **`get_or_compute` 在 core 里根本不存在**（全仓 grep 零命中），而 `OperationCacheStats` 存在、
   却只是一个孤立结构体。后果是**命中/未命中计数被手写在三个调用点上**
   （重构前：`Factor` `:948`、`FactorBatch` `:1062`、`panel_formula` `:1313`），
   每处都重复「`get` → 自己加计数器 → 算 → `insert`」。
   所以真缺口不是「缺缓存」，而是**缺把计数收进缓存的那层抽象**。

**改动**（全部在 `core/src/operation.rs`）：

| 项目 | 说明 |
|---|---|
| `OperationResultCache`（新增，`pub`） | 把原先散在 `UnifiedOperationEngine` 上的**5 个字段**（entries / capacity / hits / misses / clock）收成一个类型 |
| `OperationResultCache::get` | **唯一**递增 hit/miss 的地方 → 一次查询必然恰好记一个 hit 或一个 miss，调用点不可能记错分支 |
| `OperationResultCache::insert` | 承担 LRU 淘汰；容量满且 key 为新才淘汰，**替换已存在的 key 不会淘汰旁人** |
| `UnifiedOperationEngine::cached_or_compute` | R4 要保留的 `get_or_compute` 抽象；三个调用点全部改走它 |
| 删除 | `get_cached_result` / `insert_cached_result` / `next_cache_tick` / `CachedOperationResult` 四个私有项 |

**一处刻意的签名偏离**（值得记下来）：`cached_or_compute` 的闭包接收 `&mut Self`，
而不是像平行轨道那样接收一个自包含的 `FnOnce() -> Result<..>`。原因是**命中路径必须保持廉价**：
编译 plan 需要 `&mut self`，若在查结果缓存**之前**就编译 plan，
则每一次结果缓存命中都会白付一次 plan 查询，并把 `factor_plan_cache_hits` 从 0 变成 1 ——
而既有测试 `multi_target_factor_execution_shares_dependencies_and_batch_cache`
正是断言「命中结果缓存后 plan 缓存命中数仍为 0」。所以**抽象保留，形状按借用检查器调整**。

**验收**：新增 5 个 `OperationResultCache` 单元测试（可脱离引擎直接测，这也是把它抽出来的理由之一）：
`result_cache_counts_exactly_one_hit_or_miss_per_lookup`、
`result_cache_isolates_entries_by_data_revision`（**`data_revision` 变化必须 miss** —— 这是正确性要求而非性能要求）、
`result_cache_evicts_the_least_recently_used_entry`、
`result_cache_replacing_a_key_does_not_evict_another_entry`、
`result_cache_capacity_and_clear_scope_their_effects_correctly`。
既有 4 个缓存行为测试（LRU 顺序、帧+revision 隔离、批量缓存、plan 缓存计数）**原样全部通过**：
`cargo test -p finkit --lib operation` → **31 passed / 0 failed**。

## 6. 收敛完成记录（平行轨道删除）

`crates/finkit-{array,series,math,factor,runtime}` 五个 crate **已整体删除**（30 个文件），
`Cargo.toml` 的 `members` 同步移除 5 项。删除前按 §2.3 的要求先做了依赖确认。

### 6.1 删除前置确认：零个生产依赖

全仓库（排除 `crates/` 自身与构建目录）检索五个 crate 的名字，命中只有三类：

| 命中位置 | 性质 |
|---|---|
| `Cargo.toml:4-8` | `members` 列表本身 —— 本次移除对象 |
| `core/src/factor_graph.rs:66`、`core/src/factor_provider.rs:22`、`core/src/operation.rs:1672` | **溯源注释**（说明移植来源），非代码依赖 |
| `docs/**`、`.workbuddy-ai/memory/**` | 文档与工作记录 |

`core` / `ffi/*` / `cli` / `wasm` / `visualization` / `factor-analysis` 均**未**以任何形式引用它们；
五个 crate 只互相引用，随本次删除一并消失。注意 `finkit-factor-analysis` 是**另一个 crate**
（`factor-analysis/`，保留），与 `finkit-factor` 无依赖关系 —— 检索时勿混淆。

### 6.2 能力覆盖表

| crate | 能力 | core 对应实现 | 判定 |
|---|---|---|---|
| `finkit-array` | `FloatArray` | `buffer_arena::BufferArena`（`core/src/buffer_arena.rs:177`）+ ndarray 缓冲 | superseded |
| `finkit-math` | `ema`、`rolling_mean/variance/std` | `math/statistics.rs:226/260/294`、`features/simd_opt.rs`、`formula/functions.rs` | superseded |
| `finkit-series` | `QuantSeries`：symbol 标签 + 长度/顺序校验 | `data_contract::TemporalSeries` + `FrameKey`（见 6.3） | superseded |
| `finkit-factor` | `trait Factor` 对象边界 | `factor_provider::FactorProvider`（R2） | **adopted (R2)** |
| `finkit-factor` | `Sma`/`Ema`/`Rsi`/`Macd` | core 指标集（TA-Lib 201/201 对齐门） | superseded |
| `finkit-factor` | `FactorResult`（多输出） | `execution_plan::OutputLayout` + `FormulaHotPlan::outputs()` | superseded |
| `finkit-runtime` | `factory`（provider + 类型化参数校验） | `core/src/factor_provider.rs` | **adopted (R2)** |
| `finkit-runtime` | `graph`（`FactorGraph`/`FactorNode`） | `core/src/factor_graph.rs` | **adopted (R3)** |
| `finkit-runtime` | `cache`、`cache_key` | `operation::OperationResultCache` + `OperationCacheKey` | **adopted (R4)** |
| `finkit-runtime` | `executor` | `unified_executor::UnifiedExecutor` + dispatcher + `BufferArena` | superseded |
| `finkit-runtime` | `scheduler` | `compute::ComputePlanError::DependencyCycle` | superseded |
| `finkit-runtime` | `registry` | `factors::FactorRegistry` | superseded |
| `finkit-runtime` | `factories` | core 指标集 | superseded |

### 6.3 §2.3 两处「合并后删除」的实测结论：无需再合并

§2.3 曾要求 `finkit-series` 的 `QuantSeries` 与 `finkit-factor` 的 `trait Factor` **先合并再删除**。
实测两者**都无需再做合并**，因为它们所声称的缺口在 core 中早已被更强的实现覆盖：

* **`QuantSeries`** —— core 的 `data_contract::TemporalSeries::new`（`core/src/data_contract.rs:68`）
  在**构造期**校验：名称非空、`timestamps.len() == values.len()`、时间戳单调；
  并额外提供显式 `align_to(target, TemporalAlignment::{Exact, AsOfClosed})` 对齐策略，
  这是 `QuantSeries` 完全没有的能力。symbol 维度由 `FrameKey::new` 承载（校验 symbol 与 timeframe 非空）。
  语义差异（**以 core 为准**）：`QuantSeries::is_valid` 要求时间戳**严格递增**，
  core 的 `validate_monotonic_timestamps` 只拒绝**下降**（允许相等）。R2 实际落地的是 provider 边界，
  并非 `QuantSeries` 迁移。
* **`trait Factor`** —— 该对象边界已由 R2 的 `FactorProvider` 承载；其 `compute(&QuantSeries)` 签名
  会拖入已被 superseded 的 `QuantSeries` 输入类型，照搬等于把要删的东西重新引入。
  `FactorResult`（多输出）亦已由 `OutputLayout` + `FormulaHotPlan::outputs()` 覆盖，
  R2 已明确记录「不搬」以避免与 `factors::FactorResult<T>`（一个 `Result` 别名）冲突。

### 6.4 与 §2.3 的一处流程偏离

§2.3 要求「每个 crate 的删除与对应阶段的合并**同一个提交**」。R2/R3/R4 已分别在
`41149fb` / `d6138be` / `c0b11f5` 提交完毕，无法回溯捆绑，故五个 crate 改为**一次性删除**。
该偏离保留了 §2.3 真正要的属性 —— **一次 `git revert` 即可整体回滚** —— 且删除不引入任何功能变更。

### 6.5 随本次删除的生成物与测试变化

* `docs/generated/version-matrix.md` 重新生成，少 5 行（被删的五个 crate）。
* 顺带修复 3 个**既有**过期生成物：`formula-functions.md`（305→308）、`indicators.md`（383→385）、
  `pine-compatibility.md`（33→45）。它们是 `26251ef` / `68b5756` / `fb70a7c` 等特性提交之后
  **未重新生成**所致 —— 也就是说 `docs-check.yml` 的 `gen_ssot_docs.py --check` 在任何全新检出上
  本来就会失败，本次一并修好。`--check` 与 `check_versions.py` 现均通过。
* `version-matrix.md` 的 `Criterion JSON benchmarks indexed` 由 **13 → 0**。该值取自本地
  `target/criterion/`；本机与 CI 全新检出均无此目录，故 **0 才是可复现值**，13 不可复现
  （保留 13 会让 `--check` 必失败）。真正的基准数据在 `docs/BENCHMARK_REPORT.md`。
  **遗留项**：把环境相关值写进被逐字节比对的生成物是设计缺陷，应把该行移出生成物或改读已提交的数据文件。
* 随 crate 一并删除 **9 个测试**（`finkit-factor` 3 + `finkit-runtime` 5 + `finkit-series` 1）。
  `cargo test -p finkit`（core）基线不受影响，仍为 **3997 passed / 0 failed**。
