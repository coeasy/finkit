# 运行时与缓存合同

- 适用版本：Finkit 0.2.x（workspace 目标版本；已发布资产仍为 0.1.15）
- 状态：**合同文档（单一事实来源）**。缓存容量、key 语义、淘汰策略、统计字段、
  清理 API 与线程共享要求都在此写明，并可从源码常量与门禁追溯。
- 相关：[公式引擎内部](formula-engine.md)、[公式运行时合同](../formula-runtime-contract.md)、
  [核心契约](../core-contracts.md)、[数值语义](numerical-semantics.md)。

本页存在的理由：公式引擎与统一运行时各自持有编译产物缓存，**它们都是有界的，但 key、
淘汰策略、统计字段与生命周期并不完全共享**。把它们说成"一个缓存 / 一个 Runtime"会超出
实际；本页先固化各层真实合同，再谈是否合并。

## 1. 缓存清单

| 缓存 | 归属 | key | 淘汰 | 容量 | 统计入口 | 清理 |
|---|---|---|---|---|---|---|
| `FormulaCache`（tree 编译产物） | `FormulaEngine.cache` | source hash（再比对 source 防碰撞）+ feature set | **LRU** | 100（`FormulaCache::new(100)`；`with_cache_size(n)` 可改） | `cache_hit()` / `cache_size()` | `clear_cache()` |
| `semantic_plan_cache`（语义 DAG） | `FormulaEngine` | 精确公式 source | FIFO | `ENGINE_CACHE_CAPACITY` = **1024** | `plan_cache_size()` 间接 | `clear_cache()` |
| `plan_cache`（`FormulaPlanCache`，hot numeric plan） | `FormulaEngine` | source + dialect + 参数指纹 | FIFO | `ENGINE_CACHE_CAPACITY` = **1024** | `plan_cache_size()` / `FormulaPlanCacheStats` | `clear_cache()` |
| `bytecode_cache` | `FormulaEngine` | source + feature set | FIFO | `ENGINE_CACHE_CAPACITY` = **1024** | — | `clear_cache()` |
| `ArtifactCache`（跨层编译产物） | `RuntimeContext` | `ArtifactKey { namespace, content_hash }` | **LRU**（clock） | `ExecutionLimits::max_artifact_entries` = **128** | `ArtifactCacheStats { entries, capacity, hits, misses, evictions }` | `clear()` / `set_capacity()` |

容量常量集中在 `core/src/formula/engine.rs`（`ENGINE_CACHE_CAPACITY`）与
`core/src/runtime_context.rs`（`ExecutionLimits`），**不散落内联**。

## 2. 合同规则

1. **硬容量上限。** 每个缓存都有界；任何"无界增长"都是违约。FIFO 缓存在到达容量时
   淘汰最旧条目，LRU 缓存淘汰最久未使用条目。
2. **key 语义分层，不共享。** 语义 DAG（仅按 source）、hot numeric plan（source +
   dialect + 参数）、tree AST 产物与 `ArtifactCache` 的 `namespace + content_hash`
   是四套不同的 key。宣称"一个缓存"只有在 key/lifetime 规则真正一致时才成立——
   当前并非如此。
3. **命名空间隔离。** `UnifiedRuntime` 用 `ArtifactCache` 的 namespace 区分
   `FINPLAN0`（as-declared）与 `FINPLAN1`（CSE 优化）计划，二者**永不互相命中**
   （`the_plan_cache_separates_optimization_settings`）。
4. **失败的构建不缓存。** 一个 `GraphPlanError` 之后修正的声明不会被旧失败污染。
5. **清理覆盖派生计划。** `FormulaEngine::clear_cache()` 在 `Tree` 模式下也清掉
   `plan_cache`（`clear_cache_drops_the_plan_cache_from_tree_mode_too`），否则调用方
   要求"干净起点"后 plan 模式仍会跳过重编译。
6. **模式感知的统计。** `cache_hit` / `cache_size` 报告**当前激活后端**的缓存；plan
   模式下报告 AST 缓存会让热/冷引擎无法区分
   （`cache_statistics_follow_the_active_mode`）。
7. **`set_capacity` 立即降容。** 调小容量会当场淘汰到新上限。

## 3. 线程与生命周期

- `FormulaEngine` 的缓存是每引擎 `RefCell`，因此**同一引擎不是 `Sync`**；
  每个线程/任务应持有独立引擎，或在 Rust 层用明确的线程安全封装。
- `RuntimeContext::ArtifactCache` 由统一运行时拥有；跨语言绑定里 `CompiledFormula`
  是 unsendable，不要在线程间共享同一实例。
- borrowed 输入只在一次同步调用期间有效，不得跨调用保存裸指针。

## 4. 增量执行（DirtyRange）

- 只有**有完整依赖链证明**的 `DependencyShape::FixedLookback` 才允许 DirtyRange；
  其余（`Expanding`/`Dynamic`/`CrossSectional`/`Global`/未知）**保守回退**到 full run。
- `dependency_shape(id)` 折叠整个上游锥；任一上游节点不可 range，则整条链拒绝 range。
- 增量结果必须与全量重算**逐值一致**（`runtime_convergence.rs` 的
  `full == range` 边）。

## 5. 观测与门禁

| 关注点 | 入口 / 门禁 |
|---|---|
| 有界性 | `engine.rs` 的有界性测试（喂入 > `ENGINE_CACHE_CAPACITY` 条公式） |
| 命名空间隔离 | `unified_runtime.rs` 的 `the_plan_cache_separates_optimization_settings` |
| 清理覆盖派生计划 | `core/tests/formula_execution_mode.rs` |
| 模式感知统计 | `core/tests/formula_execution_mode.rs` |
| 增量 == 全量 | `core/tests/runtime_convergence.rs` |
| 合同值一致性（本文档 ↔ 源码） | `scripts/check_formula_engine_contract.py`（`make check-formula-contract`） |

## 6. 决策规则：何时才合并缓存

在长时、多公式、多租户负载量化出各缓存的占用与命中率之前，**不把公式引擎缓存迁入
`ArtifactCache`**。合并的前提是：

1. key 语义、淘汰策略、统计字段与清理 API 能统一为一份合同；
2. profiling 证明共享带来真实收益，而不是名义上的 SSOT；
3. 迁移不改变任何公开的数量结果。

在此之前，本页描述的分层缓存是**有意的设计**，不是待清理的技术债。**不以"行数最少"
或"缓存最少"为目标。**
