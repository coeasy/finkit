# Finkit 改进优化方案（2026-09-20）

> 本文档是**唯一执行基线**。`finkit-unified-quant-platform-refactor-plan-v6.md` 保留为架构意图参考，
> `architecture-gap-assessment-2026-09-20.md` 保留为证据记录；其余历史重构计划建议归档到 `docs/archive/`（见 P2-2）。
> 所有结论以源码位置或可复现命令为依据；未验证项一律标注，不得表述为「已支持」。

---

## 0. 一句话结论

**架构方向正确，问题在「最后一公里」：统一编译执行链已建成但未接入生产路径；多语言 SSOT 的检查此前是空检查（已修）；方言层声明 5 个、实际只有 1 套解析器。**

---

## 1. 现状判定（三条硬事实）

| # | 事实 | 证据 |
|---|---|---|
| 1 | 默认公式执行是 **AST 逐节点树遍历**，`FormulaHotPlan` / `UnifiedExecutor` / `FormulaKernelDispatcher` 只被测试引用 | `core/src/formula/executor.rs:56/88`；`formula/hot_plan.rs:20`；`core/src/unified_executor.rs:169`；`formula/unified_dispatch.rs:14` |
| 2 | 多语言 SSOT 的 drift 检查此前对 7/8 语言**空过**（`docs/ffi_registry.json` 仅 python 存 71 个 body），且 `gen_binding.py` 是死路径 | 已修复，见 P0-1 |
| 3 | 方言层「声明 5 个」但 TDX/THS/EM **共用同一个 parser**；大智慧无方言枚举；文华财经无 terminal/corpus/文档；pandas-ta 完全缺失 | `formula/mod.rs:100/162-166`；`formula/compat.rs:22`；`tests/formula_corpus.rs:51`；`formula/functions.rs:5297` |

补充：`crates/finkit-{array,series,math,factor,runtime}` 无任何生产 crate 依赖（仅互相引用），是一条未接地的 V2 轨道。

---

## 2. 仓库与分支治理

### 2.1 拓扑实测结论（2026-09-20）

```
origin/HEAD -> refs/heads/main（默认分支）
main                       1b093cc   2026-09-15
perf/outperform-talib-...  6f6171b   2026-09-17   ← 是 main 的后代
feature/finkit-v1-...      f441e76   2026-09-19   ← 是 main 与 perf 的后代
```

- `main` **是** `feature` 的祖先；`perf/outperform-talib-v3-20260904` **也是** `feature` 的祖先。
- `feature` 独有 174 个提交，`perf` 独有 **0** 个。

**结论：不存在需要合并的内容。** `feature` 已 100% 包含 `main` 与 `perf`；所谓「合并」本质是一次**快进（fast-forward）**，无冲突、无合并提交。

### 2.2 本地已完成（可逆）

- 本轮 P0-1 改动已提交为 `d6306d4`（parent `f441e76`）。
- `main` 已快进到 `d6306d4`；`main` 与 `feature` 指向同一提交，工作区干净。

### 2.3 远端待决策（不可逆操作，需确认）

| 分支 | 建议 | 理由 |
|---|---|---|
| `perf/outperform-talib-v3-20260904` | **删除** | 完全被 `main` 包含，内容零丢失，可随时由 main 重建 |
| `feature/finkit-v1-unified-engine-20260917` | **暂留** | 完全被 `main` 包含，但 `multilang-release.yml:25` 的推送触发依赖 `feature/finkit-v1-unified-engine-*`。直接删除会**静默关闭发布流水线的推送触发** |
| `main` | 推送快进 | 让远端 main 结束「落后 877 个提交」的状态 |

若确定要收敛为「只有 main 一条主干」，必须先做：

1. 把 `multilang-release.yml` 的 `push.branches` 增加 `main`（或改为 `main` + `release/v*`）；
2. 检查其余 workflow 是否有 `feature/finkit-v1-unified-engine-*` 依赖；
3. 再删除 `feature` 分支。

---

## 3. 工作流

### P0-1 多语言 SSOT 闭环

**已完成（本轮）**

- `sync_bindings.py --check` 现在按语言报告覆盖率 `covered=M/N`；`covered=0` 标记 `status=UNCHECKED`，不再打印 `drift=none`，默认退出码 1；`--allow-unchecked` 为显式逃生口。
- `gen_binding.py` 空 registry 时拒绝运行（此前会让 `--check` 空过、`--generate` 写出**空绑定文件**），并标注 deprecated。

**剩余（需二选一）**

| 方案 | 做法 | 优点 | 代价 |
|---|---|---|---|
| **A. 落库 body** | 对 c/node/go/java/dotnet/ios/android 各跑一次 `--discover`，把 body 写入 `ffi_registry.json` | 沿用现有 verbatim 机制，改动最小，与 C 绑定现有做法一致 | registry 体积变大；body 与源码仍可能漂移（但此时能被 `--check` 真实检出） |
| **B. 单向生成** | 不再存 body，改为「从 Rust catalog + 声明式 schema 生成」 | 彻底消除漂移源，符合 V6 §12「统一描述 + 自动生成」 | 需先补 `c_params` / `core_call` / `copies` / `out_kind` / `core_arg_kinds` 元数据；工作量大 |

