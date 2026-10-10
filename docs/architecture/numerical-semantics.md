# 数值语义与输入边界合同

- 适用版本：Finkit 0.2.x（workspace 目标版本；已发布资产仍为 0.1.15）
- 状态：**合同文档（单一事实来源）**。任何 NaN/Infinity 处理、零判定带宽、缺失数据
  与 warm-up 策略都必须能从本页追溯到源码常量、门禁脚本和 golden 测试。
- 相关：[公式运行时合同](../formula-runtime-contract.md)、
  [核心契约](../core-contracts.md)、[公式引擎内部](formula-engine.md)。

本页存在的理由：同仓库对缺失值历史上出现过**传播（propagate）**、**拒绝（reject）**、
**忽略（ignore）** 三类不同策略。把三者机械统一成"一个 epsilon / 一套 NaN 规则"会破坏
有明确 parity 依据的既有行为（例如 TA-Lib 的 `TA_IS_ZERO` 带宽与 Alpha158 因子的
`+1e-12` 正则化）。因此本页按**领域**分域治理，而不是全仓文本替换。

## 1. 两个零判定带宽（不是同一个 epsilon）

源码里只有两处地方写下带宽，二者都是具名常量（`core/src/utils.rs`）：

| 覆盖面 | 常量 | 值 | 归属 |
|---|---|---|---|
| indicators / streaming / math kernels | `utils::TA_IS_ZERO_BANDWIDTH` | `1e-8` | TA-Lib 的 `TA_IS_ZERO` |
| formula 运行时 / features / transforms / risk | `utils::NUMERIC_EPSILON` | `1e-15` | finkit 自身 |

规则：

- **不得内联**写成 `1e-8` / `1e-15` 比较。`scripts/check_zero_guard_policy.py`
  （`make`/CI 的 `check-zero-guard`）只允许 `core/src/utils.rs` 一处写出带宽，
  其余出现即为策略回归。
- 把 formula/features 的 guard 拓宽到 TA-Lib 的 `1e-8` **不是 parity 修复**，而是
  行为变更：Alpha158 因子用 Qlib 的 `+1e-12` 正则化分母，正常分母就落在 `~1e-12`，
  在 `1e-8` 带下会被误判为零并整段置零。
- 绝对值形式用 `utils::is_zero(v)`；带符号 / 仅正形式用 `utils::TA_IS_ZERO_BANDWIDTH`。

## 2. NaN / Infinity 策略按领域

"缺失值以 `NaN` 到达"是这里的统一输入约定（`null_policy = "nan"`）；分歧只在
**这一领域的入口是拒绝、传播还是忽略**。

| 领域 | 入口对非有限值 | kernel 行为 | 合同证据 |
|---|---|---|---|
| 批量指标公开入口（`indicators::*` / `math::moving_avg::*` 的分配版与 `_into` 版） | **拒绝**：返回 `InvalidParameter` 语义（`TaError::is_invalid_parameter()`），不 panic | — | `core/tests/edge_case_invalid_input.rs` |
| 数值 kernel（`math::kernels::*`） | 按调用方预处理后的输入计算 | **传播**：用 `f64::total_cmp` 排序，或 `f64::max` 吸收（对齐 TA-Lib `fmax`）；**禁止** `partial_cmp(..).unwrap()` | `core/tests/adx_nan_golden.rs`；`scripts/check_nan_unsafe_ordering.py` |
| 公式运行时 | 由 `FormulaContext` 数据决定 | 前导 warm-up 段为 NaN；外部变量每条路径可解析 | `formula-runtime-contract.md` §3.1–§3.3；`core/tests/formula_differential_tests.rs` |
| 执行/runtime 层 | `NanPolicy::Error` 在运行前拒绝非有限字段 | `NanPolicy::Preserve` 原样保留、`NanPolicy::ForwardFill` 前向填充（只在真正需要时物化） | `core/src/runtime.rs`；`core/tests/runtime_convergence.rs` |
| streaming | `FormingBar` 管理未收盘 bar 回滚纪律 | 重投递同一根 bar 不重复折叠 | `scripts/check_forming_bar_discipline.py`；`core/tests/repaint_tests.rs` |

**关键区分（不得混淆）**：

- 批量入口的"拒绝"发生在**边界校验**（输入契约违规 → 报错）；
- kernel 的"传播"发生在**计算内部**（合法输入里的空洞 → 结果对应位置为 NaN）；
- 二者不是同一层的两种写法，所以不能"统一成一个策略"。

### 2.1 NaN 是值，不是 panic

任何 kernel 都不得因 `NaN` 中止求值。浮点排序用 `f64::total_cmp`（全序，永不返回
`None`）或 `partial_cmp(...).unwrap_or(Ordering::Equal)`；`partial_cmp(..).unwrap()`
在任一操作数为 `NaN` 时 panic，属于违约。门禁扫描
`core/`、`factor-analysis/`、`visualization/`、`cli/`、`ffi/`、`wasm/`。

