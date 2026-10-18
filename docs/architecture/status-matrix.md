# 实现现状矩阵

- 基线：`codex/release-v0.2.0-convergence`，最近已推送 PR head `270e02b`（性能门槛校准改动待提交）（workspace `0.2.0`）
- 状态更新日期：2026-10-18（Windows x64 本地验证 + PR #30 GitHub Actions；不代表公开发布）
- 依据：[项目功能、架构与重构审查（2026-10-18）](../PROJECT_REVIEW_AND_REFACTOR_PLAN_2026-10-18.md)
- 上一版基线：`496763b7`（2026-10-10），记录见 §5

## 当前证据快照（2026-10-18）

| 证据类别 | 当前判断 | 依据 / 限制 |
|---|---|---|
| 源码与声明 | 校准 gate 的代码提交 `d3661ca`；workspace `0.2.0` | Git working tree；发布证据须绑定最终 commit/tag |
| 静态架构审查 | 已审查 workspace、关键源码入口、feature 声明、binding tier、发布工作流 | 见 [2026-10-18 审查](../PROJECT_REVIEW_AND_REFACTOR_PLAN_2026-10-18.md)；静态存在不代表运行成功 |
| 当前候选工作树测试 / 构建 | `cargo test --workspace --locked --no-fail-fast` **通过**；Node 16/16、Python 全套及 CLI 研究端到端专项测试通过 | Windows x64；候选改动在最终提交后的验证仍需重跑，不能外推到最终 SHA |
| 当前候选 GitHub Actions | `d3661ca` 上 Docs Check 已通过；Python wheels、TA-Lib gate 与多语言工作流运行中 | Docs Check、Linux/Windows/macOS ABI3 wheels 与 Python 3.8–3.14 兼容性成功；AArch64 warning contract、WASM、iOS XCFramework/Swift typecheck、Android AAR、Rust package、Java/.NET/Go/Node 多平台及 C/C++ numeric contract 全部通过。PR 上发布资产 job 按条件跳过。基于三次独立 Actions 测量新增稳定基线和三轮中位数非回归 gate；该 gate 已在 `d3661ca` 启动，结果以本 SHA 的 Actions 为准 |
| 公开发行 | GitHub Releases 页面显示最新公开 Release 为 `v0.1.15`；`0.2.0` 是仓库当前候选版本 | [GitHub Releases](https://github.com/coeasy/finkit/releases)；候选 tag、registry 发布、平台包均须按实际完成状态更新 |

下列 §4–§6 记录保留其原始日期与 SHA，仅作历史追溯。它们不是 `8b398999` 的验证结果。

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

## 0. 证据分类（必须分开读）

每条"已实现 / 已验证 / 已发布"都必须能落到下面某一类；不得从一类外推到另一类：

| 类别 | 含义 | 本次（2026-10-13）覆盖情况 |
|---|---|---|
| **静态检查** | 读源码 / 文档 / 门禁脚本本身得出的结论 | ✅ 本次执行（§4） |
| **历史本地测试** | 旧 SHA 上本机跑过的测试记录 | 仅作历史证据（§6），**不代表当前工作树** |
| **当前 CI** | GitHub Actions 在当前 SHA 上的结果 | 本机不可查，标 **unknown**；以 CI 页面为准 |
| **公开发行** | registry / Release 资产的既成事实 | 未在本次验证，以仓库文档与 Release 页面为准 |

> §4 记录基线时点的专项检查；扩展工作树的 crate 全套测试与发布门禁见 §5。
> macOS / Android / iOS 原生目标、远端 Actions 与 registry 状态仍以 CI 和对应平台为准。

## 1. 核心入口矩阵

| 面 | 源码入口 | 可编译 | 专项测试 | parity / 一致性 | CI target | 发行状态 |
|---|---|---|---|---|---|---|
| 批量指标 / 数值核 | `core/src/indicators/`、`core/src/math/` | ✅ | `golden_tests.rs`、`golden_talib_tests.rs`、`talib_coverage_matrix.rs`、`adx_nan_golden.rs`、`indicator_api_surface.rs` | TA-Lib golden；`allocating == into`（`runtime_convergence.rs`） | `test` | 源码 + v0.1.15 `.crate` |
| 公式 tree 后端（默认） | `core/src/formula/engine.rs` `eval*` | ✅ | `formula_execution_mode.rs`、`formula_differential_tests.rs`、`formula_input_bounds.rs` | `tree == plan`（rostered，`formula_plan_differential.rs`） | `test` | 源码 + v0.1.15 |
| 公式 plan 后端（opt-in） | `formula/compute_ir.rs`、`executor.rs`、`bridge_dispatch.rs`、`unified_dispatch.rs` | ✅ | `formula_plan_differential.rs`、`formula_string_literals.rs` | 同上；tree-only 入口 `BackendUnsupported` 明确失败 | `test` | 源码（默认关闭） |
| streaming | `core/src/streaming/` | ✅ | `runtime_convergence.rs`、`repaint_tests.rs`、`long_run_stability.rs` | `batch == streaming`；注册表双向门禁 | `test` | 源码 + v0.1.15 |
| 因子图 / 统一运行时 / DirtyRange | `semantic_graph.rs`、`unified_runtime.rs`、`compute.rs` | ✅ | `factor_graph_contract.rs`、`unified_operation_seam.rs`、`runtime_convergence.rs` | `full == range`；增量保守回退 | `runtime-integration-tests` | next-release（非 v0.1.15） |
| Factor Research | `factor-analysis/` | ✅ | `research_invariants.rs`、`ssot_contracts.rs` + 47 单元 | point-in-time 不变量 | `research-test` | next-release |
| CLI | `cli/src/`、`cli/src/bin/` | ✅ | 12 单元（`csv_io`）+ `schema_cli.rs`（3） | — | `test` | v0.1.15 CLI 资产 |
| 可视化 | `visualization/` | ✅ | `polars_integration_tests.rs` 等 | — | `workspace-check` | 源码 |
| WASM | `wasm/` | ✅（host） | — | — | `future-compat`/`multilang-cross-platform` | CI 候选（`wasm32` 构建） |

> 公式入口的**逐项**后端归属（含 range/last/into 与字符串字面量的真实状态）见
> [公式入口矩阵](formula-entry-matrix.md)。该矩阵由
> `scripts/check_formula_engine_contract.py` 与源码守卫对账。

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
> 移动端 15/78 是已确认的范围决策，不列入扩容目标。
> 详见 [language-bindings.md](../language-bindings.md)。

## 3. 一致性与门禁映射

| 断言 | 门禁 / 测试 |
|---|---|
| 公式默认后端是 Tree | `formula_execution_mode.rs`；`check_formula_engine_contract.py` |
| tree-only 入口矩阵无空白（四份名册一致） | `check_formula_engine_contract.py`（engine 守卫 / formula-engine.md / runtime 合同两处名册 / 测试） |
| 批量 == 流式 == range == into == tree/plan == FFI | `runtime_convergence.rs`（五条边 rostered） |
| 字符串字面量两路径一致 + 缺口名册 | `formula_string_literals.rs` |
| 缓存有界且命名空间隔离 | `engine.rs`、`unified_runtime.rs`、`formula_execution_mode.rs` |
| 绑定指标 surface 不倒退 | `audit_binding_parity.py --check`（ratchet） |
| 绑定 feature contract | `check_binding_feature_contract.py` |
| 依赖方向（kernel 不依赖 adapter） | `check_dependency_direction.py` |
| feature 组合可构建 | `check_feature_matrix.py`（SUPPORTED 必过；KNOWN_BROKEN 必仍失败） |
| 零判定带宽 / NaN 排序 | `check_zero_guard_policy.py`、`check_nan_unsafe_ordering.py` |
| 数值语义合同 | [numerical-semantics.md](numerical-semantics.md) |
| 缓存合同 | [runtime-and-cache-contract.md](runtime-and-cache-contract.md) |

## 3.1 候选工作树复验（2026-10-18）

下列结果针对当前本地候选工作树，不是最终发布 SHA：

| 核验 | 命令 / 产物 | 结果 |
|---|---|---|
| Rust workspace tests + doctests | `PYO3_PYTHON=<有效 Python> cargo test --workspace --locked --no-fail-fast` | **退出码 0**；全部执行的测试与 doctest 通过，仓库标记的 ignored 项未执行 |
| Workspace 编译 | `PYO3_PYTHON=<有效 Python> cargo check --workspace --all-targets --locked` | 通过（14 个 crate；Windows incremental lock cleanup 有权限警告，不影响退出码 0） |
| Clippy | `PYO3_PYTHON=<有效 Python> cargo clippy --workspace --all-targets --locked` | 通过，退出码 0；pedantic 配置产生数千条既有 warning，应作为后续质量治理项 |
| 核心 rustdoc | `RUSTDOCFLAGS=-D warnings cargo doc -p finkit --no-deps --locked` | 通过；Windows incremental lock cleanup 有权限警告，不影响退出码 0 |
| docs.rs feature configuration | `RUSTFLAGS=-D warnings RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc -p finkit --no-deps --no-default-features --features "std formula serde tracing metrics formula-jit formula-simd indicators-all" --locked` | 通过；修复了原 `all-features` 与互斥 `std` / `no_std` 配置冲突 |
| crates.io package 验证 | `cargo package -p finkit --locked --allow-dirty` | 通过；541 files / 8.1 MiB，包含重新编译验证 |
| 公式执行 mode | `cargo test -p finkit --test formula_execution_mode --locked` | **14/14 通过** |
| Feature matrix | `python scripts/check_feature_matrix.py` | 通过；具体受支持/known-broken 组合数以脚本当前清单为准 |
| 依赖、绑定、文档与版本合同 | `check_dependency_direction.py`、`audit_binding_parity.py --check`、`check_binding_feature_contract.py`、`check_docs_links.py`、`check_versions.py`、`gen_ssot_docs.py --check`、`check_formula_engine_contract.py`、`check_python_stub.py`、`check_changelog.py` | 均通过 |
| Python / Node / Factor Study CLI 消费测试 | `pytest` binding suites、`npm test`、`cargo test -p finkit-cli --test factor_study_cli --locked` | Python 全套通过；Node **16/16**；CLI **1/1** |
| Python wheel preparation | `python scripts/verify_python_bindings.py` + `python scripts/prepare_python_hot_bindings.py` | 本机候选工作树通过；PR 首轮四平台 job 暴露过期源路径，现已修正并等待远端复验 |

完整命令和提交 SHA 需在最终候选提交完成后再次固定；该本地验证不代替 GitHub Actions、多平台 runner 或公开 registry 安装验证。

## 4. 本次核验记录（2026-10-13，HEAD `5854e6fb`）

本机实际执行、可复现的记录（**仅本机能重建的面**）：

| 核验 | 命令 | 结果 |
|---|---|---|
| 公式后端契约 | `cargo test -p finkit --locked --test formula_execution_mode` | **13/13 通过**（默认 Tree；13 个 tree-only 守卫；plan 可用入口填充 plan 缓存） |
| 公式差分 | `cargo test -p finkit --locked --test formula_plan_differential` | **4/4 通过**（国内/Pine 语料 tree==plan） |
| 字符串字面量 | `cargo test -p finkit --locked --test formula_string_literals` | **6/6 通过**（两路径 `string_table` 一致；`string_base` 偏移；4 个消费下标的函数在 Plan 下运行且与 tree 逐值一致；参数边界保留 Tree 既有兼容行为） |
| 相邻公式面 | `cargo test -p finkit --locked --test dzh_compat_tests --test em_compat_tests --test formula_dialect_coverage --test formula_corpus --test formula_terminal_contract --test runtime_convergence` | **94/94 通过**（43+21+2+10+1+17） |
| 公式引擎合同 | `python scripts/check_formula_engine_contract.py` | 通过（13 项 tree-only 在 **四份**名册一致；默认 Tree；缓存容量一致） |
| feature 组合 | `python scripts/check_feature_matrix.py` | 通过（**4** 个受支持组合构建成功；**6** 个 known-broken 仍按记录失败） |
| 依赖方向 | `python scripts/check_dependency_direction.py` | 通过（30 条 shipped 边全部向下；1 条 dev-only 边已记录） |
| 绑定 parity | `python scripts/audit_binding_parity.py --check` | 通过（8 个绑定基线一致；iOS/Android 15/78 属 ratchet 基线） |
| 绑定 feature contract | `python scripts/check_binding_feature_contract.py` | 通过（9 个绑定编译其声明的 feature contract） |
| 版本一致 | `python scripts/check_versions.py` | 通过（全部为 `0.2.0`） |
| 文档链接 | `python scripts/check_docs_links.py` | 本次曾失败：`docs/README.md` 指向一个**未被 git 跟踪**的新文档；已通过纳入跟踪修复 |

> 上表记录的是 2026-10-13 首轮公式/结构检查。2026-10-14 扩展工作树的
> workspace、发布和多语言包验证见下节；二者不应被混作同一个 HEAD 记录。

## 5. 扩展工作树验证记录（2026-10-14，基于 `5854e6fb`）

本节记录在该基线之上的审查与修复工作树，所有结论只覆盖 Windows x64 本机：

| 核验 | 命令 / 产物 | 结果 |
|---|---|---|
| Rust crate 全套测试 | `cargo test -p finkit -p finkit-factor-analysis -p finkit-cli -p finkit-visualization -p finkit-ffi-common -p finkit-ffi -p finkit-go -p finkit-dotnet -p finkit-java -p finkit-android -p finkit-ios -p finkit-wasm --locked --no-fail-fast` | 12 个 crate 通过 |
| Workspace 构建 | `PYO3_PYTHON=<有效 Python> cargo check --workspace --locked` | 通过 |
| Clippy CI 合同 | `cargo clippy -p finkit --locked`；`cargo clippy --workspace --all-targets --locked`，设 `RUSTFLAGS=-D unfulfilled_lint_expectations` | 通过；仓库 warning budget 允许既有 lint 警告，严格 `-D warnings` 不属于项目 CI 合同 |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p finkit --no-deps --locked` | 通过。`check_rustdoc.sh` 的 WSL 启动器在该主机不可用，直接执行策略命令成功 |
| crate 发布包 | `cargo package -p finkit --locked --allow-dirty` | 541 文件 / 8.1 MiB，验证成功 |
| 使用接口包 | `bash scripts/build-usage-packages.sh` | Windows x64：Python、Node、Java、Go、C、.NET、WASM 七包全部 clean-consumer 验证通过；44 个 manifest 组件 |
| 候选 bundle | `dist/finkit-0.2.0-windows-x64-usage-bundle.zip` | 26,280,558 字节；7 种语言、44 个组件；包 manifest 检查通过 |
| C 原生 archive | `dist/native/windows-x64/finkit-0.2.0-native-windows-x64.zip` | 6,904,528 字节；SHA-256 `ebbf42746db5b4f4fbe70dbe04e64850223f34337b8be080f08bc4a603d15474` |
| 未验证平台 | Linux/macOS、iOS/Android 原生 runner、远端 Actions、registry 发布 | 未覆盖，不据此宣称这些平台已发布或通过 |

## 6. 历史核验记录（2026-10-10，HEAD `496763b7`）——仅供追溯

以下为**旧 SHA** 上的本机记录，**不得当作当前 HEAD 的验证**：

| 核验 | 结果 |
|---|---|
| `cargo test -p finkit --test formula_execution_mode` | 12/12 通过 |
| `cargo test -p finkit-factor-analysis` | 52/52 通过 |
| `cargo test -p finkit-cli` | 12 单元通过；`schema_cli.rs` 3 项在本机因 Windows `ERROR_PIPE_BUSY` 失败——环境限制，非代码缺陷 |
| `python scripts/audit_binding_parity.py --check` | 通过 |
| `python scripts/check_dependency_direction.py` | 通过（30 条边） |

> 与当前 HEAD 的差异示例：本次 `formula_execution_mode` 为 **13/13**（旧记录 12/12），
> 说明测试面在两次基线之间确有变化——这正是"不得从旧 SHA 绿灯推断当前 HEAD"的理由。

## 7. 待核验的旧审计结论

以下旧结论**未经本矩阵复现**，标注为"待核验"，不得直接当作当前缺陷：

- 历史上"同仓库对 NaN 有传播/拒绝/忽略多种策略"的说法：现已在
  [numerical-semantics.md](numerical-semantics.md) 按领域分域记录，并有门禁与 golden 覆盖；
  是否仍有未覆盖的 kernel 家族需逐项核对。
- "多处同源实现"的剩余清单：每保留一份重复实现都应有理由与差分测试，需继续按 kernel
  家族清点（`formula_plan_differential.rs` 的 allowlist 为空即为一项已闭合的断言）。

## 8. 相关文档

- [架构总览](overview.md)
- [公式引擎内部](formula-engine.md)
- [公式入口矩阵](formula-entry-matrix.md)
- [数值语义](numerical-semantics.md)
- [运行时与缓存合同](runtime-and-cache-contract.md)
- [语言绑定与发行状态](../language-bindings.md)
- [架构审查与重构方案（2026-10-13）](../ARCHITECTURE_REVIEW_AND_REFACTOR_PLAN_2026-10-13.md)