**建议**：先做 A 止血（让 `--check --all` 真实变绿），再排期 B。禁止把 A 的产物当作 B 的终态。

**验收**

1. `sync_bindings.py --check --all` 退出码 0，且每种语言 `covered>0`；
2. 人为改坏任一语言任一个函数体，`--check` 必须报 `changed:<name>` 且退出码 1（Python 路径已验证，需对 8 种语言各验证一次）；
3. CI 主流水线加入 `--check --all`，且**不允许** `--allow-unchecked`。

---

### P0-2 统一编译执行链接入生产（价值最大）

**目标**：让「统一 AST → Factor IR → 优化 DAG → Rust Runtime 执行」从文档变成事实——`FormulaEngine` 默认路径走编译计划，AST 树遍历降级为参考实现。

**步骤（必须按序，每步可独立验证）**

1. **建 differential harness**（先做，无此步不得动生产路径）
   - 对 `tests/formula_corpus/` + `tests/pine_corpus/` + `tests/contracts/formula_*` 的全部公式，同时用「AST 树遍历」与「`FormulaHotPlan` + `UnifiedExecutor`」执行，逐元素比对（含 warm-up 段、NaN/`null` 语义、多输出通道名）。
   - 落点为 `core/tests/formula_plan_differential.rs`。允许的差异必须是**显式白名单**并写明原因。
2. **编译期接入**：在 `FormulaEngine` 增加 plan 编译与缓存（按 `source + dialect + 参数指纹` 作 key），编译失败时**明确报错**而非静默回退。
3. **执行期接入**：用 `FormulaKernelDispatcher` 驱动 `UnifiedExecutor`；先覆盖无状态算子，再覆盖状态算子。
4. **切换默认**：提供 `formula_execution_mode = tree | plan`（默认 `tree`），在 differential 全绿且性能不回归后把默认改为 `plan`；`tree` 保留一个发布周期作为回滚路径。
5. **兑现优化收益**（切换后再做，逐项独立验证）：DAG CSE → buffer liveness 复用 → 并行切分 → 算子融合。

**验收**

1. differential harness 全绿（8 种语言共享同一 formula contract，见 `tests/contracts/`）；
2. `cargo test -p finkit --lib` 全绿（当前基线 2947 passed / 1 ignored，不得减少）；
3. 关键公式性能不回归（按 `docs/benchmark-baseline.json` 口径，分规模报告）；
4. `tree` 模式下行为与切换前**逐元素一致**。

**风险与回滚**：`hot_plan` 长期无流量，可能存在未被测试覆盖的分支。回滚 = 把 `formula_execution_mode` 切回 `tree`（无需回滚代码）。

---

### P0-3 方言一等化：大智慧 / 文华财经

**问题**：`tests/formula_corpus.rs:51` 把 `"dzh"` **静默映射到 `TongDaXin`**——这是真实语义错误，用户会拿到「看起来成功但语义可能不对」的结果。文华财经仅 `functions.rs:5297` 几个信号函数，无 terminal、无 corpus、文档零提及。

**步骤**

1. 新增 `FormulaTerminal::{DaZhiHui, Wenhua}` 与对应 `FormulaDialect`，停止 DZH→TDX 静默映射；未声明方言直接报错。
2. 每个新方言补：语义矩阵（函数名/参数/warm-up/输出名）+ 参考向量 + `tests/formula_corpus/*_dzh.json`、`*_wh.json`。
3. 按 `CompatibilityLevel::CommonSubset` 诚实标注，**不宣称完整兼容**。

**验收**：方言边界测试覆盖新方言；`docs/generated/*` 由脚本重新生成而非手改。

---

### P1-1 Pine `var` / `varip` 跨 bar 状态语义（静默语义错误，优先级高于新增函数）

- 现状：`pine/ast_mapper.rs:116` 把 `var` 映射为普通 `AstNode::Assignment`，`is_varip` 被忽略；`pine/runtime.rs:100` 的 bar-by-bar series 未接入 `FormulaEngine`。
- 目标：`var` 具备跨 bar 持久化语义，`varip` 具备 bar 内不重算语义；与 `core/src/streaming/` 的状态机共用同一状态定义。
- 验收：新增 Pine corpus 用例（含 `var` 累加、`varip` 场景），数值与 TradingView 参考行为一致；无法支持时必须**报错**而非静默降级。

### P1-2 终端解析真正分方言

- 现状：`formula/mod.rs:162-166` 把 TDX/THS/EM 全部路由到同一 `parse_formula`，差异只在 `SemanticProfile` 元数据层，解析阶段无法拦截终端特有语法缺口。
- 目标：方言差异落到 parse 阶段（方言专属别名表 / 语法子集），使不合法语法在解析期报错。
- 验收：为每个终端增加「应拒绝的语法」负向用例。

### P1-3 流式能力下沉到 C/C++/Java/.NET

- 现状：核心 `core/src/streaming/` 有 145/236 指标标记 streaming，但 FFI 侧 Python ~12 类、Node 28 类、Go 7 族，**C/C++/Java/.NET/iOS/Android 为 0**。
- 目标：至少把 C ABI 的 streaming handle 暴露出来，其余语言复用同一 handle 语义。
- 验收：`tests/contracts/` 增加跨语言 streaming 向量；各语言宿主实际运行。

