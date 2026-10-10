# Finkit 架构、开发细节与分层重构总纲 V5

> **基线**：本机当前 `HEAD`，workspace 版本 `0.2.0`，核验日期 2026-10-07。
> **证据分级**：本文每条结论按来源标注——
> - `[实测]` 本次在 HEAD 上由只读命令/脚本直接得到；
> - `[V4]` 引自 V4 滚动审计日志（4737 行、追加到第 27 轮；该文件已于 2026-10-10
>   从工作树删除，可用 `git log --diff-filter=D --
>   docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` 恢复），本次未在 HEAD 上完整复现；
> - `[待核验]` 需要专门方法学或跨平台环境才能确认，本文不作确定性结论。
>
> **与历史方案的关系**：V4 是滚动式审计日志，作为**历史证据来源**保留在 Git 历史中。
> 本文是**稳定的当前基线总纲**；V4 中尚未结清的条目在 [第三部分](#第三部分-基线核验v4-遗留项在-head-的状态) 逐项核验。

---

## 0. 执行摘要

Finkit 已经是一套**成熟的 Rust-first 量化计算引擎**，不是骨架，也不是待重写的原型：

1. **主体架构方向正确，不需要推倒重写。** 14 个 workspace 成员、约 32 万行 Rust，主要链路真实存在且已连通。
2. **工程治理已相当扎实。** 62 个测试 target、22 个 `check_*.py` 内务门禁、11 个 CI workflow；本次抽验的结构性门禁（孤儿模块、源码可达性、无界循环、dead_code、tracked 构建产物）全部为绿 `[实测]`。
3. **性能已在多数项目上超过 TA-Lib C。** 90 项配对中 78 项领先或持平，0 项显著落后，12 项落在 0.91–0.99 的窄带 `[实测]`。
4. **真正的剩余问题集中在"一致性"而非"功能缺失"**：同一指标存在多份手抄实现、除法守卫政策未统一、**绑定 feature 声明与实际编译态不符**、文档与已冻结契约漂移。这些是重构主轴。

**重构主轴排序（为什么这样排）**：把**语义一致性**排在结构解耦之前——多份手抄实现直接威胁产品承诺（canonical numerical semantics），一处政策改动要同步多个实现点，漏一处就产生数值漂移；而超大文件只影响可读性与维护成本，不改变对外行为。因此路线是：
**基线核验 → 语义统一 → 结构解耦 → 运行时收敛 → 工程与文档治理 → 多语言编译态确定性**；崩溃面（Batch 6）安全相关，可与语义统一并行。

**本版新增（2026-10-07，第三轮核心逻辑深挖）**：在 [4.7](#47-核心逻辑深度审查三轮2026-10-07) 记录 5 项可复核发现——NaN 三态不一致、零判定带宽偏离 TA-Lib 的 1e-8、repaint 代码生成宏零引用而 10 份快照手抄、运行时缓存策略不统一且部分无界、递归深度防护为事后检查且默认关闭。它们分别落到 Batch 1/3/6。

---

# 第一部分 架构全景

## 1.1 产品定位与边界（约束条件）

Finkit 的定位是**计算引擎**，不是策略平台。以下边界是**已确认的产品定调**，重构不得越过 `[V4]`：

- **不涉及回测、不涉及选股。** 相关模块（`backtest`、`backtest_evaluation`、`selectors`、`sector`）已删除，不重新引入。
- **JIT / `eval_simd` 冻结保留，不删除。** 四个语言绑定将其导出为公开 API，直接删除是跨语言破坏性变更；只接受正确性修复，不再新增能力，且 `eval()` 永不路由到它们。真正的 SIMD 在内核层（`math::simd_kernels`）的常规路径上。
- **公式执行路径只保留 `tree`（参考实现）+ `plan`（生产）。**
- **`chan`（缠论）属计算职责，保留。**

## 1.2 Workspace 拓扑

14 个成员 `[实测]`：

```text
finkit (core)              —— 引擎本体：指标 / 公式 / 流式 / 特征 / 因子 / 数学内核
├─ factor-analysis         —— 面板感知的因子研究工作流
├─ visualization           —— 图表渲染（SVG/PNG/HTML/WebGL）
├─ cli                     —— 命令行前端
├─ wasm                    —— 浏览器 / Node WASM 绑定
└─ ffi/
   ├─ ffi-common           —— 跨语言共享契约（错误/JSON/所有权/panic 边界）
   ├─ c-binding            —— 纯 C ABI
   ├─ python-binding       —— PyO3
   ├─ node-binding         —— napi-rs
   ├─ java-binding         —— JNI
   ├─ go-binding           —— cgo
   ├─ dotnet-binding       —— P/Invoke
   ├─ ios-binding          —— 移动端 shim
   └─ android-binding      —— 移动端 shim
```

依赖方向单一：所有绑定只依赖 `core`（+`ffi-common`）；`cli` 额外依赖 `visualization`。`core` 不依赖任何绑定。

## 1.3 六层分层模型

```text
┌───────────────────────────────────────────────────────────────┐
│ L6 交付层 (Delivery)                                           │
│     Python / Node / Java / Go / .NET / C / iOS / Android        │
│     WASM / CLI                                                  │
├───────────────────────────────────────────────────────────────┤
│ L5 主机契约层 (Shared Host Contract)  = ffi-common              │
│     operation / error / JSON / ownership / panic 边界 /         │
│     golden / research 契约                                      │
├───────────────────────────────────────────────────────────────┤
│ L4 统一运行时层 (Unified Runtime)                               │
│     SemanticGraph → ComputePlan / FactorPlan → UnifiedRuntime   │
│     UnifiedExecutor / RuntimeContext / BufferArena / StateArena │
├───────────────────────────────────────────────────────────────┤
│ L3 规范语义层 (Canonical Semantic Layer)                        │
│     Formula AST / Operation / FactorGraph                       │
│     dependency / lookback / capabilities / effects              │
├───────────────────────────────────────────────────────────────┤
│ L2 规范数值内核层 (Canonical Numeric Kernel Layer)              │
│     math / indicators / streaming / patterns / transforms       │
│     batch / streaming / SIMD                                    │
├───────────────────────────────────────────────────────────────┤
│ L1 研究与可视化层 (Research & Visualization)                    │
│     factor-analysis (prepare/IC/validation/risk/report)         │
│     visualization (scene/layout/render/SVG/HTML/WebGL)          │
└───────────────────────────────────────────────────────────────┘
```

分层的关键契约：**上层只经由 L3 的语义对象与 L4 的计划对象调用下层**，不允许绑定层直接触碰 L2 内核（否则数值语义会在各语言各写一份）。

## 1.4 核心数据流

目标主线（也是当前生产路径）`[V4]`：

```text
Formula 源串 / Factor 声明 / Streaming 更新 / Research 请求
                        │
                        ▼
        SemanticGraph（唯一的图模型 + 校验 + CSE + 分层调度）
                        │  lower()
                        ▼
        ComputePlan / FactorPlan（编译后的可复用计划）
                        │
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
      Full           Range          Streaming
        └───────────────┼────────────────┘
                        ▼
        UnifiedExecutor（经 RuntimeContext 记账）
                        │
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
     Kernels      BufferArena       StateArena
                        │
                        ▼
        Canonical Artifact（含 ArtifactHash）
```

- **公式**：`parse → AST → 优化 → plan`，`eval` 走生产 plan 路径；`eval_simd` 是 `eval` 的精确别名。
- **因子**：`FactorPlan` 经 `UnifiedRuntime` 执行，DAG 走与公式**同一张** `SemanticGraph`。
- **脏区（DirtyRange）**：只有整条依赖链能证明"增量 + 固定回看 + 时序安全"时才允许局部重算，否则回退全量——**correct first, fast second**。
- **流式**：逐 bar O(1) 更新，与批量路径共享同一内核语义（有 `batch == streaming` 门禁守护）。

## 1.5 SSOT（单一事实源）体系

| 事实源 | 位置 | 职责 | 消费者 |
| --- | --- | --- | --- |
| `FunctionSpec` / `registry` | `core/src/registry.rs` | 函数元数据、dependency shape、能力（range/streaming） | 公式、因子、绑定、文档生成 |
| `OperationSpec` | `core/src/operation.rs` | 跨前端统一的操作元数据 | 指标/公式/因子/绘图适配 |
| `schema` | `core/src/schema.rs` | 由注册表派生的机器可读 API schema | 绑定/文档生成 |
| `ffi_registry.json` | `docs/ffi_registry.json` | 绑定导出的函数清单 | 绑定对齐审计 |

设计规则：**任何函数的依赖形状必须从 `FunctionSpec` 读取，不得在别处二次推导**（已有门禁 `every_function_derives_its_dependency_shape_from_its_lookback` 守护 `[V4]`）。

## 1.6 模块地图（`core` 42 个顶层条目）

按层归类 `[实测]`：

- **L4 运行时**：`semantic_graph`、`compute`、`execution_plan`、`unified_runtime`、`unified_executor`、`runtime_context`、`state_arena`、`buffer_arena`、`runtime`、`performance`、`metrics`。
- **L3 语义**：`operation`、`registry`、`schema`、`data_contract`、`error`。
- **L2 数值内核**：`math/`、`indicators/`、`streaming/`、`patterns/`、`transforms/`、`factors/`、`features/`、`composite`、`factor_graph`、`factor_provider`、`factor_system`、`chan`、`chan_mtf`、`multi_period_resonance`、`returns`、`risk`、`calendar`、`traits`、`utils`。
- **L1 研究/扩展**：`polars_ext`（`finkit-polars`）、`talib_ffi`（`talib-c`）。
- **公式引擎**：`formula/`（32 个文件，含 parser/AST/optimizer/executor/bytecode/JIT/SIMD/模板/沙箱/pine）。

`core` 的三个"巨型模块"及其规模见 [2.1](#21-规模与热点)。

---

# 第二部分 开发细节与现状

## 2.1 规模与热点

各成员规模（Rust 源码）`[实测]`：

| 成员 | 文件数 | 行数 | 说明 |
| --- | ---: | ---: | --- |
| `core` | 471 | 244 038 | 引擎本体，占绝对多数 |
| `visualization` | 37 | 18 013 | 图表渲染 |
| `ffi/python-binding` | 11 | 17 350 | 最大的绑定 |
| `ffi/ffi-common` | 22 | 9 328 | 跨语言共享契约 |
| `factor-analysis` | 31 | 6 571 | 因子研究 |
| `ffi/node-binding` | 6 | 4 988 | |
| `wasm` | 4 | 4 896 | |
| `ffi/c-binding` | 4 | 3 609 | |
| `ffi/dotnet-binding` | 3 | 3 573 | |
| `ffi/go-binding` | 3 | 3 505 | |
| `ffi/java-binding` | 2 | 3 465 | |
| `cli` | 5 | 2 383 | |
| `ffi/ios-binding` | 2 | 607 | 收窄 ABI 子集 |
| `ffi/android-binding` | 2 | 266 | 收窄 ABI 子集 |

`core` 内部各子系统规模 `[实测]`：

| 子系统 | 行数 |
| --- | ---: |
| `formula/` | 50 851 |
| `indicators/` | 35 866 |
| `streaming/` | 35 511 |
| `math/` | 21 176 |
| `patterns/` | 13 124 |
| `features/` | 10 198 |
| `factors/` | 1 092 |
| `transforms/` | 745 |
| `polars_ext/` | 129 |

**超大文件（>2 500 行）** `[实测]`——它们是结构解耦的主要候选：

| 文件 | 行数 |
| --- | ---: |
| `formula/functions_legacy.rs` | 7 248 |
| `indicators/momentum.rs` | 5 928 |
| `math/simd_ops.rs` | 5 296 |
| `patterns/candlestick.rs` | 4 490 |
| `formula/unified_dispatch.rs` | 4 218 |
| `formula/engine.rs` | 3 922 |
| `formula/templates.rs` | 3 451 |
| `formula/simd.rs` | 3 320 |
| `operation.rs` | 3 288 |
| `formula/executor.rs` | 3 122 |
| `registry.rs` | 3 092 |
| `streaming/registry.rs` | 3 064 |
| `indicators/overlap.rs` | 3 003 |
| `indicators/cycle.rs` | 2 867 |
| `math/moving_avg.rs` | 2 759 |

## 2.2 子系统职责与入口

| 子系统 | 职责 | 公开入口 | 依赖 |
| --- | --- | --- | --- |
| `math/` | 移动平均、回归、统计、SIMD 内核（含 AVX2/AVX512/WASM 变体） | `math::moving_avg`、`math::linear`、`math::statistics` | 无（最底层） |
| `indicators/` | 150+ 批量指标，按类别分模块 | `indicators::{sma, rsi, macd, …}` | `math`、(部分) `overlap`/`volatility` |
| `streaming/` | 逐 bar O(1) 更新，`StreamingIndicator` trait | `streaming::overlap::sma::StreamingSma` 等 | `math`、`indicators` 语义 |
| `patterns/` | 60+ K 线形态 + 15+ 图表形态 + A 股扩展 | `patterns::{candlestick, chart, astock_kline}` | `common`（ATR/影线上下文） |
| `features/` | 特征工程：滞后、滚动统计、标准化、标签、组合、选择、导出 | `features::FeatureSet`、`FeatureMatrix` | `indicators`、`formula` |
| `formula/` | 表达式引擎：pest 语法 → AST → 优化 → plan 执行；模板、沙箱、pine | `formula::FormulaEngine`、`parse_formula` | `math`、`indicators` |
| `composite` / `factor_graph` / `factors` / `factor_system` / `factor_provider` | 声明式因子图与因子引擎 | `composite::CompositeEngine`、`factors::FactorContext` | `formula`、`indicators` |
| `chan` / `chan_mtf` | 缠论结构（含多周期） | `chan::*` | `math` |
| `risk` / `returns` / `performance` | 组合风险与收益率分析 | `risk::*`、`performance::*` | `math` |
| `calendar` | 交易日历、时段 | `calendar::*` | 无 |
| `factor-analysis` | 面板因子研究：prepare / IC / 验证 / 风险 / 报告 | `factor-analysis::*` | `core` |
| `visualization` | 场景、布局、渲染（SVG/PNG/HTML/WebGL）与交互 | `visualization::renderer`、`scene` | `core` |
| `ffi-common` | 跨语言共享契约：错误、JSON、所有权、panic 边界、金标 | 各绑定共用 | `core` |
| `cli` | 命令行：指标计算、形态检测、公式求值 | `finkit` 二进制 | `core`、`visualization` |
| `wasm` | 浏览器/Node 绑定 | `wasm-bindgen` 导出 | `core` |

## 2.3 工程体系

**测试**（`core/tests/`，62 个测试 target）`[实测]`——分层清晰，按用途可分为：

- **数值等价**：`golden_talib_tests`、`golden_reference`、`golden_example`、`accuracy_check`。
- **公式**：`formula_*` 系列（约 20 个，覆盖解析、优化、执行模式、差分、方言、模板、SSOT、控制流）。
- **跨路径一致性**：`rolling_volatility_consistency`、`runtime_convergence`、`fastpath_full_writer_contract`、`extrema_cached_path`。
- **契约与门禁**：`test_index_contract`、`indicator_api_surface`、`unified_operation_seam`、`factor_graph_contract`、`serde_roundtrip_tests`。
- **兼容方言**：`dzh_compat`、`tdx_compat`、`ths_compat`、`fox_compat`、`em_compat`、`pine_corpus_runner`。
- **性能/内存**：`performance_regression`、`memory_regression`、`long_run_stability`。
- **公开金标库**：`alpha158_parity`、`worldquant101_library`、`talib_coverage_matrix`。

**内务门禁**（22 个 `scripts/check_*.py` + `check_rustdoc.sh`）`[实测]`，本次抽验结果：

| 门禁 | 结果 |
| --- | --- |
| `check_orphan_modules.py` | OK（40 个公开模块） |
| `check_rust_source_reachability.py` | OK（481 个源文件全部可达） |
| `check_dead_code_allows.py` | OK（28 处抑制全部带理由） |
| `check_unbounded_loops.py` | OK（9 处无界循环均有终止论证） |
| `check_no_tracked_build_artifacts.py` | OK（1297 个 tracked 文件无构建产物） |
| `audit_binding_parity.py --check` | OK（8 个绑定对齐基线） |

**CI**（11 个 workflow）`[实测]`：`ci.yml` 含 15 个 job（workspace-check、version-consistency、binding-ssot、fmt、clippy、test、research-test、runtime-integration-tests、binding-unit-tests、package-contract、regression-gates、doc、audit、python-build-state、future-compat）；发布链另有 `python-wheels`、`talib-release-gate`、`multilang-cross-platform`、`multilang-release`、`release-installers`、`release-readiness`（同 SHA 聚合总门禁）等。

## 2.4 性能现状

`docs/BENCHMARK_REPORT.md`（Criterion 自动生成）`[实测]`：

- **90 项配对**：78 项 ✅（领先或持平）、12 项 ⚠️、**0 项 ❌**。
- 12 项 ⚠️ 全部在 **0.91–0.99** 窄带，绝对差距均 ≤ 4 µs：`min_30` 0.91、`ultosc` 0.93、`var_20` 0.93、`aroonosc` 0.94、`linreg_angle`/`max_30` 0.97、`ad`/`aroon`/`trima` 0.98、`acos`/`ln`/`minus_di` 0.99。
- 显著领先项示例：`ceil`/`floor` 15.4×、`wclprice` 9.3×、`t3` 9.0×、`tema` 4.4×、`kama` 3.9×。

**关键方法学结论** `[V4]`：§C 公开路径（API 包装层）会**系统性低估**分配次数多的实现；内核层已被确认与该 C 库持平，剩余窄带属"包装/API 分配形状"而非"算法量级"。因此**继续单点攻坚内核的边际收益低**，杠杆在公开 API 的分配形状——而这项在连续多轮的 ✅/⚠️ 抖动中已被证明小于本机跨运行方差。

## 2.5 feature gate 体系

`core` 的 feature 分层 `[实测]`：

- **默认集**：`std`、`formula`、`serde`、`tracing`、`metrics`、`formula-jit`、`formula-simd`、`indicators-all`。
- **指标类别树**：`indicators-{overlap,momentum,volume,volatility,cycle,statistics,price-transform,patterns,market}`，由 `indicators-all` 汇聚，用于增量裁剪二进制。
- **冻结面**：`formula-jit`、`formula-simd`（保留、不推荐、不再增长）。
- **no_std 脚手架**：`no_std` + `math` 子集。
- **可选集成**：`finkit-polars`、`talib-c`、`rayon`、`tracing`、`metrics-prometheus`。

> 注意：本次核验发现 **`circuit-breaker` 与 `precision-f32` 两个 feature 在 `core/src` 中零引用** `[实测]`——属悬空 feature（见 [4.4](#44-工程与文档治理)）。

---

# 第三部分 基线核验：V4 遗留项在 HEAD 的状态

V4 的结论跨越 27 轮，**不能照抄**。下表是本次在 HEAD 上对 V4 遗留项的逐项核验结果：

| # | V4 遗留项 | HEAD 状态 | 证据（`[实测]`） |
| --- | --- | --- | --- |
| 1 | 多语言绑定补齐（Go/.NET/Py/Node/Java） | ✅ **已解决** | `audit_binding_parity.py --check`：c/python/node/go/java/dotnet 均 78/78 |
| 2 | iOS / Android 覆盖 15/78 | ⏸ **有意保留** | 审计脚本标记 `deferred`，属移动端收窄 ABI 子集 |
| 3 | 孤儿模块 / 源码可达性 / 无界循环 / dead_code 门禁 | ✅ **已解决且全绿** | 四个 `check_*.py` 均 exit 0 |
| 4 | `core/src/circuit_breaker.rs` 未进 module graph | ✅ **已删除** | 文件不存在；`check_rust_source_reachability` 481 文件全可达 |
| 5 | `core/target/criterion` 构建产物被 tracked | ✅ **已清理** | `git ls-files \| grep criterion` = 0；build-artifact 门禁绿 |
| 6 | Python 构建原地改写 tracked 源码 | ✅ **已解决** | `prepare_python_hot_bindings.py` 已重构为：先验证 → 生成瞬态 overlay（`target/`）→ 重新生成 `generated.rs` → **证明手写 tracked 源码未被改写**。实跑输出 `handwritten-source=unmodified`，运行后 `git status` 无源码变化 |
| 7 | LINREG 递推在仓内七份、方向不一致 | ◐ **方向已统一，七份实现仍在** | 方向已收敛（§44.5），但 `math/linear.rs`(4) + `math/simd_ops.rs` + `formula/simd.rs` 仍是多份手抄 |
| 8 | ADX 两份手抄在 NaN 上给不同答案 | ❌ **仍成立** | `adx_into` 同时存在于 `indicators/momentum.rs:4912` 与 `math/kernels/compat.rs:481`；`true_range_fast` 与 `utils::true_range` 并存 |
| 9 | 除法守卫用固定带宽而非精确零（52 处入队） | ❌ **大部分仍成立** | `indicators/momentum.rs`、`indicators/volume_ext.rs`、`math/moving_avg.rs` 仍大量使用 `> 1e-15` 守卫 |
| 10 | 12 项 ⚠️ 指标窄带 | ❌ **仍成立** | `BENCHMARK_REPORT.md` 仍为 12 ⚠️ / 78 ✅ / 0 ❌ |
| 11 | kernel fusion | ⏸ **未实现（有意）** | V4 §24a 明确"不谎报"，待有具体热点再做 |
| 12 | doctest 本机 Windows 命名管道耗尽 | ⏸ **环境问题** | `[V4]`，与代码无关，需换机/CI 确认 |

**本次基线核验的新发现**（V4 未记录）：

| # | 发现 | 证据（`[实测]`） |
| --- | --- | --- |
| N1 | `circuit-breaker` feature 悬空（已于 第二十八轮 移除：`core/Cargo.toml` 定义行 + `core/README.md`/`docs/usage.md` 提及一并清理） | `grep -rn "circuit-breaker" core/src` 无匹配 |
| N2 | `precision-f32` feature 悬空（已于 第二十八轮 移除：`core/Cargo.toml` 定义行 + `core/README.md`/`docs/usage.md` 提及一并清理） | `grep -rn "precision-f32" core/src` 无匹配 |
| N3 | 架构文档漂移：`docs/architecture/formula-engine.md` 仍描述 JIT / "hot loop ≥ 1M → JIT" / `eval_simd` 为真实路径，与"二者已冻结"的契约矛盾 | 文档正文 vs `lib.rs` / `Cargo.toml` 的 FROZEN 注释 |
| N4 | **绑定 feature 声明与实际编译态不符**：python-binding 声明 `default-features = false`，但因 `factor-analysis` / `visualization` / `ffi-common` 均以默认 features 依赖 `finkit`，feature unification 把 core 的 `default`（含冻结的 `formula-jit`/`formula-simd`）重新拉回 | `cargo tree -p finkit-python -e features -i finkit` 显示 `finkit feature "default"`、`"formula-jit"`、`"formula-simd"` 均被启用；另已实证：收紧三个中间 crate 后 `cargo check -p finkit-python` 报 `no method named eval_jit/eval_simd` |
| N5 | **各绑定 core feature 组合不一致**：c/dotnet/java 用全默认；python/node 用 `std+serde+indicators-all`；go 另加 `formula`；wasm 另加 `tracing`；ios/android 仅 `indicators-all` | 各 `ffi/*/Cargo.toml` 的 `finkit` 依赖行 |
| N6 | **无门禁守护 feature 一致性或"发布构建态 == workspace 构建态"** | 22 个 `check_*.py` 均不检查 feature 组合 |
| N7 | Python 包目录残留旧构建产物：`alpha_ta.cp311/cp313-win_amd64.pyd`（共 ~12.9 MB）位于 `python-packages = ["finkit"]` 的打包范围内 | `ls ffi/python-binding/finkit/*.pyd`；被 `.gitignore` 忽略但仍在工作区 |
| N8 | dotnet 工作区残留 ~110 个 `.dll` / 12 MB（含旧名 `AlphaTA.dll`） | `find ffi/dotnet-binding -name '*.dll' \| wc -l` = 110 |

**核验方法**：只读命令 + 内务门禁脚本；未修改任何代码。

---

# 第四部分 分层问题诊断

诊断按 **影响面 × 修复成本** 排序。每条给出可复核的证据位置，避免"感觉问题"。

## 4.1 语义一致性（最高优先级）

**问题**：同一指标存在多份手抄实现，形成"一处政策、多处实现"的结构。改一处而漏一处，就产生静默的数值漂移——而数值语义一致正是 Finkit 的核心产品承诺。

**证据** `[实测]`：

| 现象 | 位置 |
| --- | --- |
| ADX 同一递推两份，NaN 语义不同 | `indicators/momentum.rs:4912` 的 `adx_into` vs `math/kernels/compat.rs:481` 的 `adx_into` |
| true range 两种语义并存 | `indicators/momentum.rs:4991` 的局部 `true_range_fast`（NaN 传播）vs `utils::true_range`（`f64::max` 忽略 NaN） |
| LINREG 族多份手抄 | `math/linear.rs`（4 个公开函数）+ `math/simd_ops.rs`（AVX2 + scalar 回退）+ `formula/simd.rs` |
| 除法守卫用固定带宽 | `indicators/momentum.rs`（≥18 处 `> 1e-15`）、`indicators/volume_ext.rs`（≥10 处）、`math/moving_avg.rs`（≥6 处） |

**为什么最高优先级**：
1. 它是**正确性**风险，不只是可读性——`adx_into` 的两份在含 NaN 的 OHLC 上给出不同答案 `[V4]`。
2. 它使每次内核优化必须"同步多个副本"，是长期维护成本的主要来源。
3. 产品边界（多语言共享同一数值语义）在结构上依赖它。

**方向**：每个指标一个 canonical kernel；batch / streaming / SIMD / FFI 都**复用**同一内核，而不是各自手抄；守卫政策统一为"抄 TA-Lib 的精确语义（`> 0.0` / `== 0.0`），不自行发明带宽" `[V4]`。

## 4.2 结构耦合

**问题 A：超大文件集中。** 见 [2.1](#21-规模与热点)。`formula/functions_legacy.rs` 7248 行、`indicators/momentum.rs` 5928 行、`math/simd_ops.rs` 5296 行。巨型文件本身不改行为，但抬高修改风险与评审成本，并掩盖 4.1 的多份实现（它们正是"藏"在这些文件里）。

**问题 B：公式函数表三层。** `formula/functions.rs` 是路由表，从 `functions_legacy.rs`（兼容目录）起步，用 `functions_talib_081.rs` 覆盖 TA-Lib 敏感名。这是**刻意的分层**（注释明确警告不要一次性高风险重写），但**边界尚未固化**——没有门禁断言"哪些名字必须由哪一层提供"，新增/覆盖的归属靠人记。`[实测]`（`formula/mod.rs` 注释）+ `[V4]`。

## 4.3 运行时

**现状**：`FunctionSpec` SSOT、`SemanticGraph`、`ComputePlan`、`UnifiedRuntime`、`RuntimeContext` 均已落地，且每条都有对应门禁持有 `[V4]`（§24a Batch 3）。

**剩余问题**：

- **kernel fusion 未实现**——这是**有意的取舍**，不是缺陷：在"不重复计算、不重复分配"尚未榨干前做融合，只会得到更难维护且无人能量化收益的 kernel `[V4]`。
- **公开 API 的分配形状**是当前性能的主要杠杆（12 项窄带），但改造属 API 层，需独立评估 `[V4]`。
- **FunctionSpec 是否仍有二次推导残留**需在全注册表范围复核 `[待核验]`。

## 4.4 工程与文档治理

| 问题 | 证据 | 性质 |
| --- | --- | --- |
| `circuit-breaker` feature 悬空（已于 第二十八轮 移除） | `core/Cargo.toml:215` 定义，`core/src` 零引用 `[实测]` | 卫生 |
| `precision-f32` feature 悬空（已于 第二十八轮 移除） | `core/Cargo.toml` 定义，`core/src` 零引用 `[实测]` | 卫生 |
| 架构文档漂移 | `docs/architecture/formula-engine.md` 描述已冻结的 JIT/SIMD 热路径 `[实测]` | 文档 |
| 绑定 feature 一致性 | 见 [4.6](#46-python-多层配置与多语言编译模块) `[实测]` | 构建可信度 |
| 警告债务 | V4 §5.6 记录约 5516 条 clippy warning `[V4]` | 卫生 |

**为什么"Python 构建原地变换"被列为结构问题而非小事**：workspace 的普通编译绿，**不能证明变换后的源码可编译**——构建前把源码改写成 `u16`/`"var"` 之类的形态，是发布链 P0 事故的根源（V4 §8）`[V4]`。正确方向是"SSOT → 确定性生成器 → 生成源码"，而非"构建时打补丁"。

## 4.5 交付层

- **移动端（iOS/Android）15/78**：属**独立路线图项**，不是"少写封装"——它们暴露的是一个刻意收窄的 ABI 子集。若要补齐，应连同移动端二进制体积预算一起评估 `[V4]`。
- **12 项 ⚠️ 指标**：见 [2.4](#24-性能现状)，属"包装/API 开销"而非算法落后。

## 4.6 Python 多层配置与多语言编译模块

### 4.6.1 Python 绑定的配置层次

Python 交付面横跨 **5 层配置**，任一层与其它层不一致都会在发布时才暴露：

| 层 | 位置 | 内容 |
| --- | --- | --- |
| ① Cargo features | `ffi/python-binding/Cargo.toml` | `default = ["formula", "rayon"]`；core 依赖 `default-features = false, features = ["std", "serde", "indicators-all"]` |
| ② maturin 全局 | `pyproject.toml` `[tool.maturin]` | `features = ["pyo3/extension-module", "abi3"]`、`module-name`、`python-packages`、`strip` |
| ③ maturin 逐平台 | `pyproject.toml` `[tool.maturin.target.*]` | **6 个平台块逐字重复同一 features 列表** |
| ④ 构建期脚本 | `scripts/prepare_python_hot_bindings.py` | 验证 → 瞬态 overlay → 生成 `generated.rs` → 证明手写源码未改（**已重构，良好**） |
| ⑤ Python 包 | `ffi/python-binding/finkit/` | `__init__.py`(32 KB 手写) + `__init__.pyi`(46 KB) + `accessor.py`/`alphalens.py`/`exceptions.py` |

### 4.6.2 核心问题：feature 声明与实际编译态不符（高优先级）

python-binding 在 Cargo.toml 里写了 `default-features = false`，意图是只编入 `std + serde + indicators-all`。**实际并非如此。**

`cargo tree -p finkit-python -e features -i finkit` 显示 core 的 `default`、`formula-jit`、`formula-simd`、`tracing`、`metrics` **全部被启用** `[实测]`。

**根因**：`factor-analysis`、`visualization`、`ffi-common` 三个中间 crate 都以**默认 features** 依赖 `finkit`（`finkit = { path = "../core" }`，无 `default-features = false`）。Cargo 的 feature unification 会把它们打开的 `default` 合并回同一次构建，**覆盖** python-binding 的 `default-features = false`。

```text
python-binding (default-features=false) ─┐
factor-analysis  ─ finkit (default) ─────┼─▶ feature unification ─▶ finkit {
visualization    ─ finkit (default) ─────┤      default, formula-jit,
ffi-common       ─ finkit (default) ─────┘      formula-simd, tracing, metrics }
```

**两个直接后果**：

1. **声明失真**：任何基于 "python wheel 只含精简 feature 集" 的二进制体积/裁剪判断都是错的——实际编入了全部 default（含冻结的 `formula-jit`/`formula-simd`）。
2. **可编译性依赖无关 crate 的隐式契约**：`ffi/python-binding/src/lib.rs` 的 `formula_eval_jit`（:2507）与 `formula_eval_simd`（:2575）只被 `#[cfg(feature = "formula")]` 门控，却分别调用了 core 中 `#[cfg(feature = "formula-jit")]` 的 `eval_jit` 和 `#[cfg(feature = "formula-simd")]` 的 `eval_simd`。它们**能编译，仅仅因为**上述三个中间 crate 恰好打开了这两个冻结面。

**这不是推测，已实证** `[实测]`：把 `factor-analysis` / `visualization` / `ffi-common` 三者的 `finkit` 依赖改为 `default-features = false, features = ["std", "formula", "indicators-all"]`（tree-shaking 的正常方向）后，`cargo check -p finkit-python` 立即失败：

```text
error[E0599]: no method named `eval_jit`  found for struct `FormulaEngine`
             --> ffi/python-binding/src/lib.rs:2507
error[E0599]: no method named `eval_simd` found for struct `FormulaEngine`
             --> ffi/python-binding/src/lib.rs:2575
```

即：**python-wheel 能否编译，取决于三个与它无关的中间 crate 的 feature 声明**——一个无门禁守护的隐式契约。收紧它们（正常的精简动作）会连带打断 Python 发布。

### 4.6.3 其他问题

| 问题 | 说明 |
| --- | --- |
| maturin 逐平台配置重复 | 6 个 `[tool.maturin.target.*]` 块逐字重写同一 features，属 DRY 违反，改一处漏一处 |
| 各绑定 feature 组合不一致 | c/dotnet/java 全默认；python/node `std+serde+indicators-all`；go 另加 `formula`；wasm 另加 `tracing`；ios/android 仅 `indicators-all`。不同语言交付面编入不同组合，行为/体积可能分歧 |
| 无 feature 一致性门禁 | 22 个 `check_*.py` 不检查 feature 组合，也不校验"发布构建态 == workspace 构建态" |
| 工作区产物残留 | `finkit/*.pyd`（~12.9 MB，含旧名 `alpha_ta`）在打包目录内；dotnet 工作区 ~110 个 `.dll` |

### 4.6.4 为什么必须加强

这不是洁癖问题：**它使 "workspace 编译绿" 不再能证明 "发布产物可编译"**。对于一个对外承诺多语言统一语义的引擎，构建态的确定性本身就是产品的一部分。

---

## 4.7 核心逻辑深度审查（三轮，2026-10-07）

本节是对核心逻辑的**专项深挖**，按"每轮一个维度"推进：第一轮数值/算法、第二轮运行时/状态/资源、第三轮契约/边界/发布。每条均给出可复核的文件/行号 `[实测]`；无法在本机端到端复现的标注 `[待核验]`。

### 4.7.1 第一轮 · 数值/算法

**R1-1　NaN 三态不一致（正确性，最高优先级）**

同一个仓库对"缺失值（NaN）"存在三种互斥立场，且**同一指标族内部就不一致**：

| 立场 | 位置 | 行为 |
| --- | --- | --- |
| **传播**（声明策略） | `formula/compat.rs` 的 `null_policy: "nan-propagating"`；`scripts/check_nan_unsafe_ordering.py` 文档串明确"NaN 是真实行情到达的常态，应传播" | NaN 流入 → NaN 流出 |
| **拒绝** | `indicators/momentum.rs` 的 `rsi`/`macd`/`apo`/`ppo_with_ma_type`/`macd_into`/`macd_fast_into`/`macd_line_into` 等入口 `position(|v| !v.is_finite())` → `Err(InvalidParameter)`；另见 `overlap.rs`/`math/moving_avg.rs`/`math/typed_moving_avg.rs` | 含非有限值 → 直接报错 |
| **吞噬** | `utils::true_range`、`math/simd_ops.rs` 的 `true_range_scalar`/`true_range_avx2`，以及 `indicators/talib_ext.rs:307`、`indicators/volatility.rs:33`、`indicators/volatility_ext.rs:470` 等手抄副本 | `f64::max` / `_mm256_max_pd` 忽略 NaN → 返回**有限值** |

分歧可复现：`high=NaN, low=10, prev_close=9` 时，`f64::max` 版返回 `1.0`（有限），而 ADX 使用的 `true_range_fast`（`if x > range` 比较式，`momentum.rs:4991`）返回 `NaN`。即 **ATR/TRANGE 族吞掉缺失、ADX 族传播缺失**，二者对同一份含 NaN 的 OHLC 给出不同答案。

可达性：`indicators/volatility.rs::validate_ohlc`、`patterns/common.rs::validate_ohlcv` **只校验长度、不校验有限性**，NaN 能进入 ATR/TRANGE 计算；而全树只有 **37 处**入口做非有限拒绝（仅分布在 4 个文件），绝大多数指标入口不拒绝。

> **方向**：先定一条**唯一契约**（建议与 `nan-propagating` 声明一致 = 传播），再据此统一 `true_range` 的两份实现与全部手抄副本；并把"入口是否拒绝 NaN"收敛为统一策略，不做混合。

**R1-2　零判定带宽是自造的，且偏离 TA-Lib**

`momentum.rs`（`:42` STOCH `denom > 1e-15`、`:877` DX `sum.abs() > 1e-15`、`:958` `di_pair_from_state` 的 `tr.abs() <= 1e-15`）、`volume_ext.rs`、`math/moving_avg.rs` 等统一使用 `1e-15` 作为"视作零"的带宽（全树 `1e-15`/`1e-12` 守卫 **547 处**，非测试）。

TA-Lib 的实际契约是 `TA_IS_ZERO(v) = ((v) > -1e-8) && ((v) < 1e-8)`——**带宽 1e-8**（`ta_utility.h`；并经 TA-Lib issue #157 与 release notes #390/#395 佐证其在 1e-8 边界上有明确数值行为）。`[实测]`（常量比对，公开来源引证）

后果：当分母落在 `(1e-15, 1e-8)` 区间时，Finkit 会相除得到有限比值，TA-Lib 会判零走 fallback，两者**输出分歧**；且各指标的 fallback 取值（STOCH→`50.0`、DX→`0.0`、DI→`(0.0, 0.0)`）没有逐指标契约记录。现有对拍数据未覆盖该区间，故门禁不会报警。

> **方向**：把零判定带宽收敛为**一个具名常量**并标注其 TA-Lib 出处；为每个指标显式记录 fallback 契约；补 `(1e-15,1e-8)` 区间的对拍金标。

### 4.7.2 第二轮 · 运行时/状态/资源

**R2-1　repaint 代码生成宏零引用，10 份手写快照并存**

- 两个意在消除重复的 `#[macro_export]` 宏——`impl_repaint!`（`streaming/macros.rs`）与 `impl_compute_bar!`（`streaming/repaint.rs`）——在全仓（含 `core/tests`、`ffi`、`wasm`）**零调用**。
- 实际由 11 个流式指标各自手写 `compute_bar`，并各自定义私有的 `struct SnapshotState`（**10 处**定义），保存/恢复逻辑逐字重复。
- `streaming/repaint.rs`（95 行）**仅**承载那个未被使用的宏 → 对 `check_dead_code_allows` **不可见**（宏不计入 dead_code）。

> **方向**：二选一——把宏收敛为唯一实现并迁移调用方，或删除未用宏；`streaming/repaint.rs` 随之评估可删。

**R2-2　运行时缓存策略不统一，且部分无界（资源）**

- `runtime_context.rs::ArtifactCache` 的文档声称是"the one cache the runtime context owns"、取代各层自有缓存；但实际 **只有 `unified_runtime` 在用**（`ArtifactKey` 全仓仅 `unified_runtime.rs` 两处构造，namespace 仅 `FINPLAN0`/`FINPLAN1`），SSOT 声明未兑现。
- formula/composite 层仍各自持有缓存：`formula/compiler.rs::FormulaCache`（`LruCache`）、`formula/engine.rs::FormulaPlanCache`（`HashMap`）、`semantic_plan_cache`（`RefCell<HashMap<String,_>>`）、`composite.rs::CompositeCacheEntry`。
- 其中 `FormulaPlanCache::insert` 无驱逐路径，`engine.rs:95` 注释**自述为 "Unbounded"**。→ 长生命周期/多租户场景下按不同公式**无限增长**（内存风险）；`RefCell` 缓存也使引擎不具备跨线程共享性。

> **方向**：要么让各层真正收敛到 `ArtifactCache`（兑现声明），要么把该文档改为"仅 unified runtime 使用"，并给 formula/composite 缓存补上有界驱逐。

### 4.7.3 第三轮 · 契约/边界/发布

**R3-1　递归深度防护是"事后检查"，默认关闭，且缺少源长度上限（崩溃面）**

- 解析器只对**括号嵌套**设限（`formula/parser.rs:18` 为 256；`formula/pine/parser.rs:207` 为 128，另加 1 MiB 源上限），但 `parse_expression` 对 `a+b+c+...` 构建的是**左嵌套** AST（`parser.rs:668-683`），其深度 = 项数，而解析本身是迭代的（不消耗解析器栈），因此括号深度限制**拦不住**扁平长链。
- lowering 是递归的（`compute_ir.rs:279` `lower` → `lower_inner` → `self.lower(child)`），深度随 AST 深度线性增长；`lower()` 只**记录** `depth`/`max_depth`，**不做**校验。其注释（`compute_ir.rs:276-277`）声称"这里是必须施加限制的地方"，但代码并未施加。
- 真正的检查 `sandbox_check_depth` 在 `engine.rs:796` 调用，此时 plan 已 lowering 完成——**栈已在 lowering 期间被消耗**，检查跑不到就会先溢出。
- `ExecSandboxConfig::default()` 三项全 `None`（unlimited，`sandbox.rs:10`），即**默认不检查**。
- 失败形态是 **stack overflow = 进程 abort**；仓库自己的文档明确"abort no FFI guard catches"（`formula/parser.rs:15`、`ffi/ffi-common/src/panic.rs`）——`catch_unwind` 拦不住。
- 可达性：需要一个不含括号、但项数极大的表达式（如 `1+1+...`）。**未在本机端到端复现** `[待核验]`；但"检查在 lowering 之后 + 默认 unlimited + 无源长度上限"三条静态事实已成立，契合仓库自身对该崩溃类的重视。

> **方向**：在 `lower()` 内**边下推边校验**（借用已有的 `pending_error` 机制，超限即停），给原生解析器补源长度上限（对齐 Pine 的 1 MiB），并把默认 sandbox 从 unlimited 改为有记录的安全默认。

---

# 第五部分 分层重构路线

原则：**每批独立可交付、可回滚；每批都必须有门禁持有结果**——没有门禁的"已重构"等于没重构，下一轮就会悄悄回退 `[V4]`。

## Batch 0 — 基线核验与固化（只读，不改代码）

| 项 | 内容 |
| --- | --- |
| 目标 | 把 [第三部分](#第三部分-基线核验v4-遗留项在-head-的状态) 的核验结果固化为**可复现**清单：每条"仍成立"附一条可执行命令 |
| 落点 | 本文档附录；不产生代码改动 |
| 阻塞门禁 | 无 |
| 验收 | 每项都能由一条只读命令复现；跨平台/工具链依赖项显式标注 `[待核验]` |
| 回滚边界 | 无（只读） |

## Batch 1 — 语义统一（正确性优先）

| 项 | 内容 |
| --- | --- |
| 目标 | 每个指标一个 canonical kernel；batch / streaming / SIMD / FFI 全部复用同一内核 |
| 落点（按优先级） | ① `adx_into` 两份合并（`indicators/momentum.rs` 与 `math/kernels/compat.rs`），方向为**向 TA-Lib 靠拢**；② `true_range` 语义统一（消除 `true_range_fast` 与 `utils::true_range` 的 NaN 分歧）；③ LINREG 族多份手抄收敛；④ 除法守卫政策统一（`> 1e-15` → TA-Lib 精确语义） |
| 阻塞门禁 | `batch == streaming` 一致性门禁；**含 NaN OHLC 的新金标测试**（合并方向会改变公开数值，必须先立金标再改代码）；`formula_plan_differential`、`formula_differential_tests` |
| 验收 | 每个指标只剩一个内核；跨路径（batch/streaming/range/SIMD）逐位或 `1e-12` 内一致；公开数值变更全部有金标覆盖 |
| 回滚边界 | **逐指标独立提交**，单个指标可单独回退；不得一次性重写整个族 |

> **为什么先立金标再改代码**：ADX 的合并会改变 `utils::true_range` 在 ADX 上的公开行为——没有 NaN 金标就改，等于用"看不见的漂移"换"看不见的修复"。

> **措辞修正（2026-10-07）**：本批④原写作"TA-Lib 精确语义（`> 0.0` / `== 0.0`）"。经本轮核对，TA-Lib 实际用的是 `TA_IS_ZERO` 的 **1e-8 带宽**，并非精确零。正确方向是把守卫带宽对齐其 TA-Lib 出处并逐指标记录 fallback，详见 [4.7.1 R1-2](#471-第一轮--数值算法)。

## Batch 2 — 结构解耦

| 项 | 内容 |
| --- | --- |
| 目标 | 按职责拆分超大文件；固化公式函数表三层边界 |
| 落点 | `formula/functions_legacy.rs`、`indicators/momentum.rs`、`math/simd_ops.rs`、`formula/unified_dispatch.rs` 等；`formula` 的 router/legacy/talib_081 分层加**门禁断言** |
| 阻塞门禁 | `check_orphan_modules`、`check_rust_source_reachability`、`indicator_api_surface`、`formula_registry_signature` |
| 验收 | **零公开 API 变更**（`indicator_api_surface` 不变）；拆分为按类别的子模块；函数表每层的归属有断言 |
| 回滚边界 | 纯移动代码，可整体回退；不与 Batch 1 混提 |

## Batch 3 — 运行时收敛

| 项 | 内容 |
| --- | --- |
| 目标 | 完成 `FunctionSpec` SSOT 复核；按 profiling 决定 kernel fusion |
| 落点 | `registry.rs` 的 dependency shape 单一来源；`runtime_context.rs` 记账；fusion 仅在 profiling 指出明确热点时立项 |
| 阻塞门禁 | `every_function_derives_its_dependency_shape_from_its_lookback`、`registry_dependency_matches_the_plan_node_capability`、`range_execution_is_refused_without_a_proven_lookback` |
| 验收 | 全注册表范围无二次推导；fusion 有可量化收益才合并 |
| 回滚边界 | 按子项独立 |

### Batch 3 补充落点（第二轮审查，2026-10-07）

- **缓存策略统一（对应 [4.7.2 R2-2](#472-第二轮--运行时状态资源)）**：兑现或修正 `ArtifactCache` 的"唯一缓存"声明；给 `FormulaPlanCache` / `semantic_plan_cache` 补有界驱逐（或迁移到 `ArtifactCache`）；消除对外层 `RefCell` 缓存的跨线程依赖。
- **repaint 代码生成收敛（对应 [4.7.2 R2-1](#472-第二轮--运行时状态资源)）**：把两个零引用的 `#[macro_export]` 宏（`impl_repaint!` / `impl_compute_bar!`）二选一收敛，并迁移 10 处手写 `SnapshotState`；`streaming/repaint.rs` 随之评估是否可删。
- 阻塞门禁建议：新增"缓存有界性"门禁（断言每个缓存存在容量/驱逐路径）与"未使用导出宏"检查。

## Batch 4 — 工程与文档治理

| 项 | 内容 |
| --- | --- |
| 目标 | 清除卫生债；建立文档防漂移；收敛 Python 构建 |
| 落点 | ① ~~删除悬空 feature `circuit-breaker` / `precision-f32`~~ → 已于 第二十八轮 移除（定义行 + `core/README.md`/`docs/usage.md` 提及一并清理）；② 修正 `docs/architecture/formula-engine.md` 与冻结契约的矛盾；③ 建立**警告预算**（correctness-first，不要求清零）；④ 清理打包目录内的残留构建产物（`finkit/*.pyd`、dotnet 工作区 `.dll`） |
| 阻塞门禁 | `check_docs_links`、`check_workflow_path_coverage`、`python-build-state`、`future-compat` |
| 验收 | 悬空 feature 归零；架构文档与 `Cargo.toml` FROZEN 注释一致；打包目录内无残留产物 |
| 回滚边界 | 文档与 feature 清理可独立回退 |

## Batch 5 — 多语言编译态确定性

| 项 | 内容 |
| --- | --- |
| 目标 | 让"声明"与"实际编译态"一致，并让"发布构建态 == workspace 构建态"成为**可验证的契约** |
| 落点 | ① 给 `factor-analysis` / `visualization` / `ffi-common` 的 `finkit` 依赖补 `default-features = false` + 显式所需 features，消除 feature unification 对绑定声明的反向覆盖；② 把各绑定声明的 core feature 组合收敛为**单一契约表**；③ 修正 `formula_eval_simd` 的 feature 门控，使其依赖显式声明而非间接巧合；④ 新增门禁：校验每个绑定 `cargo tree` 的实际 feature == 声明，且单独构建该绑定与 workspace 构建结果一致；⑤ 消除 maturin 6 个逐平台块的重复 |
| 阻塞门禁 | `python-wheels`、`multilang-cross-platform`、`binding-ssot`、新增的 feature 一致性门禁 |
| 验收 | `cargo tree -p <binding>` 实际 feature 与声明一致；单独 `cargo check -p finkit-python` 与 workspace 结果一致；把任一中间 crate 切成 `default-features = false` 后，绑定仍能**独立**编译（证明不再依赖巧合） |
| 回滚边界 | ① 是跨 crate 依赖边界改动，须单独提交并逐一观察所有绑定；⑤ 纯配置，可独立回退 |

> **为什么单独成批**：它不属于"文档卫生"，而是发布链的**可编译性契约**。它影响所有语言交付面，且改动一处会牵动其他绑定，因此不能塞进 Batch 4。

## Batch 6 — 输入边界与崩溃面

| 项 | 内容 |
| --- | --- |
| 目标 | 消除"不可控输入 → 进程 abort"的路径；把输入校验策略收敛为单一契约 |
| 落点（对应 [4.7.3 R3-1](#473-第三轮--契约边界发布) 与 [4.7.1 R1-1](#471-第一轮--数值算法)） | ① 在 `compute_ir.rs::lower()` 内**边下推边校验**深度（借用已有 `pending_error`，超限即停），替换当前"lowering 完成后才 check"的事后校验；② 给原生公式解析器补源长度上限（对齐 Pine 的 1 MiB），使扁平超长表达式无法绕过括号深度限制；③ 把 `ExecSandboxConfig` 默认从 unlimited 改为有记录的安全默认（或在文档中显式声明"默认无限额"为契约）；④ 收敛入口非有限校验，使"拒绝/传播"二选一并与 `nan-propagating` 声明统一 |
| 阻塞门禁 | 新增崩溃面门禁（对深度/规模超限输入断言返回 `Err` 而非 abort）；`check_nan_unsafe_ordering`；各绑定的 panic-guard 测试 |
| 验收 | 任意超限输入返回 typed error；FFI 边界永不 abort；输入校验三态归一 |
| 回滚边界 | 逐项独立；源长度上限须与 parser 相关门禁一起提交 |

> **为何单独成批**：这是**崩溃面**（abort 不可被 FFI 的 `catch_unwind` 捕获），与"语义统一""结构解耦"不是同一类改动；它直接决定"用户可控输入能否放倒进程"。

## 批次依赖关系

```text
Batch 0（只读基线）
   │
   ├──▶ Batch 1（语义统一）—— 正确性，最高优先
   │         │
   │         └──▶ Batch 2（结构解耦）—— 建议在 1 之后，避免重复移动
   │
   ├──▶ Batch 3（运行时收敛）—— 与 1/2 弱耦合，可并行
   │
   ├──▶ Batch 4（工程与文档治理）—— 独立，随时可做
   │
   ├──▶ Batch 5（多语言编译态确定性）—— 独立，但优先级仅次于 Batch 1
   │
   └──▶ Batch 6（输入边界与崩溃面）—— 独立，安全相关，可与 Batch 1 并行
```

**不建议**跳过 Batch 0 直接改代码：本文第三部分已证明 V4 的清单有相当比例已过期，按旧清单开发会修不存在的问题。

---

# 第六部分 追溯与关系

## 6.1 与历史文档的关系

| 文档 | 角色 |
| --- | --- |
| 本文（V5） | **当前唯一执行基线**：稳定的架构地图 + 现状基线 + 诊断 + 分批路线 |
| `FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` | **历史证据来源**：27 轮审计的原始记录与方法学。仍开条目已在上文逐项核验，不再单独作为待办清单。已于 2026-10-10 从工作树删除，保留在 Git 历史 |
| `refactor-plan-2026-09-21.md` | **产品边界与定调的来源**：不做回测/选股、JIT/`eval_simd` 冻结、公式只留 tree+plan。其 Phase 状态已过期。已于 2026-10-10 从工作树删除，保留在 Git 历史 |
| `docs/architecture/*.md` | 架构细节参考；`formula-engine.md` 存在漂移，待 Batch 4 修正 |
| `docs/BENCHMARK_REPORT.md` | 性能基线（自动生成），只对记录 commit/CPU/工具链有效 |

## 6.2 结论来源分级

| 分级 | 含义 |
| --- | --- |
| `[实测]` | 本次在 HEAD 上由只读命令/脚本直接得到，可复现 |
| `[V4]` | 引自 V4，本次未在 HEAD 上完整复现 |
| `[待核验]` | 需要专门方法学（TA-Lib C 工具链）或跨平台环境（macOS/AArch64）才能确认 |

## 6.3 开放问题

1. **性能窄带（12 项 0.91–0.99）的共同成因**：V4 多轮探查未能定位，且已发现"公开路径分配形状"是系统性混淆因素。**不预设配方**，若要继续应作为独立方法学立项。
2. **FunctionSpec 是否仍有二次推导残留**：需在全注册表范围复核 `[待核验]`。
3. **跨平台 CI 结果**：本文所有核验均在本机 Windows 完成，macOS/AArch64 与 doctest 绿度 `[待核验]`。
4. **警告债务的准确规模**：V4 记录约 5516 条 `[V4]`，本次未重跑全量 clippy 复现。
5. **各绑定应有的 canonical feature 组合**：本文指出组合不一致，但"正确的那一组合"需结合各语言导出面与二进制体积预算确定 `[待核验]`。
6. **扁平超长表达式的栈溢出端到端复现**：[4.7.3 R3-1](#473-第三轮--契约边界发布) 的三条静态事实已成立，但"是否在现实栈容量下确实 abort"未在本机跑通 `[待核验]`；应作为 Batch 6 立项的第一个动作。
7. **各指标零判定 fallback 契约**：TA-Lib 的 `TA_IS_ZERO` 带宽为 1e-8，而各指标判零后的 fallback（`0.0` / `50.0` / `(0,0)`）尚无逐指标记录；需结合对拍金标补齐 `[待核验]`。

---

# 附录 A 复核命令清单（Batch 0 可复现）

以下命令均为**只读**，用于复现本文的"仍成立"判定：

```bash
# 结构性门禁（全部应 exit 0）
python scripts/check_orphan_modules.py
python scripts/check_rust_source_reachability.py
python scripts/check_dead_code_allows.py
python scripts/check_unbounded_loops.py
python scripts/check_no_tracked_build_artifacts.py

# 绑定对齐
python scripts/audit_binding_parity.py --check

# 悬空 feature（应无输出）
grep -rn "circuit-breaker" core/src
grep -rn "precision-f32" core/src

# 多份实现（应有匹配）
grep -rn "fn adx_into" core/src
grep -rn "fn true_range_fast\|fn true_range" core/src
grep -rn "> 1e-15" core/src/indicators/momentum.rs

# 文档漂移检查
grep -n "eval_simd\|hot loop\|JIT" docs/architecture/formula-engine.md

# 绑定 feature 声明 vs 实际编译态
cargo tree -p finkit-python -e features -i finkit

# 各绑定声明的 core feature 组合
grep -n "finkit = " ffi/*/Cargo.toml

# 打包目录内残留产物
ls ffi/python-binding/finkit/*.pyd
find ffi/dotnet-binding -name '*.dll' | wc -l
```

## 核心逻辑深度审查复现命令（对应 4.7，只读）

```bash
# R1-1：NaN 三态
sed -n '13,17p' core/src/utils.rs                         # f64::max 版 true_range（吞噬 NaN）
sed -n '4991,5004p' core/src/indicators/momentum.rs       # true_range_fast（传播 NaN）
grep -rn "non-finite value at index" core/src | wc -l     # 仅少量入口拒绝非有限
grep -rn "fn validate_ohlc" core/src/indicators/volatility.rs   # 只查长度
# R1-2：零判定带宽
grep -rn "> 1e-15" core/src/indicators/momentum.rs | wc -l
# R2-1：repaint 宏零引用
grep -rn "impl_repaint!\|impl_compute_bar!" --include=*.rs core ffi wasm | grep -v "streaming/macros.rs\|streaming/repaint.rs"   # 应无输出
grep -rn "struct SnapshotState" core/src | wc -l           # 10 份手写
# R2-2：缓存不统一
sed -n '203,214p' core/src/runtime_context.rs             # "the one cache" 声明
grep -n "Unbounded" core/src/formula/engine.rs             # semantic_plan_cache 自述无界
grep -rn "ArtifactKey::new\|ArtifactKey::from_bytes" core/src   # 仅 unified_runtime 使用
# R3-1：递归深度事后检查
grep -n "fn lower" core/src/formula/compute_ir.rs          # 无深度校验，仅记录
sed -n '794,800p' core/src/formula/engine.rs              # check 在 lowering 之后
grep -n "struct ExecSandboxConfig" -A 10 core/src/formula/sandbox.rs   # 默认 unlimited
```

# 附录 B 本文未覆盖的范围

- 不包含**代码改动**：本文是方案与基线，实际重构在 Batch 1 起逐批立项。
- 不含**性能优化配方**：窄带成因未定位，见 6.3。
- 不含**跨平台验证**：受本机环境限制。

---

# 第七部分 实施状态（2026-10-07 落地轮）

本节记录 V5 各批次的落地进度。核验：`cargo test -p finkit --tests` 65/65 target 全绿（含 `golden_talib_tests` 对拍），内务门禁与新增门禁全绿。

## 已落地

| 批次 | 项 | 落地内容 | 门禁 |
| --- | --- | --- | --- |
| Batch 0 | 基线核验 | 附录 A 命令在落地轮开始时于 HEAD 复跑，与本文判定一致 | — |
| Batch 6 | ①②③ | ① `compute_ir::lower()` 边下推边校验深度（新增 `ComputePlanError::LoweringDepthExceeded`；预算 2048——实测 `lower_inner` 帧肥，8192 会溢 2 MiB 线程栈）；② 原生 parser 补源长度上限 1 MiB（对齐 Pine）+ AST 深度上限 1024：迭代式检查 + `AstNode::dismantle_ast` 迭代式拆解，同时封死"拒绝后的深树递归析构溢出"这一 R3-1 未记录的深层崩溃面；③ sandbox 默认保持 unlimited **并文档化为契约**（`sandbox_unlimited` 同时是引擎单指标 fast-path 启用开关，有界默认会静默改变所有用户的数值路径；崩溃面已由 ①② 结构性封死）；Pine 映射产物同样受深度上限约束 | `core/tests/formula_input_bounds.rs`（7 项） |
| Batch 5 | ①②③④⑤ | ① `factor-analysis`/`visualization`/`ffi-common` 改 `default-features = false` + 显式 features（`cargo tree` 实证 python 绑定不再被 unification 拉回全量 default）；② 九个绑定的 feature 组合固化为契约表；③ python/node 的 `formula` feature 显式映射 `finkit/formula-jit`+`formula-simd`，不再依赖间接巧合；④ 新门禁（9/9 绿；ios/android 如实记录经 `ffi-common` 传递的 `formula`+`serde`）；⑤ 删除 maturin 6 个逐字重复平台块 | `scripts/check_binding_feature_contract.py`；node/java/dotnet/c 逐一 `cargo check -p` 通过；python 绑定因本机 pyo3 构建脚本受 Windows 管道耗尽（os error 231，同 V4 §12 环境问题）未能本地 check，由 `cargo tree` feature 验证 + CI 兜底 |
| Batch 1 | ①② | `momentum.rs::adx_into` 改为驱动 canonical `AdxState`/`DmiState`，删除手抄 Wilder 递推与传播式 `true_range_fast`；唯一 true-range 语义 = `f64::max` 吞 NaN（TA-Lib `fmax`），公开数值在 NaN 输入下统一为 TA-Lib 行为 | `core/tests/adx_nan_golden.rs`（4 项：干净/含 NaN OHLC 逐位一致、平坦市场 fallback、`is_zero` 语义） |
| Batch 1 | ④ | 唯一政策点 `utils::is_zero` / `TA_IS_ZERO_BANDWIDTH`（1e-8，`ta_utility.h` 出处）；canonical DMI 内核切换至该谓词，DI/DX fallback=0.0 契约逐条记录。**其余 ~500 处 `1e-15` 守卫的机械替换为独立后续步骤**（须逐指标金标） | `adx_nan_golden` 两项 |
| Batch 1 | ③ | LINREG 现状固化 + 新增跨路径一致性门禁（SIMD 播种 vs warm-start 手写递推，1e-12 内一致） | `math::linear::tests::simd_seeded_and_warm_started_linreg_paths_agree` |
| Batch 3 | 缓存 | 引擎三缓存（`FormulaPlanCache`/`semantic_plan_cache`/`bytecode_cache`）统一 `ENGINE_CACHE_CAPACITY`(1024) FIFO 有界驱逐（`BoundedFifoMap`）；`ArtifactCache` "唯一缓存"声明修正为如实描述 | `plan_cache_tests` 新增 4 项 |
| Batch 3 | repaint 宏 | 删除零引用宏 `impl_repaint!`/`impl_compute_bar!`/`impl_next_with_time!` 与 `streaming/repaint.rs`（V5"二选一"的删除分支）；门禁顺带清理另 5 个零引用导出宏（`timed!`×2、`idx!`、`idx_mut!`、`impl_indicator_meta_and_methods!`） | 新门禁 `scripts/check_unused_macros.py` |
| Batch 4 | ②④ | ② `docs/architecture/formula-engine.md` 重写对齐冻结契约（JIT=peephole+bytecode 解释、`eval_simd`=`eval` 别名、生产路径=plan）；④ `git clean -fXd` 移除 `finkit/*.pyd`（~12.9 MB）与 dotnet 110 个 `.dll` | 文档无漂移残留；`find ffi/dotnet-binding -name '*.dll'` = 0 |
| Batch 2 | 函数表 | Layer 2 提炼为 `canonical_overrides()` + 显式新增名单 `canonical_new_names()`；三层边界门禁：覆盖必须有 legacy 锚点、新增名单不得与 legacy 冲突、talib_081 只增不盖 | `formula::functions::tests::function_table_layers_stay_in_their_lanes` |

## 未完事项（后续独立提交）

1. ~~**Batch 2 大文件拆分（纯移动）**：`functions_legacy.rs`(7254 行) / `indicators/momentum.rs`(5838) / `math/simd_ops.rs`(5300) / `ffi/python-binding/src/lib.rs`(6954)~~ —— **已在[第九部分](#第九部分-batch-2-大文件拆分纯移动2026-10-08)落地**：前三个已拆为同名目录模块（路径与公开面不变，65/65 target 全绿），python 绑定亦已拆为 24 个文件。
2. **Batch 5 python 绑定本机 check**：再次尝试仍被 `os error 231`（"所有的管道范例都在使用中"）阻塞，pyo3 构建脚本起不来 Python 解释器。由 `cargo tree` 验证 feature 契约 + CI `python-build-state` 兜底。`PYO3_NO_PYTHON=1 cargo check -p finkit-python --features abi3` 可作本地静态兜底。

### Batch 2 可行性核定（本轮实测，供下轮直接开工）

- `functions_legacy.rs`：316 个顶层项；`fn_*` 体之间的交叉引用只有 **52 个目标**，且集中在三个共享 helper——`ensure_args_len`(219 处引用)、`nan_vec`(190)、`extract_n`(103)。把这三个（加 `extract_ma_code`/`extract_f64_arg`/`ArgExtremeDeque`/`get_string_from_hash`）留在 `mod.rs`，其余按委派的指标模块分桶即可；子模块用 `use super::*;` 拿到私有项。
- 分桶的天然依据是**委派面**：`lib_ma` 22 个、`lib_momentum` 19、`lib_candlestick` 11、`lib_stat` 9、`lib_cycle` 7、`lib_astock` 7、`lib_classic` 6、`lib_linear` 5、`lib_math_operators` 2。
- 注意：**不能**用 `get_builtin_functions()` 体里的空行分块来分桶——实测 355 个 `map.insert` 只解析出 215 个（大量是多行格式），且切出 136 个碎片块，没有可用的语义边界。

## Batch 4 ③ 警告预算（已落地）

见 [docs/quality/warning_budget.md](../docs/quality/warning_budget.md)。要点：

- 形式是**棘轮**而非待办清单：提交每个 lint 的当前条数，门禁在任一 lint **上升**时失败。
- 基线 **14647 条 / 125 个 lint**（`cargo clippy -p finkit --all-targets`）。
- 两个非显然的实现决定（都踩过坑）：必须 `--message-format=json`（`short` 不带 lint 代码，总数基线不构成棘轮）；必须过滤"无 span 的工具链噪声"（本机沙箱的增量编译锁文件清理告警单独贡献 110 条假警告，会让预算结论随文件系统漂移）。
- 前三类 lint 合计约 7000 条（占近一半）**不该直接修**：`cast_precision_loss` 在以 `f64` 为契约的金融库里修掉等于把 `f32` 引入数值路径。这类应以 crate 级 `allow` 显式豁免并写明理由。

## Batch 2 的另一半：规模门禁（已落地）

`python scripts/check_file_size_budget.py`（基线 `scripts/file_size_budget.json`）——
逐文件行数棘轮：超过 1500 行默认上限的文件按当前规模入基线，任何文件**变大**
即失败；变小则同提交下调基线。默认上限之上的文件被显式记录后，拆分就从"计划里的一句话"
变成可量化的数字。门禁先于拆分落地（当时 46 个文件在册）；[第九部分](#第九部分-batch-2-大文件拆分纯移动2026-10-08)
拆完之后降为 **42**，全库最大文件从 `functions_legacy.rs` 的 7254 行降到
`patterns/candlestick.rs` 的 4490 行。


---

# 第八部分 第二批落地（2026-10-07 晚）

## R1-2 的真正结论：金标教出了一个"双带"政策

上一轮把 canonical DMI 内核切到 `utils::is_zero`（TA-Lib `TA_IS_ZERO`，1e-8 带宽）后，V5 把余量描述为"~500 处 `1e-15` 守卫的机械替换"。**按字面执行是错的**，本轮实测证明：

1. 首次 codemod 对全树 674 处 `1e-15` 比较统一切到 1e-8 带宽（含公式引擎的 `/` 运算符）。
2. `core/tests/alpha158_parity.rs::the_division_guard_explains_the_wvma_family` **失败**。
3. 根因：Alpha158 因子库沿用 Qlib 的 `+1e-12` 分母正则化，**正常分母就在 ~1e-12**。1e-8 带宽会把它们全部判零 → 整个因子库变 `NaN`。公式运行时的除零守卫是**语言运行时契约**，不是 TA-Lib 对齐旋钮。

于是 V5 R1-2 的原始方向（引用的全部是 `momentum.rs` / `volume_ext.rs` / `math/moving_avg.rs` 等**指标**内核）按其本意收敛为**两个具名带**，而不是一个：

| 面 | 带 | 谁的数字 | 依据 |
| --- | --- | --- | --- |
| `indicators/**`、`streaming/**`、`math/kernels/**` | `TA_IS_ZERO_BANDWIDTH` (1e-8) | TA-Lib 的 | `ta_utility.h`；输出契约即"TA-Lib 打印的那个数" |
| 公式运行时、`features/**`、`transforms/**`、`risk.rs` | `NUMERIC_EPSILON` (1e-15) | finkit 自己的 | Alpha158 `+1e-12` 正则化；`alpha158_parity` 金标锁定 |
| `streaming::float_trait::Float::epsilon()` | 1e-15（不变） | — | f32/f64 泛型；1e-15 低于 f32 分辨率，钉到任一带都会静默改变所有 f32 流 |

落地量：A 面 **215 处**（语义对齐，`.abs() > 1e-15` → `!is_zero(x)`、裸比较 → `TA_IS_ZERO_BANDWIDTH`，含 8 处 SIMD 广播常量）；B 面 **189 处**（**保值重命名**，数值零变化）。契约测试 `alpha158_parity` 改为跟随 `NUMERIC_EPSILON` 常量而非硬编码字面量，并在其文档中记录"为何此处不可放宽到 1e-8"。

**门禁** `scripts/check_zero_guard_policy.py`：全树禁止再出现裸的近零比较字面量，只允许 `is_zero` / `TA_IS_ZERO_BANDWIDTH` / `NUMERIC_EPSILON` 三种写法。首次运行即抓出一处**既有的**内联 `1e-8`（`features/rolling_stats.rs` 的 Lanczos 收敛判据），已改走政策点。

**可复现的判读口诀**：改判零带宽后若 `alpha158_parity` 失败，根因几乎一定是把 B 面的守卫当成了 A 面的；先查该守卫是"语言运行时契约"还是"TA-Lib 对齐面"。

## R2-1 后续：手写快照的收敛（Batch 3）

10 份 `SnapshotState` 的重复**不在状态字段**（每份的字段确实不同），而在**回滚纪律**：每个文件都手写一遍 `Option<SnapshotState>` + `i64`，以及决定是否回滚的 `t != 0 && t == self.last_open_time`。手写副本的失败模式不是编译错误，而是**恢复了指标自身字段却漏掉 `last_open_time`**——此后每次重喂都不再匹配，同一根 bar 被折进两次，且完全静默。

因此新增 `core/src/streaming/forming_bar.rs::FormingBar<S>`，把纪律单源化：指标只贡献自己的状态，12 个回滚点（`sma`/`ema` 各 2 个）全部改为 `bar.take_rollback(t)` + `bar.begin(t, snap)`。`FormingBar` 按值移动 `S`（`macd_ext` 的快照持有三个 `MaState` 克隆，故不能要求 `Copy`）。`SmaSnapshot` 的公开 API 保留，由 `FormingBar::set_open_time` 承接"恢复而非折叠"的语义。

**门禁** `scripts/check_forming_bar_discipline.py`：禁止 `Option<SnapshotState>` / `last_open_time: i64` 这对手写字段重新出现（`SmaSnapshot` 的 `pub(crate)` 字段为唯一白名单）。



---

# 第九部分 Batch 2 大文件拆分（纯移动，2026-10-08）

第七部分把 Batch 2 的"大文件拆分"留成了未完事项，理由是"7160 行的纯移动不可能在一轮内做到
改完 + 65 个 target 全量验证"。本轮把它做完了，并顺带发现：**这条拆分的真正风险不是工作量，
而是"纯移动"本身很容易在无声处变成"有损移动"。**

## 结果

| 原文件 | 行数 | 拆分后 | 文件数 | 公开面 |
| --- | ---: | --- | ---: | --- |
| `core/src/formula/functions_legacy.rs` | 7254 | `formula/functions_legacy/` | 21 | 不变 |
| `core/src/indicators/momentum.rs` | 5838 | `indicators/momentum/` | 20 | 不变 |
| `core/src/math/simd_ops.rs` | 5300 | `math/simd_ops/` | 13 | 不变 |
| `ffi/python-binding/src/lib.rs` | 6954 | `ffi/python-binding/src/*.rs` | 24 | 不变 |

三个 core 模块的模块路径（`crate::formula::functions_legacy` 等）与公开符号逐个不变，因此
对上游是**零感知**的纯移动。验收：`cargo check -p finkit --all-targets` 零 error 零
rustc warning；`cargo test -p finkit --tests` **65/65 target 全绿**；全部既有门禁保持绿，
且没有靠放宽门禁过关（见下"门禁增量"）。

## 教训 1：块切分必须无损，否则文档会被静默吞掉

第一版切分器从"第一个非注释行"开始起块，于是每个 item 前面的空行 + `///` 文档注释 +
`#[...]` 属性落进**无人认领的空隙**被丢弃。症状极具欺骗性——代码照样编译：

- `indicators/momentum.rs` 的 `///` 从 910 掉到 40，`math/simd_ops.rs` 从 187 掉到 3；
- `# Errors` 段 2 → 0（顺带在警告预算里表现为 `missing_errors_doc` +4）。

修法是把块定义为"从上一个 item 结束后的第一行（含前置空行/注释/属性）到本 item 结束"，
并在切分器里加**结构性断言**：`blocks[0][0] == 0`、相邻块 `b + 1 == c`、
`sum(len(v)) == len(blocks)`，再加一行 `tiling: N blocks cover M of L lines (tail T)`
打印。有了这些断言，"丢了 870 行"在第一次运行就会暴露，而不是等到金标测试失败。
修复后 `///` 与 `# Errors` 计数与原文件**逐字一致**（926/2、187/0、82/0）。

## 教训 2：桶边界是"委派面"，不是空行

`functions_legacy` 的天然分桶依据是**每条 `fn_*` 委派给了哪个指标模块**
（`lib_ma`→`overlap`、`lib_momentum`→`momentum`、`lib_candlestick`→`candlestick` …），
不是 `get_builtin_functions()` 里的空行——实测那 355 个 `map.insert` 按空行只能切出
136 个碎片块，没有语义边界。同理 `momentum` 按指标家族、`simd_ops` 按
`dispatch` / `avx2_*` / `scalar_*`。

共享 helper（`ensure_args_len`、`nan_vec`、`extract_n`、`ArgExtremeDeque` …）留在
`mod.rs`。这里有一个**编译器给不出好提示**的坑：`impl<const WANT_MAX: bool> ArgExtremeDeque<WANT_MAX>`
没有可被 `item_name()` 识别的朴素 `impl Name`，会被误判成普通 item 塞进别的桶，
于是结构体留在 `mod.rs`、`impl` 块跑去 `reference` 桶，直接抛 **66 条 `E0624 method is private`**。
判定规则改成"整行出现 `mod.rs` 里的名字即归 `mod`"。

## 教训 3：`use super::*;` 是拆分的隐性账单，改用具名 `prelude`

每个桶都要用父模块的 imports 和 helper，最自然的写法是 `use super::*;`。40 个桶这么写
就给警告预算加了 **+40 条 `clippy::wildcard_imports`**——拆分"没有改行为"，但它确实改了
lint 账本。

`clippy::wildcard_imports` 有一条豁免：**模块名就叫 `prelude`（或以 `_prelude` 结尾）
的 glob 不报**。于是每个目录加一个 `prelude.rs`：

```rust
//! Shared prelude for the `momentum` family.
//! …
pub(crate) use super::*;
```

桶里改写成 `use super::prelude::*;`。这一条在落地前**先用一个最小复现包实测确认过**
（`use super::prelude::*;` 不报、`use super::*;` 报；父模块的私有项与私有 `use`
（含 trait，如 `std::fmt::Write`）都能穿过 `pub(crate) use super::*;` 被孙级模块拿到），
没有靠猜。结果 `wildcard_imports` 精确回到基线 **99**。

顺带清理：`mod.rs` 里那些"没有任何东西解析得到"的 `pub(crate) use bucket::*;`
是真实的 `unused_imports`，只保留 sibling 桶确实依赖的那些（`simd_ops` 的 `avx2_*`/`scalar_*`
保留，`capability`/`dispatch` 删除；`momentum` 一个都不需要，因为 sibling 依赖的都是公开 API）。

## 教训 4：门禁增量的归因要精确到"哪一行是新的"

`+40 wildcard_imports` 好归因（就是那 40 行）。`doc_markdown +4` 不好归因——新目录里有
120 条 `doc_markdown`，绝大多数是从原文件搬过来的既有项。做法是把**新目录的文档行**对
`HEAD:` 原文件做多重集差集，得到"新增的文档行"，再和 lint 的 span 取交集，精确落到
**两个站点**（拆分器生成的模块文档里 `ZigZag` / `StochRSI` 没加反引号），修正后归零。

## 门禁增量与处理

| 门禁 | 增量 | 处理 |
| --- | --- | --- |
| `clippy::wildcard_imports` | 99 → 139 | 具名 `prelude` 方案 → **99**（=基线） |
| `clippy::doc_markdown` | 915 → 919 | 修正 `ZigZag`/`StochRSI` 反引号 → **915**（=基线） |
| `clippy::missing_errors_doc` | +4 | 随"无损切分"修复自动消失（`# Errors` 段回来了） |
| `clippy::module_inception` / `duplicated_attributes` | +2 / +4 | 解包冗余 `mod tests { … }`、删除重复内层属性 → 0 |
| `unused_variables` | 1 → 0 | **改善**；重基线（见下） |
| `check_file_size_budget` | 46 → 42 在册 | `--write` 重基线（3 个巨型文件出册） |
| `check_rust_source_reachability` | — | 542 个受追踪源文件全可达 |
| `check_orphan_modules` / `check_orphan_scripts` / `check_python_stub` / `check_zero_guard_policy` / `check_forming_bar_discipline` | — | 全绿（存根 95 个类） |

**警告预算重基线**：`scripts/warning_budget_baseline.json` 从 14647 收紧到 **14646**
（124 个 lint）。同时给门禁本身加了一条防伪修正：

```python
env = {**os.environ, "CARGO_INCREMENTAL": "0"}   # run_clippy()
```

增量编译会**重放**上一次修订缓存下来的诊断，于是一个源码上已经干净的构建仍可能报出
不存在的告警（本轮真的撞上了：一条 `unused_variables` 幽灵）。预算是源码级棘轮，
输入必须固定。

## 顺带修掉的一个真实缺陷（python 绑定）

存根 `finkit/__init__.pyi` 承诺的是 `KlineData` / `KlineChart`，而 Rust 侧注册名是
`PyKlineData` / `PyKlineChart`；`finkit/__init__.py` 靠两行手写别名
（`KlineData = _native.PyKlineData`）把差异兜住。已在 `ffi/python-binding/src/charts.rs`
加 `#[pyclass(name = "KlineData")]` / `#[pyclass(name = "KlineChart")]`，删掉别名，
`examples/kline_chart_example.py` 同步改为 `ta.KlineData`。用 `git archive HEAD` 在纯 HEAD
源码上跑存根门禁可确认：**93 个类**（不含 `KlineData`/`KlineChart`）——即这两个类此前
的 `Py` 前缀属于**既有**缺陷（`LEAKED_PREFIX_RE = ^Py[A-Z]` 会命中），不是本轮引入的。

## 可复现

拆分是纯移动，所以它是**可重跑**的。三个一次性迁移脚本留在 `scripts/archive/`——
该目录被 `.gitignore` 有意排除（"Legacy scripts preserved locally, not committed"），
所以它们**不在仓内**，只存在于执行过拆分的机器上。不接 CI，因为它们会改源码：

```bash
bash scripts/archive/rebuild_splits.sh
```

- `split_rs_module.py momentum|simd_ops --write`：按家族/内核分桶，`--write` 才落盘。
- `split_functions_legacy.py --write`：按委派面分桶。
- `rebuild_splits.sh`：取三个单文件 → 跑拆分器 → 装配 → 删除单文件 → 打补丁 → `cargo fmt`。

已验证：在隔离 worktree 中对当前内容重跑整套流程，产物与提交内容**逐文件一致**
（仅换行符差异——`.gitattributes` 规定 `*.rs text eol=lf`，仓内 LF、工作树 CRLF）。

补丁只包含拆分器**无法从源码文本推断**的三件事：CDL 宏展开的 wrapper 需要
`pub(crate) fn`（否则 sibling 桶看不见）、深层相对路径 `super::functions::` 要改绝对路径、
以及哪些桶真的用得到共享 prelude（这决定 `use super::prelude::*;` 的取舍）。
