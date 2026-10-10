# 实现现状矩阵

- 基线：`main`，HEAD `496763b7`（2026-10-08，workspace `0.2.0`）
- 状态生成日期：2026-10-10
- 依据：[架构审查与重构方案](../ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-10.md) 阶段 0

本矩阵给出"**当前源码和可执行合同**"的现状。它存在的目的是**阻止把四种不同状态
混成一个完成度百分比**：

| 状态 | 含义 |
|---|---|
| **可编译** | 该面能被 workspace 编译（`cargo check --workspace --all-targets`） |
| **专项测试** | 有对应 target 的专项/合同测试（列出文件名，不列"通过率"） |
| **parity/一致性** | 有跨实现/跨路径/跨语言的相等性门禁 |
| **CI target** | 有 CI 作业真正运行它 |
| **发行状态** | 源码存在 / CI 候选产物 / 公开注册表 三者中确切是哪一种 |

**四种状态不等于四种完成度。** 一个"可编译 + 专项测试"但没有 parity 门禁的面，
不能被读成"完成了 50%"；一个 78/78 的绑定覆盖率也不能说明公式面/研究面 parity。

## 1. 核心入口矩阵

| 面 | 源码入口 | 可编译 | 专项测试 | parity / 一致性 | CI target | 发行状态 |
|---|---|---|---|---|---|---|
| 批量指标 / 数值核 | `core/src/indicators/`、`core/src/math/` | ✅ | `golden_tests.rs`、`golden_talib_tests.rs`、`talib_coverage_matrix.rs`、`adx_nan_golden.rs`、`indicator_api_surface.rs` | TA-Lib golden；`allocating == into`（`runtime_convergence.rs`） | `test` | 源码 + v0.1.15 `.crate` |
| 公式 tree 后端（默认） | `core/src/formula/engine.rs` `eval*` | ✅ | `formula_execution_mode.rs`、`formula_differential_tests.rs`、`formula_input_bounds.rs` | `tree == plan`（rostered，`formula_plan_differential.rs`） | `test` | 源码 + v0.1.15 |
| 公式 plan 后端（opt-in） | `formula/compute_ir.rs`、`executor.rs`、`unified_dispatch.rs` | ✅ | `formula_plan_differential.rs` | 同上；tree-only 入口 `BackendUnsupported` 明确失败 | `test` | 源码（默认关闭） |
| streaming | `core/src/streaming/` | ✅ | `runtime_convergence.rs`、`repaint_tests.rs`、`long_run_stability.rs` | `batch == streaming`；注册表双向门禁 | `test` | 源码 + v0.1.15 |
| 因子图 / 统一运行时 / DirtyRange | `semantic_graph.rs`、`unified_runtime.rs`、`compute.rs` | ✅ | `factor_graph_contract.rs`、`unified_operation_seam.rs`、`runtime_convergence.rs` | `full == range`；增量保守回退 | `runtime-integration-tests` | next-release（非 v0.1.15） |
| Factor Research | `factor-analysis/` | ✅ | `research_invariants.rs`、`ssot_contracts.rs` + 47 单元 | point-in-time 不变量 | `research-test` | next-release |
| CLI | `cli/src/`、`cli/src/bin/` | ✅ | 12 单元（`csv_io`）+ `schema_cli.rs`（3） | — | `test` | v0.1.15 CLI 资产 |
| 可视化 | `visualization/` | ✅ | `polars_integration_tests.rs` 等 | — | `workspace-check` | 源码 |
| WASM | `wasm/` | ✅（host） | — | — | `future-compat`/`multilang-cross-platform` | CI 候选（`wasm32` 构建） |

## 2. 绑定矩阵

指标 surface 覆盖率（`docs/ffi_registry.json` 的 78 项）与**发行成熟度是两套指标**。

| 绑定 | 指标 surface | tier | 漂移检查 | parity 门禁 | 发行状态 |
|---|---|---|---|---|---|
| Rust `core` | 全部（即运行时本体） | active | `cargo test -p finkit` | — | v0.1.15 `.crate` |
| Python | 78 / 78 | active | ✅ `sync_bindings.py` | ✅ `audit_binding_parity.py` | v0.1.15 ABI3 wheels |
| Node | 78 / 78 | active | ✅ | ✅ | CI 候选 `.tgz`（非 npm 公开发布） |
| C | 78 / 78 | deferred | — | ✅（extractor 自检锚点） | CI 候选 SDK |
| Go | 78 / 78 | deferred | — | ✅ | 源码 + CI 候选 |
| Java | 78 / 78 | deferred | — | ✅ | CI 候选 JAR |
| .NET | 78 / 78 | deferred | — | ✅ | CI 候选 NuGet |
| iOS | 15 / 78 | deferred | — | ratchet 基线 | CI 候选 XCFramework |
| Android | 15 / 78 | deferred | — | ratchet 基线 | CI 候选 AAR |