### P1-4 性能门禁

- 现状：parity 201/201 通过；release 性能门禁**未通过**（top-20 最低 `1.0362x` vs 门槛 `1.05x`）。
- 目标：收敛 top-20 慢项（历史为 `MIDPRICE14` / `VAR20` / `WILLR14`），每轮保留全量 parity + 跨规模基准证据。
- 禁止：调整阈值、只报单机、只报平均值。

### P2-1 `crates/*` 双轨收敛

- 现状：`crates/finkit-{array,series,math,factor,runtime}` 无生产依赖。
- 决策二选一：**接入生产**（作为 P0-2 的 Runtime 载体）或**标记 deprecated 并删除**。不允许长期悬空。

**已定（2026-09-20）**：用户选择「接入生产」，但逐文件比对后该选择必须拆分执行——
平行轨道的 **9 项能力中 4 项是 core 的真缺口**（`FactorProvider` 边界 + 带类型参数校验、
声明式因子图、`QuantSeries`、`(symbol, factor, params, time_range)` 缓存键形状），
**5 项已被 core 超越**（`Executor` / `Scheduler` / 多输出 / 缓存 / kernel 与指标实现）。

因此**执行载体仍是 `UnifiedExecutor` + `HotExecutionPlan`**，平行轨道降级为**声明式前端**，
按 R1→R4 四阶段并入 core，每阶段独立可验证。完整分析、逐项反证与阶段划分见
[runtime-carrier-adoption-plan-2026-09-20.md](runtime-carrier-adoption-plan-2026-09-20.md)。

**R1 已完成**：决策落文档；5 个 crate 的 `lib.rs` 顶部标注了各自「adopted / superseded」
的判定与 core 对应实现位置，便于后续按阶段删除。无功能变更。

### P2-2 文档收敛

- 现状：`docs/` 74 个文件，≥10 份重叠架构/重构计划，判断真实进度需读 5 份以上。
- 目标：1 份权威基线（本文件）+ 1 份执行状态；其余移入 `docs/archive/`。

### P2-3 pandas-ta 风格组合 API（评估，不排期）

- 目标清单里有、当前完全缺失。建议在 P0-2 / P0-3 稳定后评估，避免过早铺开。

---

## 4. 里程碑

| 里程碑 | 内容 | 出口条件 |
|---|---|---|
| **M1** | P0-1（SSOT 闭环）+ 分支治理 + differential harness | `--check --all` 真绿并进 CI；harness 全绿；远端分支状态确定 |
| **M2** | P0-2（编译计划接入生产）+ P0-3（方言一等化） | 默认执行走 plan 且 differential 全绿；大智慧/文华方言有 corpus 与负向用例 |
| **M3** | P1-1~P1-4 + P2-1/P2-2 | Pine 状态语义正确；流式下沉；性能门禁通过；双轨与文档收敛 |

---

## 5. 门禁（沿用项目既有约定，不放宽）

**必须通过**

- `cargo fmt --check`、workspace compile、Clippy `-D warnings`；
- `cargo test -p finkit --lib`（基线 2947 passed / 1 ignored）与 `finkit-ffi-common`；
- TA-Lib 201/201 parity、dispatcher 与 catalog 一致性；
- Formula 方言边界、绘图、控制流、Pine host-required 与 temporal 对齐；
- Factor/Composite 的 batch-range-stream-checkpoint conformance；
- `sync_bindings.py --check --all`（真实覆盖，非空检查）；
- 多语言同一 JSON/typed-buffer 向量的**宿主实际运行**证据。

**禁止宣称**

- 未有参考结果的函数「数值等价」；
- 只通过 parser 的公式「可运行」；
- 只通过单机 benchmark 的「全面超过 TA-Lib」；
- 只通过某一 binding 的「八语言一致」；
- 未接入生产的「Runtime 已统一」。

---

## 6. 明确不做

- 不做行情采集、券商交易客户端、OMS/EMS、完整资管交易平台；
- 不为「新增函数数量」而新增函数；
- 不在 Research / Formula / Binding / Visualization 中新增独立算法实现（必须落到 canonical kernel）；
- 不在没有参考向量的情况下把函数推进 production catalog。

---

## 7. P0-2 执行记录（2026-09-20）

本节记录 P0-2 步骤 1（differential harness）的实际执行结果。**harness 一上线就抓出 3 个真实缺陷**，这印证了「无此步不得动生产路径」这条门禁的必要性。

### 7.1 已交付

| 项 | 落点 | 状态 |
|---|---|---|
| plan 路径纳入差分对比 | `core/tests/formula_differential_tests.rs`（新增 `run_plan` + 5 组回归用例，共 9 例） | 全绿 |
| 语料级差分门禁 | `core/tests/formula_plan_differential.rs`（新增） | 全绿 |
| 计划检视工具 | `core/examples/dump_formula_plan.rs`、`core/examples/diff_ast_plan.rs` | 新增 |

### 7.2 harness 抓出的缺陷（均已修复）

**缺陷 1：`ComputePlan::compile` 排序/去重依赖 → 非交换算子操作数被交换（静默数值错误）**