## 3. 输入边界与缺失数据

### 3.1 数组形状契约

- `open` / `high` / `low` / `close` / `volume` 必须**存在、非空、等长**；
- `amount` 可选；若提供必须与 OHLCV 等长；
- zero-copy / NumPy 入口要求**连续**的一维 `float64`，非连续视图须先
  `numpy.ascontiguousarray`；
- borrowed 输入只在一次同步调用期间有效，不得跨调用保存裸指针。

违约由 `MarketFrame::new` / 入口校验在**执行前**拒绝；执行 `execute_into` 时目标
数量与长度也在 plan 运行前校验，因此被拒调用没有副作用（`core-contracts.md`
"Persistent output"）。

### 3.2 公式源预算

`formula_input_bounds.rs` 钉住 parse 阶段的输入预算：源大小（1 MiB）、括号嵌套
（256）、AST 深度（1024，迭代校验，超深树迭代析构以避免栈溢出）。越界是**类型化
parse 错误**，不是崩溃。

### 3.3 warm-up 与输出长度

- `WarmupPolicy::Nan`（默认）保持行对齐，用 `NaN` 填充 lookback 前缀；
  `WarmupPolicy::Trim` 只返回稳定行。
- 滚动指标（`MA`/`EMA`/`WMA`/`RSI`/`STD`/`BOLL`/`ZSCORE`/`LINEARREG*` 等）的输出以
  **前导 NaN 预热段**开头，这是约定语义。
- **warm-up 可组合性**：把这样的输出继续作为输入（`MA(MA(CLOSE,5),9)`）时，预热段
  **不得**使下游整段变 NaN。递推种子窗口必须从**第一个有限值**起算，且每个内部滑窗
  guard 也要按该起点偏移。**输入完全有限时行为必须逐位不变。**
- 周期大于序列长度、只有 1 根数据等退化输入 → 在所有路径上产出**等长全 NaN 序列**；
  周期为 0 → 在**所有**路径上被拒绝。

## 4. 多路径数值一致

同一公式有多条执行路径（tree / bytecode / 编译计划，以及启用时的 JIT/SIMD）。
**契约**：同一输入必须给出相同结果，且门禁须同时断言**绝对性质**（精确有限值个数、
恒等式、非退化），因为两边一致地返回全 NaN 也会被判为"一致"而放过。

- 路径间容差为绝对 `1e-10`；超出即门禁失败，不允许静默放宽。
- 同统计量的多份实现必须有**互校断言**（`AVGDEV` ≡ `AVEDEV`、`SLOPE` ≡
  `LINEARREG_SLOPE`、`FORCAST` ≡ `LINEARREG`）。
- 见 `formula-runtime-contract.md` §3.2–§3.3 与 `runtime_convergence.rs` 的五条边：
  `batch==streaming`、`full==range`、`allocating==into`、`tree==plan`、`Rust==FFI`。

## 5. 门禁与测试索引

| 关注点 | 门禁脚本 | 主要测试 |
|---|---|---|
| 零判定带宽内联 | `scripts/check_zero_guard_policy.py` | — |
| 浮点排序 NaN 安全 | `scripts/check_nan_unsafe_ordering.py` | — |
| 批量入口拒绝非有限输入 | — | `core/tests/edge_case_invalid_input.rs` |
| ADX/DMI 零带与 NaN golden | — | `core/tests/adx_nan_golden.rs` |
| TA-Lib 语义对齐 | `scripts/gen_talib_numeric_contract.py` / `check_talib_ffi_contract.py` | `core/tests/talib_semantic_contract.rs`、`talib_coverage_matrix.rs` |
| 公式输入预算 | — | `core/tests/formula_input_bounds.rs` |
| 多路径一致性 | — | `core/tests/formula_plan_differential.rs`、`runtime_convergence.rs` |
| forming-bar 回滚纪律 | `scripts/check_forming_bar_discipline.py` | `core/tests/repaint_tests.rs` |
| streaming 注册表契约 | `scripts/check_streaming_registry_contract.py` | `core/tests/runtime_convergence.rs` |

## 6. 变更规则

任何会改变下列之一的工作，必须**先加极端值 / NaN / 平坦行情 golden，再改实现**，
并同时更新本页：

1. 某个入口从"拒绝"改"传播"或反之；
2. 零判定带宽或 epsilon 语义；
3. 输出长度、warm-up 长度或时间方向；
4. kernel 的 NaN 传播/吸收方式；
5. 多路径容差。

每保留一份重复实现，都要有理由和差分测试；每个 epsilon/NaN 策略都要能从代码追溯到
本页。**不以"行数最少"为目标。**