> tier 划分的 SSOT 是 `scripts/sync_bindings.py`（active = Rust/Python/Node）。
> 78/78 只代表**登记的 indicator API**，不代表所有核心 API、方言或研究工作流完全 parity。
> 详见 [language-bindings.md](../language-bindings.md)。

## 3. 一致性与门禁映射

| 断言 | 门禁 / 测试 |
|---|---|
| 公式默认后端是 Tree | `formula_execution_mode.rs`；`check_formula_engine_contract.py` |
| tree-only 入口矩阵无空白 | 同上（13 项三方一致） |
| 批量 == 流式 == range == into == tree/plan == FFI | `runtime_convergence.rs`（五条边 rostered） |
| 缓存有界且命名空间隔离 | `engine.rs`、`unified_runtime.rs`、`formula_execution_mode.rs` |
| 绑定指标 surface 不倒退 | `audit_binding_parity.py --check`（ratchet） |
| 绑定 feature contract | `check_binding_feature_contract.py` |
| 依赖方向（kernel 不依赖 adapter） | `check_dependency_direction.py` |
| 零判定带宽 / NaN 排序 | `check_zero_guard_policy.py`、`check_nan_unsafe_ordering.py` |
| 数值语义合同 | [numerical-semantics.md](numerical-semantics.md) |
| 缓存合同 | [runtime-and-cache-contract.md](runtime-and-cache-contract.md) |

## 4. 本次核验记录（2026-10-10）

| 核验 | 结果 |
|---|---|
| `cargo test -p finkit --test formula_execution_mode` | **12/12 通过**（含新增的 `eval_multi_with_dialect(Pine)`、`eval_multi_with_pine_security` 两个 tree-only 覆盖） |
| `cargo test -p finkit-factor-analysis` | **52/52 通过**（47 单元 + 3 `research_invariants` + 2 `ssot_contracts`） |
| `cargo test -p finkit-cli` | 12 单元通过；`schema_cli.rs` 3 项在本机因 Windows `ERROR_PIPE_BUSY`（子进程管道繁忙）失败——**环境限制，非代码缺陷** |
| `python scripts/audit_binding_parity.py --check` | 通过（8 个绑定基线一致） |
| `python scripts/check_binding_feature_contract.py` | 通过（9 个绑定 feature contract） |
| `python scripts/check_dependency_direction.py` | 通过（30 条 shipped 依赖边全部向下；1 条 dev-only 边已记录） |
| `python scripts/check_formula_engine_contract.py` | 通过（13 个 tree-only 入口三方一致；默认 Tree；缓存容量一致） |

> 本表只覆盖本机能重建的面。macOS / Android / iOS / Windows-msvc 各目标未在本机重建，
> 不等同于发布门禁。`workspace-check` / `fmt` / `clippy` / `doc` 等 CI 作业仍以 CI 为准。

## 5. 待核验的旧审计结论

以下旧结论**未经本矩阵复现**，标注为"待核验"，不得直接当作当前缺陷：

- 历史上"同仓库对 NaN 有传播/拒绝/忽略多种策略"的说法：现已在
  [numerical-semantics.md](numerical-semantics.md) 按领域分域记录，并有门禁与 golden 覆盖；
  是否仍有未覆盖的 kernel 家族需逐项核对。
- "多处同源实现"的剩余清单：每保留一份重复实现都应有理由与差分测试，需继续按 kernel
  家族清点（`formula_plan_differential.rs` 的 allowlist 为空即为一项已闭合的断言）。

## 6. 相关文档

- [架构总览](overview.md)
- [公式引擎内部](formula-engine.md)
- [数值语义](numerical-semantics.md)
- [运行时与缓存合同](runtime-and-cache-contract.md)
- [语言绑定与发行状态](../language-bindings.md)
- [架构审查与重构方案](../ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-10.md)