`compute.rs` 对非 `CALL:`/`STATEMENTS` 节点执行 `dependencies.sort_unstable(); dedup();`。`dependencies` 实际是**按位传递的操作数列表**，排序后 `MA(CLOSE,6)/CLOSE` 被编译成 `CLOSE/MA(CLOSE,6)`，`CLOSE+CLOSE` 被去重成单操作数加法。前者是**静默错误结果**，后者直接 arity 报错。

改法：不再改写存储的依赖顺序（拓扑排序内部已自建去重 indegree/邻接表）；新增 `compute_plan_preserves_ordered_dependencies_and_duplicates`、`compute_plan_does_not_reorder_non_commutative_operands` 固化契约。

**缺陷 2：plumbing 节点泄漏进数值计划 → 外部输入 ABI 被污染**

`ASSIGN:` / `OUTPUT:` / 局部 `VARIABLE:` / `STATEMENTS` 是语义记账节点，不产生数值。但它们留在计划里有两个后果：dispatcher 被迫实现 copy kernel；更严重的是 `InputLayout::compile` 把**每个** `VARIABLE:` 节点都当作外部输入，于是公式局部变量（`DIF`、`DEA`）被宣告为调用方必须提供的输入。MACD 的输入槽数是 3（应为 1），且 dispatcher 缺 `ASSIGN:`/`VARIABLE:`/`STATEMENTS` kernel。

改法：`hot_plan.rs` 新增 `lower_formula_plumbing`，在 CSE 之后、hot lowering 之前把 plumbing 解析为别名（`COMPOUND` 重写为等价的 `BINARY:` 节点）。MACD 计划 16 → 11 节点，输入槽 3 → 1。

**缺陷 3：无死代码消除 → 计划携带不可达节点**

`STRING_LITERAL`、被丢弃赋值的值子图都会留在计划里。Pine 脚本因 `indicator("RSI")` 的字面量直接编译失败。

改法：`hot_plan.rs` 新增 `prune_unreachable`，从 retained root 做反向可达性裁剪（必须在 plumbing 解析之后，否则过期依赖边会让已删除节点看起来可达）。

### 7.3 顺带修复的 SSOT 断裂

- `registry.rs` **缺少 `ABS`**。未注册函数在 `compute_ir::function_metadata` 落入保守兜底（`stateful: true`），于是 `CALL:ABS` 被 `add_effect` 处理并额外挂上一条**幽灵依赖**（指向上一个 effect 节点），一元 kernel 因此 arity 报错。已补注册。
- **规模远超单个函数**：`functions.rs` 注册 **327** 个函数，`registry.rs` 只声明 **104** 个。**223 个已实现函数对注册表不可见**，全部落入上述兜底路径——不能被 CSE 内联、不能被优化器推理、不能被数值 dispatcher 执行。这是当前 plan 路径剩余失败的主要根因，应作为 P0-2 的独立子项排期。
- `KernelDispatchError` 现在携带 `kernel: Option<KernelId>`，由 executor 在唯一 dispatch 调用点填充；错误信息可直接定位到具体 kernel。

### 7.4 新增 canonical kernel

`FormulaKernelDispatcher` 新增：`CALL:HHV`、`CALL:LLV`（复用 `math::kernels::{rolling_max_into,rolling_min_into}`）、`CALL:SUM`、`CALL:REF`、`CALL:ABS`、`CALL:MAX`、`CALL:MIN`。语义逐条对齐 `functions.rs` 参考实现（含 NaN 传播）。

### 7.5 当前覆盖度

| 语料 | 经 plan 路径验证 | 白名单（已写明原因） |
|---|---|---|
| `tests/formula_corpus/`（18） | **15** | 3 |
| `tests/pine_corpus/`（25） | **9** | 16 |

剩余失败归为两类，均已在 `formula_plan_differential.rs` 的白名单中逐条注明：

1. **缺 kernel**：`PLUS_DI`/`MINUS_DI`/`ADX`、`AROON_UP`/`AROON_DN`、`BOLLUP`/`BOLLMID`/`BOLLDN`、`MATH_AVG`、`DEA`、`SAR`、`IF`/`IF_THEN_ELSE`、`WINNER`/`COST`、`PERIODTYPE`/`REFDATE`。
2. **输出选择错误（设计问题，非 kernel 缺口）**：Pine 脚本末尾语句是绘图指令，retained root（「末语句的值」）落到 `hline` 常量，DCE 后计划只剩一个常量、输入槽变空。修法是保留 `OUTPUT:*` 作为命名输出（多输出支持），**不能靠加 kernel 解决**。

### 7.6 门禁状态

- `cargo test -p finkit`：**全绿**（lib 2963 passed / 1 ignored，集成测试 0 失败；基线 2947，不减反增）；
- `cargo check --workspace`：全绿；
- 新增 kernel 与缺陷 1/2/3 均有对应回归用例，白名单具备「条目变绿即失败」的反腐机制。

---

## 8. P0-2 步骤 1.5 执行记录（2026-09-20 续）：输出选择设计点已决并修复

§7.5 末尾标注的「步骤 2 之前要先解决输出选择设计点」在本轮已定案并落地。用户决策：**两条路径统一改**。

### 8.1 决策与依据

规则：**语句块的结果 = 最后一个「产值语句」（value-producing statement）**。两类节点是纯副作用，不参与结果选择：

