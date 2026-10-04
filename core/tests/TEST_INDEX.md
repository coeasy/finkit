# 测试组织索引

本文档说明 `core/tests/` 目录下测试文件的分类和组织结构。**文件清单必须与
`ls core/tests/*.rs` 一致** —— 该目录只增不减地漂移过（曾列出 3 个不存在的文件、
漏掉 23 个实际文件），所以每次增删 target 都要同步这里。

## 测试分类

### 兼容性测试 (Compatibility)
测试与外部系统 / 方言的兼容性。

- `dzh_compat_tests.rs` - 大智慧指标兼容性
- `em_compat_tests.rs` - 东方财富指标兼容性
- `fox_compat_tests.rs` - 飞狐交易师（FoxTrader）**指标**兼容性（`FOX_ZIG` / `FOX_PEAK` /
  `FOX_TROUGH` 及其 bar 偏移）。该方言的买卖信号与回测函数已按产品边界删除
- `tdx_compat_tests.rs` - 通达信指标兼容性
- `ths_compat_tests.rs` - 同花顺指标兼容性
- `pine_corpus_runner.rs` - Pine Script v5 语料回归
- `polars_integration_tests.rs` - Polars DataFrame 集成

### 边界情况测试 (Edge Cases)
测试极端输入、无效数据、边界条件。

- `edge_case_tests.rs` - 通用边界情况
- `edge_case_invalid_input.rs` - NaN / ±Inf 输入拒绝（R-1）
- `f32_tests.rs` - f32 精度测试
- `extrema_round6.rs` - 极值轮次回归
- `extrema_cached_path.rs` - 滚动极值两策略（≤512 缓存索引 / >512 单调环）对拍：
  NaN 不污染窗口、前导缺失段使首个报告推迟 `period - 1` 根、全缺失窗口报 NaN、
  融合 high/low 内核每腿独立丢弃缺失值，覆盖 `EXTREMA_CACHE_LIMIT` 两侧

### 黄金测试 (Golden Tests)
基于参考实现的基准测试。

- `golden_tests.rs` - 核心黄金测试（预计算参考 CSV）
- `golden_talib_tests.rs` - TA-Lib C golden JSON 对比
- `golden_example.rs` - 黄金测试框架示例
- `golden_reference.rs` - 跨语言 golden：AlphaTA 核心必须复现 canonical 输出
- `accuracy_check.rs` - 精度核对
- `talib_coverage_matrix.rs` - 版本化 TA-Lib 覆盖矩阵契约
- `talib_semantic_contract.rs` - TA-Lib 语义契约
- `common/golden_loader.rs` - 黄金测试数据加载器

### 属性 / 稳定性测试 (Property & Stability)
基于 proptest 的不变量测试与长跑测试。

- `property_tests.rs` - 流式指标属性测试
- `long_run_stability.rs` - 流式 + 批量长跑稳定性（O-4）
- `memory_regression.rs` - 内存回归
- `performance_regression.rs` - 性能回归

### 重绘 / 快照测试 (Repaint & Snapshot)
测试 forming-bar 重绘和状态快照功能。

- `repaint_tests.rs` - 重绘行为测试
- `serde_roundtrip_tests.rs` - 流式指标序列化 / 反序列化测试
- `classic_patterns_tests.rs` - 经典形态（Darvas / Renko / Kagi 等）

### 公式引擎测试 (Formula Engine)
测试公式引擎功能。**本组是「三条执行路径必须一致」的主战场**（见
`docs/formula-runtime-contract.md`）。

- `formula_engine_integration.rs` - 公式引擎集成测试
- `formula_differential_tests.rs` - 手写公式集的四路径差分（tree / bytecode / JIT / plan）
- `formula_draw_parity.rs` - `DRAW` 类语句：两路径返回序列必须一致（tree 给
  `Scalar(0.0)`），plan 侧尚未复现的绘图命令逐项列举成双向门禁 —— 修好一条就必须删掉一条，
  否则会永远「绿」着撒谎
- `formula_string_literals.rs` - 字符串字面量在 plan 路径必须落到
  `ctx.string_table` 并返回绝对槽位（含调用方预填充表格的偏移场景），并把 plan
  dispatcher 仍跑不了的吃字符串函数逐项列出
- `formula_plan_differential.rs` - **全语料**编译计划差分 + 不可腐烂 allowlist
- `formula_host_context.rs` - 宿主上下文（筹码 / 周期 / 资金流 / OHLC）两路径一致性
- `formula_regression.rs` - 公式引擎回归测试套件（D-1 ~ D-6）
- `formula_cache_tests.rs` - 公式缓存测试
- `formula_compat.rs` - 公式兼容性测试
- `formula_compatibility_boundary.rs` - 可执行的兼容性边界契约
- `formula_control_flow.rs` - 公式控制流
- `formula_corpus.rs` - 国内公式语料回归执行器
- `formula_execution_mode.rs` - `FormulaExecutionMode` 是真开关（不是提示）
- `formula_function_ssot.rs` - **三面 SSOT 门禁**（registry / 公式面 / plan kernel）
- `formula_if_truthiness.rs` - `IF` 真值判定不得依赖序列长度
- `formula_loop_unrolling.rs` - `for` 展开与 `INDEX` 语义
- `formula_optimizer_equivalence.rs` - 优化器等价性
- `formula_partial_eval.rs` - 公式部分求值（R-2）
- `formula_terminal_contract.rs` - 终端适配器数值语义契约
- `formula_terminal_golden.rs` - 终端 golden
- `formula_dialect_coverage.rs` - 方言覆盖矩阵
- `formula_registry_signature.rs` - 注册表函数签名契约
- `formula_talib_081_surface.rs` - TA-Lib 0.8.1 公式面覆盖

