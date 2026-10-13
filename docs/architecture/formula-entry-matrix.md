# 公式入口矩阵（Tree / Plan 逐项 SSOT）

- 基线：`main`，HEAD `5854e6fb`（2026-10-13，workspace `0.2.0`）
- 状态：**合同文档（单一事实来源）**。本页把 `FormulaEngine` 的每个公开入口按
  执行后端归属逐项列出，并给出语法/方言/参数/多输出/字符串/流式/range/last/into
  在 Tree 与 Plan 下的支持状态。
- 依据：`core/src/formula/engine.rs` 的 `require_tree_backend(..)` 调用点、
  `core/tests/formula_execution_mode.rs` 的 `assert_tree_only!` 名册、
  `core/tests/formula_string_literals.rs` 的字符串字面量合同。
- 相关：[公式引擎内部](formula-engine.md)、[公式运行时合同](../formula-runtime-contract.md)、
  [实现现状矩阵](status-matrix.md)。

本页存在的理由：`docs/architecture/formula-engine.md` 与
`docs/formula-runtime-contract.md` 各自有一份入口划分，但两处曾出现"条目数一致、
成员不一致"的漂移（runtime 合同的 §3.4 表少列了 `eval_multi_with_dialect(Pine)`）。
把逐项矩阵收敛到本页，并让 `scripts/check_formula_engine_contract.py` 同时校验三处
文档与源码守卫，是防止再次漂移的手段。

## 1. 后端归属（穷尽分类）

`FormulaExecutionMode::{Tree, Plan}` 是**后端契约，不是提示**：选定后端无法服务某入口时
必须报 `FormulaError::BackendUnsupported`，不得静默回退。

### 1.1 受管辖、plan 可用（source-level）

| 入口 | 说明 |
|---|---|
| `eval` | 主入口；plan 模式下填充 `plan_cache` |
| `eval_with_dialect` | 显式方言；四类国内方言走 `eval_multi` |
| `eval_with_params` | 参数化；Plan 分支走 `eval_plan_channels` |
| `eval_multi` | 多输出（`MultiOutput`） |
| `eval_multi_with_dialect` | 多输出 + 方言；**Pine 除外**（见 §1.2） |
| `eval_incremental` | 增量语义入口 |
| `eval_simd` | 冻结别名，语义等价 `eval`（feature `formula-simd`） |
| `eval_batch` | 批量；无副作用公式走共享编译路径 |
| `eval_batch_shared` | 批量共享编译 |
| `eval_partial` | 返回 `(结果, 诊断)`；内部委托 `eval`，因此继承所选后端 |

> `eval_partial` 此前未被两份文档列入任何一类，本页把它补进 plan 可用集合。
> 它委托 `self.eval`，Plan 模式下同样走 plan 后端。

### 1.2 受管辖、仅 tree

以下 13 个入口在选定 `Plan` 时返回
`FormulaError::BackendUnsupported { backend: "plan", entry }`：

`eval_ast`、`eval_lazy`、`eval_parallel`、`eval_optimized`、`eval_with_debug`、
`eval_template`、`eval_with_validation`、`eval_with_defaults`、`eval_zero_copy`、
`eval_zero_copy_cached`、`eval_zero_alloc`、`eval_multi_with_dialect(Pine)`、
`eval_multi_with_pine_security`。

其中 `eval_multi_with_dialect(Pine)` 是**方言限定**的：同一入口在 AlphaTA /
TongDaXin / TongHuaShun / EastMoney 下属于 §1.1（plan 可用），只有 Pine 分支被守卫。

#### 1.2.1 逐项处置（阶段 1.2 的结论）

阶段 1.2 要求为每个仅 tree 入口给出**明确处置**，而不是留在"还没决定"。
处置取值为受控词表：

| 处置 | 含义 |
|---|---|
| `by-design` | 该能力本质上是 tree 侧的（AST 级变换、分配策略、诊断插桩），**不打算**给出 plan lowering |
| `backlog` | 可以且值得 lowering，但尚未实现——是真正的缺口 |
| `deprecate` | 计划移除或折叠进别的入口 |