1. **绘图指令** —— `DrawText` / `DrawIcon` / `StickLine` / `DrawGeneric`。它们把绘图命令推入上下文并返回一个临时缓冲区（**恒为全 0**）。
2. **水平标尺（level marker）** —— 带 `DrawModifier::LevelLine` 的 `Output`，即 Pine 的 `hline`。它标记价位，自身不携带序列。

关键证据（三条，均为实测而非推断）：

- `tests/pine_corpus/rsi.pine` 末尾是 `hline(30, "Oversold")`；dump 显示语义 root 为 `STATEMENTS → OUTPUT:HLINE → NUMBER(30)`，DCE 后计划塌缩成**单个 NUMBER 节点、输入槽 0 个**，故执行器报 `cannot infer execution length without bound inputs`。
- **AST 参考路径同样是退化值**：`executor.rs` 的 `DrawGeneric` 分支返回 `pool.get_buffer()`，即**全 0 数组**。所以此前 harness 一直在拿全 0 做对比——这不是 plan 路径单方面的 bug，而是「结果选择规则」在两条路径上都错。
- **字节码路径早已正确**：`OpCode::Output` 与各绘图 opcode 都是 pop 操作数、不 push 值，故 `final_value = stack.pop()` 天然就是「最后一个产值语句」。**树遍历器才是异类**，本次改动是向既有语义收敛。

### 8.2 改动清单

| 文件 | 改动 |
| --- | --- |
| `core/src/formula/ast.rs` | 新增 `DrawModifier::LevelLine`、`OutputModifier::is_level_marker()`、`AstNode::produces_value()`、`result_statement_index()` |
| `core/src/formula/executor.rs` | 三处 `Statements` 分支（`execute_val_inner` / `execute_with_pool_cached` / `execute_with_pool`）改为只取「最后一个产值语句」的值，**其余语句仍全部求值以保留副作用**，其临时缓冲归还池 |
| `core/src/formula/engine.rs` | 可选并行路径 `execute_parallel`：当末尾存在纯副作用语句时退回串行执行（该路径按分组覆盖 `last_result`，无法表达该规则） |
| `core/src/compute.rs` | `ComputeEffect` 新增 `EmitLevelMarker(String)`（与 `EmitOutput` 同为可观测，但明确「不携带序列」） |
| `core/src/schema.rs` | 新 effect 的序列化分支 `"emit_level_marker"` |
| `core/src/formula/compute_ir.rs` | `AstNode::Output` 按 level marker 选择 `EmitLevelMarker` / `EmitOutput` |
| `core/src/formula/pine/ast_mapper.rs` | `hline` 仍降级为 `Output`（保住视觉通道与 modifier），但打上 `DrawModifier::LevelLine` |
| `core/src/formula/hot_plan.rs` | `STATEMENTS` 解析取**最后一个产值依赖**（读原始依赖而非已解析依赖，否则 `OUTPUT:HLINE` 会被折叠成常量而丢失标记信息）；新增 `FormulaOutputBinding` + `FormulaHotPlan::outputs()`；`prune_unreachable` 改为**多根可达**（primary + 全部命名输出 + 全部绘图指令） |
| `core/src/registry.rs` | 补登 `MATH_AVG`（variadic） |
| `core/src/formula/unified_dispatch.rs` | 新增 `mean_formula_into`（`CALL:MATH_AVG`，变参均值） |

**多根保留是有意为之**：绘图指令不再被 DCE 静默丢弃，而是作为根保留。计划路径目前没有绘图命令出口，因此含绘图指令的公式会**响亮失败**（`unsupported kernel DRAW_*`）而不是静默丢图。这是「不静默分歧」原则的体现；待计划路径具备绘图出口后再放开。

### 8.3 覆盖率变化（编译计划路径）

| 语料 | 本轮前 | 本轮后 |
| --- | --- | --- |
| `tests/formula_corpus/`（18） | 15 | **15** |
| `tests/pine_corpus/`（25） | 9 | **13** |

Pine 转绿 4 例：`momentum`、`roc`、`rsi`（输出选择修复）、`ichimoku`、`donchian_channels`（`MATH_AVG`）。

**注意**：`donchian_channels` 由「通过」变为「因 `MATH_AVG` 失败再修复通过」。它暴露了一个此前被静默吞掉的事实：多根保留让 `basis = math.avg(upper, lower)` 这条输出通道重新进入依赖图，而它此前被 DCE 丢掉了——**即旧行为在悄悄少算一条输出**。这正是多根保留要修的问题。

### 8.4 剩余 backlog（已按成因分类，白名单逐条注明）

1. **缺 kernel**：`PLUS_DI`/`MINUS_DI`/`ADX`、`AROON_UP`/`AROON_DN`、`BOLLUP`/`BOLLMID`/`BOLLDN`、`DEA`、`SAR`、`IF`、`STOCHF`、`TRIX`、`WILLR`、`WINNER`/`COST`、`PERIODTYPE`/`REFDATE`。
2. **kernel 形状不符**：`CALL:CCI` 走的是 4 操作数 HLC kernel，而 Pine 的 `ta.cci(src, length)` 只给 2 个 → arity 拒绝。需要 src/period 形态的 CCI kernel，或在前端把 `hlc3` 类源映射到 HLC 形态。
3. **结构性 lowering 缺口**：`volume_profile` 用 `for` 循环 + `volume[i]` 序列索引，而 `compute_ir` 把循环体当作不透明控制流、不降级进无环计划，因此没有绑定输入、无法推断执行长度。这是循环降级问题，不是 kernel 问题。
4. **SSOT 缺口（仍然存在）**：`functions.rs` 实现 327 个函数，`registry.rs` 只声明 104 个。未声明的会走保守 `stateful: true` 兜底，被 `add_effect` 追加**幻影尾依赖**，从而 arity 出错。`ABS`（上一轮）与 `MATH_AVG`（本轮）都是这个成因的实例——每接一个 kernel 都要同时补登 registry。

