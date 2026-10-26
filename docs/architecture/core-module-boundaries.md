# Core 模块边界与变更影响

> 基线：Finkit `v0.2.0` 源码架构；此文规定职责和变更验证，不代表模块之间已经由编译器强制隔离。模块调整须保持 `finkit` crate 根公开 façade 的兼容性。

## 设计约束

1. 数值语义和可复用 kernel 归 `core`；CLI、FFI、语言 binding 和发布适配不得成为 kernel 的依赖。
2. batch、formula、streaming、factor/runtime 是不同执行前端；相同操作必须复用或由 differential/golden 验证，但不能为追求“统一”而抹掉各自的输入、状态、范围和错误语义。
3. `lib.rs` 及公开 re-export 是稳定入口。消费者优先依赖 façade，不应依赖为内部组织而暴露的深层路径，除非该路径明确列入 API 合同。
4. 新增横向依赖须证明其职责必要性并附上对应合同测试。任何向 adapter 的生产依赖都违反 layering。
5. crate 拆分须有 clean/incremental build、依赖编译工作量、维护边界和 API 迁移数据支撑；缺少量化收益时保持单 crate。

## 技术职责与权威合同

| 技术责任域 | 主要源码 | 责任边界 / 事实来源 | 修改时至少验证 |
|---|---|---|---|
| 纯数学与数值 kernels | `core/src/math/`、`core/src/utils.rs` | 通用 rolling、统计、窗口和基础计算；数值合同以基准向量、语义说明和 canonical kernel 为准 | 核心 golden / TA-Lib 数值合同；`allocating == into` 与相关 benchmark |
| 批量指标与形态 | `core/src/indicators/`、`core/src/patterns/` | 对完整切片/数组执行的计算 API；registry 与 operation metadata 描述发现和参数合同 | `golden_tests`、TA-Lib/模式合同、API surface、into/并行一致性 |
| 公式语言和执行 | `core/src/formula/` | lexer/parser、方言、AST、优化、Tree 参考路径、Plan 编译/调度与缓存适配；公式函数注册器是公式 callable 的事实来源 | Tree/Plan differential、absolute golden、方言/语料、cache/input-bound/entry-contract gates |
| 逐点状态执行 | `core/src/streaming/` | 每 bar 状态、checkpoint、forming-bar、streaming registry 和指标元数据；它不是任意 batch 调用的自动包装 | streaming registry 双向合同、batch/stream parity、reset/checkpoint、repaint 与长期稳定性 |
| 语义图、计划与统一执行 | `semantic_graph.rs`、`compute.rs`、`execution_plan.rs`、`unified_executor.rs`、`unified_runtime.rs`、`runtime_context.rs` | graph lowering、计划/slot、执行策略、cache artifact、dirty range 与安全回退；不替代公式/streaming 专属状态语义 | graph/runtime contracts、full vs range/into、缓存失效、DirtyRange 保守 fallback |
| 因子与复合计算 | `factor_graph.rs`、`factor_provider.rs`、`factor_system.rs`、`factors/`、`composite.rs` | 用户可组合的研究/计算节点；不是交易订单、回测成交或组合构建引擎 | FactorPlan/Composite contract、multi-target、借用/范围/流式一致性 |
| 特征工程与数据转换 | `features/`、`transforms/`、`data_contract.rs`、`returns.rs` | 输入面板、序列转换、标签和对齐；时间信息、as-of 与未来数据边界须显式 | schema/temporal contract、未来信息不变量、输出 shape/alignment 测试 |
| 发现与公共 schema | `registry.rs`、`schema.rs`、`operation.rs` | 对外可发现的指标/API 元信息与版本化请求 schema；生成文档必须来自指定 SSOT | SSOT generator/check、schema fixtures、binding surface/parity 门禁 |
| 研究产品层 | `factor-analysis/` workspace crate | 对调用方提供的数据和收益进行因子研究、验证和报告 | `research_invariants`、版本化 JSON 合同、Rust/Python/Node/CLI consumer tests |
| 语言与命令适配 | `ffi/`、`wasm/`、`cli/` | 参数/内存/schema 适配与产品入口；不重写核心数值算法 | binding parity、共享 engine contract、包外 consumer、声明平台构建 |

表中的“权威来源”彼此有明确区别：indicator registry 不等于 formula callable registry；binding parity 只涵盖登记的 indicator API；名字能匹配不证明语义 parity。`scripts/check_coverage.py` 仅报告规范化名称精确匹配或显式别名，不替代上列合同。

## 变更影响路径

| 改动起点 | 直接消费者 | 易遗漏的回归面 |
|---|---|---|
| math/kernel 数值与 NaN/warm-up | batch、formula、factor、streaming | 多输出/短输入、相邻运算组合、分配和 into 变体、语言输出对齐 |
| formula parser/lowering/kernel | Tree、Plan、Python/Node 公式 API、缓存 | dialect、string/draw/multi-channel、输入修订、明确拒绝的 Tree-only 入口 |
| Streaming state 或 registry | streaming builder/bindings、文档发现 | 公共 re-export、元数据类别、形成中 bar 重绘、checkpoint 恢复及 batch 对照 |
| FactorPlan/UnifiedRuntime/cache | factors、composite、研究上游 | range 端点、DirtyRange 扩散、identity/revision 隔离、未知语义 fallback 到 full |
| schema/registry/FFI | Rust、Python、Node 和其他 tier | 兼容性版本、共享 contract fixture、语言错误映射与数值形状 |
| 因子研究 prepare/analyze/report | Rust API、Python/Node JSON、Factor Study CLI | 输入 schema、空/短样本、收益对齐、point-in-time 不变量及报告稳定字段 |

## 当前治理证据与剩余工作

- Workspace crate 依赖层级由 `scripts/check_dependency_direction.py` 门禁：生产依赖边全部向下；`core` 不再通过 dev-dependency 引入 `ffi-common`。适配器 catalog/dispatcher 合同由 `ffi-common` 自己验证，core 只保留公式与数值相关的检查。
- core 的公开 façade 位于 `core/src/lib.rs` 及对应模块 re-export；Formula、Streaming 和 batch 等领域仍保留各自执行器，不能宣称已全部集中到一个 Runtime。
- 当前尚无可比较的 clean/incremental 编译基线。本轮未执行会清空现有构建缓存的操作，也未提取新 crate。后续若提议拆 crate，先在独立 target 目录记录 clean 与增量构建时间/工作量并比较改造成本。
- 回测不在 Finkit 产品边界内；任何模块拆分、研究 API 或工作流测试都不应引入模拟成交/策略执行。