`Plan 替代` 列给出选定 Plan 后应改用的入口；`—` 表示没有直接替代。
该列（非 `—` 时）必须是**非**仅 tree 的入口，否则替代本身也会报
`BackendUnsupported`——这条由门禁校验。

| 入口 | 处置 | 理由 | Plan 替代 |
|---|---|---|---|
| `eval_ast` | `by-design` | 按定义是进入 tree/bytecode/JIT 流水线的通道（供 Pine 等方言复用）；`AstNode` 本身可 lower，但改变本入口语义不是收敛方向，见 §1.3.1 | `eval_plan` |
| `eval_lazy` | `by-design` | 惰性求值是对 AST 的剪枝，plan 图在 lowering 时已定序，无等价语义 | `eval` |
| `eval_parallel` | `by-design` | 并行调度基于 AST 结构；plan 执行器没有并行调度器 | `eval` |
| `eval_optimized` | `by-design` | 显式 AST 优化 pass；plan 侧有**自己**的优化（CSE/layering），重复套用会改变语义 | `eval` |
| `eval_with_debug` | `by-design` | 插桩的是 tree walker 的求值轨迹；plan 执行器不暴露逐节点轨迹 | `eval` |
| `eval_template` | `by-design` | 模板展开是 source 级关注点；展开后交给 `eval` 即可 | `eval` |
| `eval_with_validation` | `backlog` | 参数校验 + AST 改写；`eval_with_params` 已证明 plan 侧可接收参数，此处只是尚未接线 | `eval_with_params` |
| `eval_with_defaults` | `backlog` | 同上：默认值注入是 AST 改写，plan 侧可由参数通道承担 | `eval_with_params` |
| `eval_zero_copy` | `by-design` | 承诺的是 **tree 执行器的**借用输入策略；plan 路径本就以 `&[&[f64]]` 借用输入，语义不同名同 | `eval` |
| `eval_zero_copy_cached` | `by-design` | 同上，另加 `VarNameCache` | `eval` |
| `eval_zero_alloc` | `by-design` | 走 `execute_val` + `to_array` 的分配策略，属执行器实现细节 | `eval` |
| `eval_multi_with_dialect(Pine)` | `backlog` | Pine 的 plan lowering 缺失（方言分支，非入口问题） | `eval_multi_with_dialect`（非 Pine 方言） |
| `eval_multi_with_pine_security` | `backlog` | 依赖 host 提供的 `request.security` 解析器，plan 侧尚无该通道 | `eval_multi` |

**计数**：`by-design` 9、`backlog` 4、`deprecate` 0（合计 13）。

**四个 `backlog` 的共性**：都不是"入口不能跑"，而是**尚未接线**，分两类——

- **方言/host 通道**（2 项）：`eval_multi_with_dialect(Pine)`、`eval_multi_with_pine_security`；
- **参数注入通道**（2 项）：`eval_with_validation`、`eval_with_defaults`
  （`eval_with_params` 已证明 plan 侧能接收参数，二者只是没接上去）。

因此它们都不应被读成"Plan 默认化的阻塞项"。按 §3.1，range/last/into 也**不是**阻塞项
（后端显式，`execution_mode` 不影响）；真正的默认化依据是**真实 workload 的
覆盖/性能/内存证据**（阶段 1.4）。

### 1.3 后端显式（不受 `execution_mode` 管辖）

这些入口的**参数类型本身就是后端选择**，因此不受 `FormulaExecutionMode` 影响：