### 8.5 第三路径收敛：字节码 VM 的结果选择缺陷（新发现）

新增的差分回归用例 `formula_differential_trailing_draw_directive_all_paths` **又抓到一个独立缺陷**——这次在字节码 VM，与本次改动无关（属既存缺陷）。

现象：`MA5 := MA(CLOSE, 5); DRAWICON(...)` 经字节码 VM 求值，`ast=NaN, bytecode=0`。

根因：`Assignment` / `CompoundAssignment` / `Output` 分别编译成 `StoreVar` / `CompoundStore` / `StoreVar + Output`，**这些 opcode 都会 pop 操作数且不 push 值**，于是 `final_value = stack.pop()` 拿到空栈，返回全 0 数组。

推论：**任何以赋值语句结尾的公式，经字节码路径都返回全 0**（例如单独一句 `MA5 := MA(CLOSE, 5)`）。此前无任何用例覆盖，故一直是绿的。

修法：在 `compile_to_bytecode` 顶层补一条 `LoadVar(name)`，把结果语句存下的值重新压栈。

- **只在顶层做**，不放进 `AstNode::Statements` 的编译分支——嵌套语句块若被当作表达式使用，绝不能扰动值栈。
- `load_variable` 是直接查 `self.variables`，而 `StoreVar` 用的就是原始名字，故 `LoadVar(同名)` 一定能取回。
- 覆盖 `Assignment` / `CompoundAssignment` / `Output` 三种会吃掉值的语句。

至此**三条执行路径（树遍历 / 字节码 / 编译计划）在「结果选择」上完全一致**，由该用例钉住。

### 8.6 语言收敛方向（用户 2026-09-20 指示）

**优先 Python、Node、Rust 三个版本；Java/C++/C/.NET/Golang 后续再扩展。先收敛项目，再继续改进优化。**

对应调整：

- P0-1 的多语言绑定工作按此收敛：Rust（本仓）→ Python → Node 为第一梯队，其余语言从「近期里程碑」降为「后续扩展」。
- P0-3 的终端方言（`FormulaTerminal::{DaZhiHui, Wenhua}`）与 `"dzh" → TongDaXin` 静默映射仍待处理，但优先级低于语言收敛。
- 先做「收敛」：把当前计划路径的门禁、白名单、文档全部对齐到实际状态，确保 `cargo test -p finkit` 与 `cargo check --workspace` 全绿后再开新战场。

---

## 9. P0-2 步骤 2 执行记录（2026-09-20 续）：编译期接入

步骤 2 的目标（见 §P0-2 步骤 2）：**在 `FormulaEngine` 增加 plan 编译与缓存（按 `source + dialect + 参数指纹` 作 key），
编译失败时明确报错而非静默回退。**

### 9.1 交付

| 项 | 落点 |
|---|---|
| plan 编译 + 缓存 | `core/src/formula/engine.rs`：`FormulaPlanCache` / `FormulaPlanKey` / `parameter_fingerprint` |
| 公开 API | `FormulaEngine::{compile_plan, compile_plan_default, plan_cache_stats, plan_cache_size, clear_plan_cache}` |
| 统计类型 | `FormulaPlanCacheStats { hits, misses }`（与 `FormulaEngine` 一同 re-export） |
| dialect 可作 key | `FormulaDialect` 增加 `Hash` derive |

`compile_plan` 的 AST 管线**刻意对齐 `eval_with_dialect`**（先 `normalize_formula_source`，再按方言解析），
再加上 `compile()` 的自定义组件展开，因此 plan 描述的就是树路径会求值的那个程序：

```
normalize_formula_source(source, dialect)
  → parse_formula_with_dialect(&normalized, dialect)
  → custom_formulas.expand(&ast)
  → apply_params(&ast, params)          // 参数替换必须在规划之前
  → FormulaHotPlan::compile(&ast)
```

### 9.2 三个 key 分量都是必需的

- **`source`** —— 调用方写的原文（未规范化）。
- **`dialect`** —— 同一段文本按不同方言解析出不同 AST（Pine 走 `parse_pine` + `map_pine_to_alphata`）。
  已知代价：AlphaTA 与 TongDaXin 目前共用 `parse_formula`，故二者会各编译一份。这是**刻意选保守的一侧** ——
  多付一次编译，而不是冒「方言不同却复用同一 plan」的风险。
- **参数指纹** —— `apply_params` 把参数引用改写成数值字面量，**早于** plan 把字面量绑进 parameter arena，
  所以 `MA(CLOSE,N)` 的 `N=14` 与 `N=20` 是**两个不同的 plan**。测试直接断言两个 plan 的 arena 里
  分别是 `14.0` 与 `20.0`，而不只是断言「键不同」。