### 收敛门禁 (Convergence Gates)
§18 的「五条边」每条都必须被钉住：`batch == streaming`、`full == range`、
`allocating == into`、`tree == plan`、`Rust == FFI`。**门禁必须能腐烂检测** ——
allowlist 型条目要双向断言（stale + undeclared），否则清理后的陈旧条目会永远
「绿」着撒谎。

- `runtime_convergence.rs` - **跨执行形态收敛门禁**（batch/streaming、full/range、
  allocating/into）+ 五条边的 artifact 清单 + CSE 数值等价门禁。本文档暴露过 6 个
  真实的流式内核与批量内核不一致（ADX / DX / +DI / −DI / TRANGE / ATR），全部按
  批量语义修正而非加入 allowlist
- `test_index_contract.rs` - **本索引自身的双向门禁**：每个 `core/tests/*.rs`
  必须被本文档列出（新 target 不能悄悄不可发现），本文档列出的每个路径也必须真实
  存在（删除/改名 target 会在这里失败，而不是留下死指针）

### 公开 API 面 (Public API Surface)

「没有任何内部调用方」不等于「没有任何需要的调用方」。`_into` 零拷贝家族就是这类
名字：crate 内部一律直接调 `math::` 规范内核，所以删掉 `indicators::` 侧的公开
包装可以做到全绿 —— `cargo check` 看不见**被删除**的 `pub fn`，单元测试又早已改
成直接调内核，生成态快照只会忠实记录新状态而不会提出异议。这类名字由编译期守卫钉住。

- `indicator_api_surface.rs` - **零拷贝（`_into`）公开入口的编译期守卫**：逐条列出
  53 个 `indicators::<模块>::<名字>_into` 路径，任一被删除或改名都会让本 target
  **编译失败**（而非测试失败），使移除成为一次刻意的、可评审的决定。同时单独钉住
  `core/src/indicators/mod.rs` 的根级重导出接缝 —— `pub use volume::*` 与显式
  `pub use math::volume_kernels::{ad, adosc, obv}` 并存处，显式条目会遮蔽同名 glob
  成员，这正是 `adosc_into` 当初漏掉的地方

### 因子库 / 多周期 (Factor Library & Multi-period)

- `alpha158_parity.rs` - Alpha158 因子库一致性
- `worldquant101_library.rs` - WorldQuant 101 因子库
- `multi_period_resonance_smoke.rs` - 多周期共振冒烟
- `rolling_volatility_consistency.rs` - 滚动波动率一致性

### 声明式前端契约 (Declarative Front-end)
`factor_provider` / `factor_graph` / `operation` 三层的外部契约。

- `factor_provider_contract.rs` - 因子 provider 边界契约
- `factor_graph_contract.rs` - 因子图契约
- `unified_operation_seam.rs` - 接入统一 operation façade 的两条声明式接缝

### Builder 测试
测试 Builder 模式 API。

- `builder_tests.rs` - Builder 模式测试（22+ 测试用例）

### 价格源测试 (Price Source)
测试不同价格源配置。

- `price_source_tests.rs` - 价格源配置测试

## 公共模块 (Common)

`tests/common/` 目录包含测试共享工具：

- `common/mod.rs` - 模块导出
- `common/golden_loader.rs` - 黄金测试数据加载

## 运行测试

```bash
# 运行所有测试
cargo test -p finkit

# 运行特定类别测试
cargo test -p finkit --test builder_tests
cargo test -p finkit --test repaint_tests
cargo test -p finkit --test property_tests

# 运行 golden 测试
cargo test -p finkit --test golden_tests

# 运行兼容性测试
cargo test -p finkit --test dzh_compat_tests
cargo test -p finkit --test ths_compat_tests

# 量「另一条执行路径还差多远」时必须 --no-fail-fast
# （cargo test 默认在第一个失败 target 上就停，会把单个 target 的红数
#   误当成全量缺口；见 docs/refactor-plan-2026-09-21.md §3.3）
cargo test -p finkit --no-fail-fast
```

## 测试覆盖统计

- **Builder 测试**: 22+ 测试用例（覆盖所有 Builder 类型）
- **Repaint 测试**: 15+ 测试用例（覆盖重绘、快照、克隆）
- **Golden 测试**: 100+ 测试用例（对比 TA-Lib 参考实现）
- **Property 测试**: 50+ 属性测试（proptest 生成）
- **兼容性测试**: 200+ 测试用例（多平台兼容性）

## 测试最佳实践

1. **共享工具最小化**: `tests/common/` 只保留被现有多个测试实际使用的 helper
2. **命名规范**: 测试函数名应清晰表达测试意图
3. **边界测试**: 每个指标至少测试：空输入、单值、warmup 期、正常期
4. **精度测试**: 浮点比较使用 `assert!((a - b).abs() < 1e-10)`
5. **状态测试**: 测试 `reset()`、`clone()`、`save_state()`、`restore_state()`
6. **不要用 `assert_eq!` 比 f64 序列**: 预热区 `NaN` 是合法的，用 `assert_same_series`
7. **门禁要能腐烂检测**: allowlist 型门禁必须双向断言（stale + undeclared），
   否则清理后的陈旧条目会永远「绿」着撒谎