| 入口组 | 参数类型 | 运行的后端 |
|---|---|---|
| `eval_plan`、`eval_plan_with_dialect`、`eval_plan_channels` | `&str` | 恒为 plan |
| `compile_plan`、`compile_plan_default` | `&str` | 产出 `FormulaHotPlan` |
| `compile` | `&str` | 产出 tree 产物 `CompiledFormula` |
| `execute`、`eval_into`、`eval_range`、`eval_range_zero_copy_inputs`、`eval_last`、`eval_zero_copy_inputs`、`execute_zero_copy_cached` | `&CompiledFormula` | 恒为 tree |
| `compile_bytecode`、`execute_bytecode` | `&str` / `&Bytecode` | 冻结 bytecode 面 |
| `compile_jit`、`execute_jit`、`eval_jit` | `&str` / `&OptimizedBytecode` | 冻结兼容面（无 native codegen） |

#### 1.3.1 判据：后端中立输入 vs 编译产物（消解 §1.2 的疑似不一致）

§1.2 曾记录一处"待决"：`eval_ast` 接收 `&AstNode` 却受管辖，而 `eval_range` 接收
`&CompiledFormula` 却后端显式。**该不一致由以下判据消解，无需改动任何公开契约**：

| 判据 | 含义 | 归类 |
|---|---|---|
| **编译产物** | 参数类型是**某个后端编译器的产物**：`CompiledFormula`（tree 编译器）、`Bytecode`、`OptimizedBytecode`、`FormulaHotPlan`（plan 编译器） | 后端显式（§1.3） |
| **后端中立输入** | 参数类型与后端无关：`&str` 源码、`&AstNode` AST、数值切片 | 受管辖（§1.1 / §1.2） |

理由是**产物即选择**：持有 `CompiledFormula` 的调用方已经通过 `compile()` 选定了 tree
后端，`execution_mode` 再无从置喙；而 `&AstNode` 只是 `parse_formula*` 的返回值，两种后端
都能消费它——`build_plan` 内部正是 `parse → ast → FormulaHotPlan::compile(&ast)`，
**AST 是可 lower 的**。

因此 `eval_ast` 留在 §1.2 是**判据正确的结果**，不是遗漏。它 tree-only 的原因是
**该入口按定义就是进入 tree/bytecode/JIT 流水线的通道**（见其文档注释：供 Pine 等方言
复用"bytecode、JIT、SIMD、partial-eval"），而非"AST 无法 lower"。

> 若将来需要"持有 AST 且走 Plan"，正确做法是新增一个 AST 形参的 plan 入口
> （判据上它属于后端中立输入），而不是把 `eval_ast` 改成 Plan——那会改变该入口的语义。
> 该项登记在 §3.2，属**未实现的可选项**，非缺口。

### 1.4 分析 / 元数据（后端无关）

不执行数值、不选后端：`analyze`、`analyze_ast`、`metadata`、`metadata_for_formula`、
`get_param_defs`，以及缓存统计/清理族（`cache_hit`、`cache_size`、`clear_cache`、
`clear_plan_cache`、`plan_cache_size`、`plan_cache_stats`）。

## 2. 能力矩阵（source-level 入口）

图例：✅ 支持；➖ 该入口不提供该维度；⚠️ 有限制（见脚注）。