`ParamValues` 是 `HashMap`，迭代顺序不确定，故指纹**按名排序**：同一组参数换个插入顺序必须命中同一 plan
（有专门用例钉住）。值用 `{:?}` 而非 `{}` 渲染：Rust 的 float `Debug` 输出「可往返的最短表示」，
因此能区分 `0.0` / `-0.0`（二者行为确实不同），同时把各种 NaN 折叠成同一个 key（行为确实相同）。
**缓存键不允许碰撞**，这是选 `{:?}` 而不是 `{}` 的唯一理由。

### 9.3 「明确报错」为什么复用 `InvalidOperation`

`ffi/c-binding/src/lib.rs:123-134` 对 `FormulaError` 做**穷尽 match** 来产出错误码 ABI（`ffi/node-binding` 亦然）。
**新增一个 variant 会静默改变所有语言绑定的错误码契约**，所以编译失败报
`FormulaError::InvalidOperation("formula plan compilation failed: …")` ——
与 `compile()` 既有的「语义规划失败 → `InvalidOperation`」一致；消息带前缀，日志里仍可区分。

### 9.4 缓存语义

- 与 `semantic_plan_cache` / `bytecode_cache` 一致：**不设容量上限**（进程内公式种类有限，设上限会淘汰即将复用的 plan）。
  `clear_plan_cache()` 可整体丢弃。它持有的是**数值 plan**，与按 source 单独作键的 `semantic_plan_cache`（语义 DAG）是两回事。
- **失败不缓存**：`compile_plan` 只在成功时 `insert`，因此失败不会被记成「过期成功」。
- 计数器（hits/misses）与判定它们的查询**放在同一个 `get` 里**，调用点无法记错分支 —— 与 R4 的 `OperationResultCache` 同一原则。
- **注册自定义组件会失效 plan 但保留计数器**（`invalidate` vs `clear`）：新注册的组件可能改变既有 source 的解析结果，
  所以 plan 必须丢弃；计数器仍描述本引擎的行为，故保留。

### 9.5 验收

`cargo test -p finkit --lib plan_cache_tests` → **9 passed / 0 failed**，覆盖：

同一请求命中缓存、参数值参与身份（并直接断言 parameter arena 内容）、参数插入顺序不分裂缓存、
方言参与身份、Pine 源经 Pine 解析器可编译、**不可规划公式报错且不缓存**、解析失败报错、
注册自定义组件失效 plan 但保留计数、`clear_plan_cache` 同时清计数。

其中「不可规划」用**两个绘图语句组成的块**触发（`STATEMENTS` 无产值语句 → `UnsupportedPlumbing`）。
注意**单个 `STICKLINE` 不足以触发**：它降级为普通数值节点，只会在**执行期**报缺 kernel，
所以这条用例同时钉住了「失败发生在编译期」这一点。

### 9.6 下一步

- **步骤 3（执行期接入）**：用 `FormulaKernelDispatcher` 驱动 `UnifiedExecutor`，先覆盖无状态算子，再覆盖状态算子。
  当前 `compile_plan` 返回**拥有所有权的 `FormulaHotPlan`**（内部缓存克隆一份），刻意不用 `Rc`：
  `FormulaEngine` 含 `RefCell` 已是 `!Sync`，但用 `Rc` 会进一步变成 `!Send`，可能破坏 FFI/线程化调用方。
- **步骤 4（切换默认）**：需要 `formula_execution_mode = tree | plan`，并要求 differential 全绿 + 性能不回归。
  步骤 2 的「失败即报错」正是该开关能成为**真开关**而非提示的前提。

## 10. P0-2 步骤 3 执行记录（2026-09-20 续）：执行期接入

步骤 3 的目标（见 §P0-2 步骤 3）：**用 `FormulaKernelDispatcher` 驱动 `UnifiedExecutor`，先覆盖无状态算子，再覆盖状态算子。**

### 10.1 交付

| 项 | 落点 |
|---|---|
| 执行入口 | `core/src/formula/engine.rs`：`FormulaEngine::{eval_plan, eval_plan_with_dialect, eval_plan_channels}` |
| 多通道输出 | `FormulaPlanOutput { primary, channels }`（`primary()` / `channels()` / `channel(name)` / `into_primary()`） |
| 输入绑定 | 按 `plan.input_bindings()` 的 `name → slot` 显式填槽，槽未绑满即报错 |
| **降级缺陷修复** | `core/src/formula/hot_plan.rs`：`OUTPUT:` 现在也记入 `local_writes` |
| 导出 | `core/src/formula/mod.rs` 增加 `FormulaPlanOutput` re-export |

执行路径就是 differential 门禁验证的那一条：`compile_plan` 取缓存 plan → `unified_formula_executor(&plan)` → `UnifiedExecutor::execute(&inputs)`。
树路径**完全未动**，本步只是新增入口，因此步骤 4 的开关仍有真正的回退路径。

### 10.2 接入执行期暴露了一个真实缺陷：`OUTPUT:` 未记入 `local_writes`

这是本步最重要的产出，**不是**测试问题而是生产缺陷。

`lower_formula_plumbing` 只把 `ASSIGN:` 记进 `local_writes`，`OUTPUT:` 没有。而 `InputLayout::compile` 会为**每个**
`VARIABLE:` 节点开一个输入槽、**不检查依赖**，所以 `VARIABLE:<id>` 能否解析成定义，取决于该定义是否在 `local_writes` 里。

单冒号形式 `DIF:expr` 解析出的是**单个 `AstNode::Output`**（`parser.rs`），而树路径的 `Output` 分支**会**写
`ctx.variables[name]`（`executor.rs:133-155` 与 `1039-1054` 调 `assign_var` / `assign_var_no_copy`）。
所以 `Output` 本质**同时是一次变量写**，缺了这条记录就使

```
DIF:EMA(CLOSE,12)-EMA(CLOSE,26);DEA:EMA(DIF,9);MACD:(DIF-DEA)*2
```

这一**国内终端最通用的写法**在 plan 路径上把 `DIF` 判成外部输入，要求调用方提供名为 `DIF` 的序列。

修复即在 `OUTPUT:` 分支补 `local_writes.insert(name.to_string(), node_id)`，并加了回归用例
`a_named_output_is_readable_as_a_local_variable`（断言外部输入恰好是 `["CLOSE"]`，输出是 `["DIF","DEA"]`）。

读取优先级已核对**与树路径逐条一致**：`FormulaContext::get_data` 先 `classify_builtin_var`（OHLCV/A），
再落 `ctx.variables`；树路径 `resolve_variable_zero_copy` 同样是先 builtin 再 `ctx.variables`。
两处都没有「数据列」这一层，因此不存在「输出名恰好等于某数据列」时的歧义。

### 10.3 primary 与最后一个具名输出通常是**同一块 buffer**

`ExecutionOutput::values` 按保留顺序排列，`values[i]` 对应 `output_layout().outputs()[i].1`，`values[0]` 是主结果。
但**主结果与最后一个具名输出常常指向同一个槽**（上式里是 `MACD`）。

第一版实现先取 channels 再取 primary，于是 `take` 掉了 primary 也指向的缓冲区，主结果变成空序列
（表现为 `left: 0, right: 80`）。现在改为：`values` 用 `Option` 逐个消费，**先取 primary**，
槽已被取走的 channel 回落到 `primary.clone()`。

用例 `a_multi_channel_formula_exposes_every_named_output` 钉住这一点，并额外断言 `DIF` 通道**确实带计算值**
（不能只证明「两个全 NaN 序列相等」）。另一个用例
`a_multi_channel_formula_carries_distinct_finite_series` 用 `A:MA(CLOSE,5);B:MA(CLOSE,10);C:A-B` 做**数值**验证：
`C == A - B` 逐点成立、`A` 与 `B` 不同、`C`（末句）同时是 primary 与通道，且与树路径一致。

### 10.4 为什么比较是 NaN-aware 的

`assert_eq!` 不能用于 `f64` 序列：`NaN != NaN`，而**预热期 NaN 是合法结果**。
`assert_same_series(plan, tree, tol)` 把「两边都是 NaN」也判为相等，本模块所有序列比较都走它。

顺带记录一个**既有数值特性**（两条路径行为一致，故非本步缺陷，也不在本步修）：
`ema_scalar`（`core/src/math/simd_kernels.rs:64`）以 `data[..period]` 的**简单均值**作种子，
故种子窗口含 NaN 会**污染整条序列**。因此上式里 `DEA = EMA(DIF,9)` 与 `MACD` 在全 80 根上都是 NaN
（`DIF` 前 25 根为 NaN，种子均值吃到 NaN）。这是「EMA 种子对前导 NaN 不健壮」的通用问题，
**单独列为后续项**，不混进步骤 3。

### 10.5 验收

`cargo test -p finkit` 全量：**4015 passed / 0 failed / 16 ignored**（步骤 2 后为 4006，+9 即本步新增用例）。

**differential 门禁必须重点看**，因为本步改了降级行为：

- `domestic_corpus_plan_matches_ast_reference` → ok
- `pine_corpus_plan_matches_ast_reference` → ok

即 `assert_allowlist_matches` 未被触发 —— `OUTPUT:` 修复**既没有把任何白名单用例变绿**
（现有条目全是缺 kernel 类，与输入布局无关），**也没有弄红任何原本通过的用例**。

本步新增用例：`plan_execution_tests` 8 条（无状态一致、有状态一致、预热 NaN 一致、多通道具名输出、
多通道数值一致、缺输入报错、重复求值复用缓存、缺 kernel 明确报错）+ `hot_plan.rs` 回归 1 条。

### 10.6 下一步

- **步骤 4（切换默认）**：加 `formula_execution_mode = tree | plan`，默认仍是 `tree`；differential 全绿且性能不回归后
  才切默认，并**保留 `tree` 一个版本**作为回退路径。
- **先补 registry SSOT 缺口**：`registry.rs` 只声明了公式引擎实现的 327 个函数中的 104 个，
  缺席者被降级成「未知有状态函数」（多一条幽灵尾依赖、永不纯化），这是剩余 plan 路径失败的主因，
  也是白名单里 kernel 类条目清不掉的根因。
- **EMA 种子健壮性**（见 §10.4）单独处理：要么种子只用**已就绪**的窗口，要么前导 NaN 不参与种子。