| 入口 | 语法 | 方言 | 参数 | 多输出 | 字符串字面量 | 流式/增量 | range | last | into | Plan 后端 |
|---|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| `eval` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_with_dialect` | ✅ | ✅ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_with_params` | ✅ | ➖ | ✅ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_multi` | ✅ | ➖ | ➖ | ✅ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_multi_with_dialect` | ✅ | ✅ | ➖ | ✅ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ⚠️² |
| `eval_incremental` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ✅ | ➖ | ➖ | ➖ | ✅ |
| `eval_simd` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_batch` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅³ |
| `eval_batch_shared` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅³ |
| `eval_partial` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ✅ |
| `eval_ast` | AST | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_lazy` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_parallel` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_optimized` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_with_debug` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_template` | 模板名 | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_with_validation` | ✅ | ➖ | ✅ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_with_defaults` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_zero_copy` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_zero_copy_cached` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_zero_alloc` | ✅ | ➖ | ➖ | ➖ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |
| `eval_multi_with_pine_security` | ✅ | Pine | ➖ | ✅ | ✅¹ | ➖ | ➖ | ➖ | ➖ | ❌ |

**脚注**

1. **字符串字面量**：两条路径都能把字面量文本按同一顺序写入
   `FormulaContext::string_table`，并发出**绝对表位置**（plan 侧由
   `unified_dispatch` 的 `string_base` 加上编译期局部下标）。消费该下标的四个调用目标
   `EM_REF`、`BLOCKDATA`、`BLOCKINDEX`、`BLOCKAVG` 也已有 plan kernel：它们经
   dispatcher 的 `string_table` 解析下标，再读 `ctx.em_data` / `ctx.block_data`
   （随 `HostContext` 进入 plan 执行器）。无 host 数据 → 全 `NaN`（与 tree 一致）；
   有 host 数据 → 逐值一致。字符串字面量在 Plan 路径上已无已知缺口。
2. `eval_multi_with_dialect` 在 **Pine** 方言下是 §1.2 的仅 tree 入口；其余方言为 ✅。
3. `eval_batch` / `eval_batch_shared` 在包含**可观测副作用**（赋值/输出/绘图）时按顺序
   求值，不跨副作用复用结果；Plan 支持以"全部公式无副作用时走共享编译路径"为条件。

## 3. range / last / into 的后端事实（阶段 1.3 的界定）

`range`、`last`、`into` 三个维度目前**只存在于 tree 产物入口**：`eval_range`、
`eval_range_zero_copy_inputs`、`eval_last`、`eval_into`、`eval_zero_copy_inputs`、
`execute_zero_copy_cached` 都接收 `&CompiledFormula`（tree 产物），因此按 §1.3.1 的
**编译产物**判据恒为 tree。

### 3.1 阶段 1.3 的结论：`by-design`，非缺口，非默认化阻断项

| 问题 | 结论 | 依据 |
|---|---|---|
| 切到 Plan 会不会改变这些入口的行为？ | **不会** | 它们按 §1.3.1 是后端显式：参数类型即后端选择，`execution_mode` 不参与 |
| 这算"静默回退"吗？ | **不算** | 回退的判据是"选定后端无法服务却仍执行"。此处后端由**产物类型**选定，从未声明为 plan |
| Plan 侧有没有 range/last 能力？ | **有，但故意不暴露** | `UnifiedExecutor::execute_range` / `execute_last` 已实现并经 `runtime_convergence.rs` 覆盖，但只经 `unified_formula_executor*` 可达，不经 `FormulaEngine` |
| 现在要不要加 `eval_plan_range` 之类的入口？ | **不加** | 无调用方需要；加公开入口会扩大公共面而无证据支撑 |
| 绑定怎么用？ | 走 tree 产物入口 | python-binding 的 `eval_range` / `eval_last` / `append_bar` 全部基于 `CompiledFormula` |

**登记为 `by-design`（internal capability）**：Plan 侧范围执行是**执行器内部能力**，
不是 source-level 入口。复评触发条件（满足其一即可重新评估）：

1. 有绑定或下游需要**经 plan 后端**做区间执行（例如增量图表刷新想拿到 plan 的性能）；
2. 阶段 1.4 的真实 workload 测量显示区间执行是 plan 默认化的瓶颈路径；
3. `FormulaHotPlan` 获得跨调用持久状态，使 plan 侧 `last` 的流式语义可与 tree 的
   `try_eval_last_streaming_*` 快路径对齐。

在此之前，`range`/`last`/`into` 在 Plan 侧**没有** source-level 入口，这一事实本身
不是待办，而是已定契约。

### 3.2 明确排除项（记录，不实施）

- **`eval_ast` 的 plan 对应入口**：见 §1.3.1。`AstNode` 可 lower，但没有"持有 AST 且走
  Plan"的公开入口。无调用方需要，不新增。
- **把 `eval_range` 等改成受管辖**：会让 `execution_mode = Plan` 下的既有产物型调用方
  报错，属公共行为变化，且与 §1.3.1 判据矛盾。不做。

## 4. 门禁

| 断言 | 门禁 |
|---|---|
| 仅 tree 名册三方一致（engine 守卫 / formula-engine.md / 本页 §1.2 / formula_execution_mode.rs） | `scripts/check_formula_engine_contract.py` |
| runtime 合同的仅 tree 名册与源码一致 | `scripts/check_formula_engine_contract.py` |
| 默认后端是 Tree | `formula_execution_mode.rs`；`check_formula_engine_contract.py` |
| Plan 可用入口确实填充 plan 缓存 | `formula_execution_mode.rs`（`plan_capable_entries_stay_governed`） |
| 字符串字面量两路径一致 + 缺口名册 | `formula_string_literals.rs` |
| 每个 plan 内核都已在 registry SSOT 声明（未声明会退化为 stateful barrier） | `formula_function_ssot.rs`（`every_plan_kernel_is_registered_in_the_ssot`） |
| 三面（registry / formula / plan 内核）尺寸与记录一致 | `formula_function_ssot.rs`（`the_three_surfaces_have_the_expected_sizes`） |
| 跨路径数值一致 | `formula_plan_differential.rs`、`runtime_convergence.rs` |

## 5. 绑定后端能力矩阵（阶段 1 第 3 条：绑定能否选择/报告 backend）

方案阶段 1 第 3 条要求检查活跃绑定**能否选择或报告 backend**。逐仓核查结果：
**没有任何绑定能选择或报告后端**。全部绑定都调用 `FormulaEngine::new()`，即
**静默使用默认（Tree）**；唯一与后端有关的代码是 `BackendUnsupported` 的**错误文案**。

| 绑定 | 能选择 backend？ | 能报告 backend？ | `BackendUnsupported` 映射 |
|---|---|---|---|
| c-binding | ❌ | ❌ | 错误码 `11` |
| python-binding | ❌ | ❌ | `RuntimeError`（消息含 entry + backend） |
| node-binding | ❌ | ❌ | `napi::Status::GenericFailure` |
| dotnet-binding | ❌ | ❌ | 字符串（无类型化分支） |
| go-binding | ❌ | ❌ | 字符串（无类型化分支） |
| java-binding | ❌ | ❌ | 字符串（无类型化分支） |
| ios-binding | ❌ | ❌ | 字符串（无类型化分支） |
| android-binding | ❌ | ❌ | 字符串（无类型化分支） |

### 5.1 这是默认化的**第二个阻断项**（第一个是阶段 1.4 的性能退化）

若把默认切到 Plan，**每个绑定的行为都会静默改变**：绑定使用者既无法**显式钉住 Tree**
以规避回归，也无法**查询自己当前在哪个后端**。方案退出条件要求"所有绑定默认/显式
backend 行为一致且有迁移说明"——在补齐选择/报告面之前，该条件**不满足**。

### 5.2 附带发现：`BackendUnsupported` 的跨语言映射不一致

同一错误在 8 个绑定里有 4 种表现：错误码 `11`（C）、`RuntimeError`（Python）、
`GenericFailure`（Node）、以及**完全无类型化分支**（其余 5 个，退化为字符串）。
绑定使用者无法用同一套逻辑跨语言识别"后端不支持"，这本身也是"行为一致"要求的缺口。

> 修复顺序建议：先给每个绑定加**只读**的 backend 报告（`engine.backend()`，成本极低、
> 无行为风险），再加可选的显式选择；最后统一 `BackendUnsupported` 的映射并写迁移说明。
> 在默认值切换**之前**完成，否则使用者没有退路。

## 6. 相关文档

- [公式引擎内部](formula-engine.md)
- [公式运行时合同](../formula-runtime-contract.md)
- [实现现状矩阵](status-matrix.md)
- [架构审查与重构方案（2026-10-13）](../ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-13.md)
