# Finkit Outperform TA-Lib Architecture 3.0

> 状态：性能优先架构主计划  
> 日期：2026-09-04  
> 目标分支：`perf/outperform-talib-v3-20260904`  
> 基线分支：`fix/talib-performance-plan-20260904`  
> 适用范围：Core / Indicators / Formula / Factor / Batch / Streaming / Python / Node / WASM / Benchmark / Release Gate

## 0. 目标重新定义

Finkit 下一阶段不再以“接近 TA-Lib”为目标，而是把 **公开 API 对公开 API 的真实性能超过 TA-Lib** 设为版本发布前必须满足的硬门槛。

Architecture 3.0 的目标不是只赢少数指标，而是建立一条在单指标、多指标、公式、因子、流式增量场景中都具有持续性能优势的统一计算架构。

核心要求：

1. 正确性先于性能，TA-Lib 对标指标必须保持兼容的 warm-up、NaN、seed、输出长度和数值容差。
2. 性能对比必须使用真实已安装 wheel / package，不能只对比内部 Rust 函数。
3. 输入转换、输出物化、Python FFI、内存分配必须包含在公开 API Benchmark 内。
4. 对于 TA-Lib 已提供的同等能力，Finkit Release Gate 必须超过 TA-Lib，而不是仅达到同量级。
5. 对于 `compute_many()`、Formula DAG、Streaming 等 TA-Lib 缺少统一能力的场景，Finkit 必须建立 2x、5x、10x 以上的结构性优势。

性能统一定义：

```text
speedup = TA-Lib elapsed / Finkit elapsed
```

`speedup > 1.0` 表示 Finkit 更快。

---

# 1. 强制发布性能门槛

## 1.1 Public API Release Gate

使用相同的：

- Python 版本
- NumPy 版本
- CPU runner
- contiguous `float64` 输入
- 数据内容
- warm-up 次数
- 测量轮次
- 输出消费方式

执行 Finkit 已安装 wheel 与 TA-Lib 最新锁定版本的公开 API 对比。

正式 Release 必须同时满足：

| 指标 | Release 最低要求 |
|---|---:|
| 所有正式对标指标几何平均 | `>= 1.15x` |
| 1M bars 几何平均 | `>= 1.20x` |
| 100K bars 几何平均 | `>= 1.15x` |
| 核心 Top 20 指标 | 每项 `>= 1.05x` |
| 全量有效指标 p50 | `>= 1.15x` |
| 全量有效指标 p90 下界 | `>= 1.00x` |
| 任何正式发布对标指标 | 不允许持续低于 `0.95x` |
| Python FFI 对 Core 的额外开销 | 热路径 `< 10%` |

最终目标不是 1.01x 勉强越线，而是让 1.15x～1.50x 成为单指标常态。

## 1.2 Core Kernel Gate

同语义 Core 对 Core：

- 热点 Top 20：几何平均 `>= 1.25x`
- 所有核心对标项：不得稳定慢于 TA-Lib Core
- SMA / EMA / RSI / ATR / MACD / ADX / BBANDS / STOCH / MINMAX / ROC / OBV 等必须纳入长期回归

Core 必须保留足够性能余量，避免 FFI 后退后重新输给 TA-Lib。

## 1.3 Multi-Indicator Gate

对具有共享中间状态的组合：

- ATR + NATR + ADX + DX + PLUS_DI + MINUS_DI：`>= 2.5x`
- EMA12 + EMA26 + MACD + SIGNAL + HIST：`>= 2.0x`
- SMA20 + VAR20 + STD20 + BBANDS：`>= 2.0x`
- MIN/MAX + STOCH + WILLR + AROON：`>= 2.0x`

比较基准为“逐个调用 TA-Lib 同等指标并取得全部结果”。

## 1.4 Formula / Factor Gate

复杂公式：

- 几何平均至少 `>= 2.0x`
- 高频公式集合目标 `>= 3.0x`
- 重复公共子表达式明显的公式目标 `>= 5.0x`

Factor 多因子同数据集：

- 共享窗口/状态的因子组目标 `>= 3.0x`

## 1.5 Streaming / Incremental Gate

TA-Lib 通常需要重复重算数组；Finkit 必须把这一点变成明确优势：

- EMA / RSI / ATR / OBV：每 bar `eval_last()` 相对重复全量 TA-Lib `>= 20x`
- MACD / ADX / rolling stats：`>= 10x`
- 100+ 指标实时状态组合：总体 `>= 10x`

历史越长，增量优势应越明显，而不是随 N 退化。

---

# 2. 当前基线结论

历史基准已经证明两个事实：

1. 旧 Finkit Public Python 指标接口主要输在 `Vec<f64> -> Python list -> Python float -> np.asarray` 的输出物化路径，而不是 Rust 算术。
2. 同一套 Rust Core 在 Formula `eval_zero_copy()` 路径中，MA 和 RSI 已经出现超过 TA-Lib 的结果，EMA 也接近同量级。

因此 Architecture 3.0 的优先级必须是：

```text
FFI / 数据流 / 内存
    > 共享计算 / Kernel ABI
    > Formula DAG / Streaming
    > SIMD / 指令级微优化
```

禁止在 Python 返回路径仍有数量级开销时，优先花大量时间做 5%～10% 的 Kernel 微调。

---

# 3. 总体架构：One Engine, Five Frontends

Architecture 3.0 将所有计算能力统一到一个底层执行模型：

```text
Python API        Rust API        Node/WASM
     \               |               /
      \              |              /
       Standalone / compute_many()
                 |
Formula Compiler / Factor Compiler
                 |
          ComputePlan IR
                 |
    CSE / DCE / Fusion / Liveness
                 |
         Shared State Graph
                 |
          Kernel Registry
                 |
     BufferArena + StateArena
                 |
        Unified Executor
          /      |      \
     Scalar     SIMD   Parallel
                 |
       Streaming Executor
```

五类前端：

1. Standalone Indicator API
2. Batch / `compute_many()`
3. Formula
4. Factor
5. Streaming / `eval_last()`

这些入口只负责描述计算，不再各自拥有不同数值实现。

---

# 4. Kernel ABI 3.0

## 4.1 所有热点指标迁移到 `*_into`

Canonical Kernel：

```rust
pub fn ema_into(
    input: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()>;
```

多输出：

```rust
pub fn macd_into(
    input: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
    macd_out: &mut [f64],
    signal_out: &mut [f64],
    hist_out: &mut [f64],
) -> Result<()>;
```

规则：

- `*_into` 是唯一高性能数值实现。
- 返回 `Array1` / `Vec` 的接口只负责一次性分配并调用 `*_into`。
- Formula / Batch / Python / Streaming 不允许复制一份完整指标算法。
- 能完全覆盖输出的 Kernel 不应提前 `fill(NaN)` 整个数组后再覆盖。
- warm-up 区域由 Kernel 精确写入，避免额外 O(n) 初始化。

## 4.2 Kernel Metadata

每个 Kernel 记录：

```text
inputs
parameters
outputs
warmup
state requirements
streamability
vectorizability
fusion family
alignment requirements
in-place safety
semantic reference
```

Planner 根据 Metadata 自动选择执行策略。

---

# 5. Python FFI：必须成为低开销路径

这是第一优先级。

## 5.1 输入零拷贝

contiguous `numpy.float64`：

```text
PyReadonlyArray
  -> borrowed &[f64]
  -> kernel
```

禁止默认 `.to_vec()`。

非 contiguous / 非 float64：

- fallback 只转换一次
- Benchmark 单独记录 conversion case
- 文档明确 fast path 条件

## 5.2 输出直接 ndarray

正确路径：

```text
allocate PyArray / NumPy-compatible output
    -> obtain mutable slice
    -> release GIL
    -> kernel_into
    -> return same ndarray
```

禁止：

```text
Rust Vec
 -> Python list
 -> millions of PyFloat
 -> np.asarray
```

## 5.3 `out=` / reusable output

公开 API 支持：

```python
out = np.empty_like(close)
finkit.ema(close, 20, out=out)
```

高频循环时消除 Python 层输出重复分配。

多输出：

```python
finkit.macd(close, out=(macd, signal, hist))
```

## 5.4 GIL 策略

- 参数校验和 PyArray 获取：持有 GIL
- 纯 Rust Kernel：释放 GIL
- Kernel 完成后再进入 Python 返回流程

必须加入多线程 Python Benchmark，验证 GIL release 确实生效。

## 5.5 Generator / SSOT

Python Binding 不再手工与 generated 混杂。

SSOT 自动生成：

- Rust registration
- Python signatures
- PyArray output binding
- `.pyi`
- 参数默认值
- multi-output tuple
- docs
- contract test

阻止再次出现 stub 声明和真实函数不一致。

---

# 6. 内存架构 3.0

## 6.1 BufferArena

统一负责 Formula / Batch / Factor 的中间数组。

核心能力：

- size class
- shape-aware reuse
- last-use recycle
- pinned output
- caller-owned output
- arena statistics
- peak bytes 统计

新增两种获取方式：

```rust
take_filled(len, value)
take_overwrite(len)
```

`take_overwrite()` 只允许给“保证写满”的 Kernel，避免每次 cache hit 仍 `fill()` 全数组。

## 6.2 StateArena

Rolling / Streaming State 独立于临时 Buffer：

```text
EMAState
WilderState
RollingSumState
RollingVarState
RollingExtremaState
MACDState
DMIState
```

ComputePlan 用 slot id 访问，避免字符串查找。

## 6.3 消灭 Formula 大数组 Clone

禁止热路径：

```text
FormulaValue::Array(a) => a.clone()
```

改造为：

```rust
enum ValueHandle<'a> {
    Scalar(f64),
    Borrowed(ArrayView1<'a, f64>),
    Arena(BufferId),
    Output(OutputId),
}
```

执行器依据消费者数量和 last-use 决定：

- borrow
- move
- consume
- recycle

只在用户要求持久化中间变量时复制。

## 6.4 Allocation Gate

CI 中记录：

- allocations/call
- bytes allocated/call
- arena hits/misses
- peak live buffers
- clones

热点重复执行目标：临时 heap allocation 下降 `>= 95%`。

---

# 7. Algorithm Kernel Family 重构

超过 TA-Lib 不能只靠 FFI，Core 必须继续留出性能余量。

## 7.1 Moving Average Family

### SMA

- rolling sum O(n)
- 仅一次输入扫描
- period 不使用每元素 modulo 的慢路径
- 窗口索引用增量边界指针

### EMA

- seed 预处理与 steady-state loop 分离
- steady loop 去除重复条件判断
- 常用 alpha 提前计算
- 允许编译器自动向量化的前后处理独立出来

### WMA

使用 O(1) rolling recurrence，避免每个窗口 O(period) 求和。

### DEMA / TEMA

在 ComputePlan 中共享 EMA stage，不重复构造不必要中间数组。

## 7.2 Rolling Statistics Family

SMA / VAR / STDDEV / BBANDS 共用 rolling state：

```text
sum
sum_sq 或稳定 variance state
window queue/ring
```

目标：单次扫描同时得到 mean/variance/stddev/bands。

数值稳定性和 TA-Lib 语义必须通过独立 parity test。

## 7.3 DMI / ATR Family

一次扫描生成共享 primitive：

```text
TR
+DM
-DM
Wilder(TR)
Wilder(+DM)
Wilder(-DM)
```

再派生：

```text
ATR
NATR
PLUS_DI
MINUS_DI
DX
ADX
ADXR
```

禁止 7 个指标各自重新计算 TR/DM/Wilder。

## 7.4 MACD Family

同一计划共享：

```text
EMA(fast)
EMA(slow)
MACD line
EMA(signal)
```

单 `macd()` 可以使用 fused kernel，`compute_many()` 可以共享任意相同 EMA period。

## 7.5 Extrema Family

MIN / MAX / STOCH / WILLR / AROON 使用 monotonic deque / rolling extrema state。

把滚动窗口最值从朴素 O(n * period) 固定到 O(n)。

## 7.6 Price Transform / Elementwise Family

BOP / AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / ROC / MOM / arithmetic 等：

- 单 pass
- branch-light
- contiguous access
- 后续使用 runtime SIMD dispatch

---

# 8. ComputePlan 3.0

## 8.1 Typed IR

建议节点：

```rust
struct PlanNode {
    id: NodeId,
    op: OpCode,
    inputs: SmallVec<NodeId>,
    params: ParamBlock,
    output_shape: Shape,
    warmup: usize,
    state_slot: Option<StateSlot>,
}
```

禁止在 Executor 热路径使用：

- `HashMap<String, ...>`
- 动态字符串函数名
- 重复参数解析

编译阶段全部映射为整数 ID / slot。

## 8.2 CSE

Canonical key：

```text
op + input IDs + normalized parameters + semantic mode
```

同一 `EMA(CLOSE,20)` 在整张图只允许一个节点。

## 8.3 DCE

不输出且无下游消费者的节点完全删除。

## 8.4 Liveness / Register Allocation

中间 Buffer 生命周期分析后分配 slot：

```text
IR value
 -> live interval
 -> buffer slot
 -> reuse
```

目标不是“每个节点返回 Array”，而是像寄存器机一样复用有限 Buffer。

## 8.5 Kernel Fusion

首批固定融合 pattern：

- DMI family
- MACD family
- BBANDS family
- elementwise chains
- rolling z-score
- REF + elementwise expression

每个 fusion 都必须有“fused vs unfused”数值等价测试。

---

# 9. Formula Compiler 3.0

Formula pipeline：

```text
Source
 -> Lexer/Parser
 -> AST
 -> Typed IR
 -> Constant Folding
 -> CSE
 -> DCE
 -> Range/Warmup Analysis
 -> State Lowering
 -> Fusion
 -> Liveness
 -> Buffer Allocation
 -> ComputePlan
```

## 9.1 Direct Builtin Lowering

以下函数不得再经过通用数组 builtin ABI：

- MA/SMA
- EMA
- WMA
- REF
- HHV/LLV
- STD/VAR
- ATR
- RSI
- ROC/MOM
- CROSS
- COUNT/SUM

直接 lower 成 ComputePlan op。

## 9.2 `REF` 优化

固定 offset：

- 不分配 full temporary shift array
- 使用 view/offset node
- 下游 elementwise kernel直接读取偏移 input

只有用户显式要求输出 REF 数组时才 materialize。

## 9.3 Result-only

`eval_last()` 或只请求少数 output：

- 不生成无用历史中间数组
- 可 stateful 的节点只保留 state
- range-aware 节点只计算必要区间

## 9.4 Plan Cache

缓存编译结果：

```text
formula hash
input schema
semantic mode
engine ABI version
compile flags
```

重复运行公式时 parser/compiler 必须接近零成本。

---

# 10. Streaming Engine 3.0

这是 Finkit 超越 TA-Lib 最重要的架构优势之一。

## 10.1 API

```python
plan = finkit.compile(formula)
session = plan.stream()
session.seed(history)

for bar in feed:
    result = session.update(bar)
```

或：

```python
session = finkit.stream([
    ("ema", {"period": 20}),
    ("rsi", {"period": 14}),
    ("macd", {}),
])
```

## 10.2 真正 O(1) / amortized O(1)

以下必须状态化：

- EMA
- Wilder
- RSI
- ATR
- MACD
- OBV
- rolling sum
- rolling var/std
- rolling min/max

禁止 `append_bar()` 后重新对完整历史 `eval()`。

## 10.3 Ring Buffer

固定窗口算子使用 ring buffer，内存复杂度与历史总长度解耦。

## 10.4 Checkpoint

支持：

- snapshot
- restore
- bounded replay

为行情修订、断点恢复、多 symbol session 提供基础。

---

# 11. SIMD / CPU Dispatch

SIMD 放到数据流优化之后实施。

## 11.1 Runtime multiversion

预留：

```text
scalar baseline
SSE2
AVX2/FMA
AVX-512（可选）
ARM NEON
WASM SIMD128
```

官方 portable wheel 使用 runtime CPU feature dispatch，不能依赖 `target-cpu=native` 才获得性能。

## 11.2 SIMD 优先对象

最适合：

- arithmetic
- comparisons
- transforms
- ROC/MOM
- BOP
- prefix/partial reductions
- rolling 初始化阶段
- Formula elementwise chain

递归 EMA/Wilder 主状态不要为了“使用 SIMD”强行并行化。

## 11.3 PGO / LTO

Release build 评估：

- thin/full LTO
- codegen-units
- PGO
- panic strategy

所有设置必须用真实 Benchmark 决定，禁止只按理论开启。

---

# 12. 并行架构

## 12.1 不优先拆单条递归时间序列

对 EMA/RSI/Wilder 等串行状态，不做低收益线程拆分。

## 12.2 优先并行维度

- symbols
- independent factors
- independent ComputePlans
- DAG independent branches

## 12.3 Batch API 改造

当前 `Fn(&[f64]) -> Result<Vec<f64>>` 风格逐步迁移为：

```rust
Fn(&InputSet, &mut OutputSet, &mut ExecutionContext) -> Result<()>
```

由 executor 统一分配/reuse buffer，job 不再自行创建 `Vec`。

---

# 13. Common-period Specialization

为了进一步超过 TA-Lib，可为常见 period 增加可验证的 specialization：

```text
5, 6, 9, 10, 12, 14, 20, 24, 26, 30, 60, 120, 250
```

适用场景：

- rolling window pointer layout
- fixed-window small loop
- ring index
- coefficient constants

原则：

- generic path 必须保留
- specialization 由 Benchmark 证明收益后才启用
- 不能牺牲可维护性换取不足 3% 的复杂度

---

# 14. Correctness First：TA-Lib Semantic Contract

超过 TA-Lib 的前提是输出仍然正确。

必须固定：

- lookback
- leading NaN
- EMA seed
- Wilder seed
- unstable period
- population/sample variance convention
- divide by zero
- SAR output contract
- multi-output warmup
- NaN propagation
- invalid parameter behavior

每个对标指标执行：

```text
shape equality
finite mask equality
lookback equality
abs/rel tolerance
edge case suite
randomized differential test
```

性能失败和语义失败必须分别报告，禁止通过扩大 tolerance 掩盖错误。

---

# 15. Benchmark 3.0

## 15.1 五层 Benchmark

### A. Kernel microbenchmark

定位纯算法差异。

### B. Rust public API

包含 allocation/wrapper 成本。

### C. Python installed wheel

最终单指标 Release Gate。

### D. ComputePlan / Formula / Factor

验证 CSE/fusion/memory reuse。

### E. Streaming

验证每-bar latency、吞吐和状态内存。

## 15.2 数据规模

至少：

```text
1K
10K
100K
1M
5M（nightly）
```

不能只用 1M，否则掩盖小数组 FFI/调度开销。

## 15.3 数据类型

- regular OHLCV
- flat series
- trending
- high volatility
- zeros where legal
- NaN edge inputs

正确性 suite 与 performance suite 分开。

## 15.4 统计

每个 case：

- warmups
- >= 7 measured samples
- median
- p90
- p95
- MAD / variance

Release Gate 使用 median + 稳定性阈值，不依据单次最好成绩。

## 15.5 Benchmark 输出

自动生成：

```text
indicator
size
Finkit median
TA-Lib median
speedup
correctness
allocations
peak bytes
winner
```

CI artifact 保存 raw JSON + Markdown summary。

---

# 16. 防性能回退机制

PR Gate：

- 只跑 Top 20 + 10K/100K
- 不允许 >5% 稳定退化

Nightly Gate：

- 全量指标
- 1M/5M
- Formula
- compute_many
- Streaming
- allocation counters

Release Gate：

- fresh build
- installed wheel
- locked TA-Lib reference
- semantic contract 全绿
- performance hard gate 全绿

任何一项不满足，版本不得标为 production release。

---

# 17. 分阶段实施顺序

## Phase 0 — Correctness Closure

目标：先把当前 `TA-Lib semantic contract` 完全收口。

任务：

- KAMA/MACD/ADOSC/TRANGE/SAR/ATR/BOLL 等历史差异逐项清理
- public API / `.pyi` / Registry 一致
- Core full regression green
- installed wheel correctness green

Exit：TA-Lib semantic suite 100% green。

## Phase 1 — Public API Outperform

目标：公开 Python API 从数量级落后直接进入超过 TA-Lib 区间。

任务：

- 直接 PyArray output
- NumPy input borrow
- GIL release
- `out=`
- Top 20 `*_into`
- remove Python list materialization

Exit：Top 20 Public API 几何平均 `>= 1.10x`，任何 Top 20 不低于 `0.95x`。

## Phase 2 — Core > TA-Lib

任务：

- family kernels
- WMA recurrence
- rolling stats
- monotonic deque
- fused MACD/DMI/BBANDS
- remove redundant fill/copy

Exit：Top 20 Core geomean `>= 1.25x`，Public geomean `>= 1.15x`。

## Phase 3 — Unified ComputePlan

任务：

- Typed IR
- NodeId/slot runtime
- CSE
- DCE
- liveness
- BufferArena 3.0
- compute_many

Exit：共享指标组合 `>= 2x` TA-Lib sequential calls。

## Phase 4 — Formula / Factor 3.0

任务：

- compiler lowering
- REF/view node
- fusion
- plan cache
- result-only
- remove array clones

Exit：Formula geomean `>= 2x`，高复用公式 `>= 3x`。

## Phase 5 — Streaming 3.0

任务：

- StateArena
- update kernels
- ring buffer
- checkpoint/replay
- true `eval_last()`

Exit：核心 streaming cases `>= 10x`，EMA/RSI/ATR 等 `>= 20x`。

## Phase 6 — SIMD / PGO / Final Tuning

任务：

- profiling
- SIMD dispatch
- compile tuning
- branch reduction
- cache layout

Exit：正式 Release Gate：Public geomean `>= 1.15x`；1M geomean `>= 1.20x`；Top 20 每项 `>= 1.05x`。

---

# 18. 建议目录重构

建议最终逐步形成：

```text
core/src/
  kernel/
    moving/
    momentum/
    volatility/
    volume/
    extrema/
    transforms/
  state/
    ema.rs
    wilder.rs
    rolling_sum.rs
    rolling_var.rs
    rolling_extrema.rs
  plan/
    ir.rs
    planner.rs
    cse.rs
    fusion.rs
    liveness.rs
    schedule.rs
  runtime/
    executor.rs
    buffer_arena.rs
    state_arena.rs
    cpu_dispatch.rs
  streaming/
    session.rs
    checkpoint.rs
  formula/
    parser/
    compiler/
    lowering/
  factor/

ffi/python-binding/
  generated/
  numpy_io.rs
  output.rs
  module.rs

benchmarks/
  kernel/
  public_python/
  compute_many/
  formula/
  streaming/
  reference/
```

不要求一次移动全部目录，优先保证行为和构建稳定，再逐步收敛结构。

---

# 19. 关键工程原则

1. **不得为 Benchmark 写特殊作弊路径。** 优化必须是生产代码真实路径。
2. **Public API Benchmark 是最终判定。** Core 快但 Python 慢仍算失败。
3. **避免重复算法实现。** 一个指标的 standalone / formula / streaming 共享 kernel/state。
4. **先消除 O(n) 复制，再考虑 5% SIMD。**
5. **跨指标共享才是 Finkit 超越 TA-Lib 的长期护城河。**
6. **Streaming 是核心产品能力，不只是 Benchmark 项目。**
7. **SSOT 必须成为 API、文档、binding、测试、benchmark 的统一来源。**
8. **所有优化必须可测量。** 每个性能 PR 都附 before/after。
9. **任何优化不得破坏 TA-Lib 对标语义。**
10. **只有通过语义 Gate + 性能 Gate 才允许发布。**

---

# 20. 最终验收标准

Architecture 3.0 完成不以“代码写完”为准，而以以下条件全部成立为准：

- [ ] TA-Lib semantic contract 全绿
- [ ] Python Public API 不再经过 list materialization
- [ ] NumPy contiguous float64 默认零拷贝输入
- [ ] Top 20 全部迁移 canonical `*_into`
- [ ] Formula 热路径无整数组隐式 clone
- [ ] BufferArena / StateArena 纳入统一 executor
- [ ] `compute_many()` 真实共享状态而不是循环调用
- [ ] Formula 编译到 ComputePlan
- [ ] `eval_last()` 真实增量而不是全量重算
- [ ] Public API geomean `>= 1.15x` TA-Lib
- [ ] 1M geomean `>= 1.20x` TA-Lib
- [ ] Top 20 每项 `>= 1.05x` TA-Lib
- [ ] Multi-indicator shared workloads `>= 2x`
- [ ] Formula workloads `>= 2x`
- [ ] Streaming workloads `>= 10x`
- [ ] Allocation hotpath 下降 `>= 95%`
- [ ] Release wheel benchmark 通过
- [ ] Linux / Windows / macOS 语义回归通过
- [ ] Benchmark 原始数据和报告作为 CI artifacts 保存

只有上述条件全部满足，才把这一轮定义为“Finkit 性能全面超过 TA-Lib”。

---

# 21. 实施优先级结论

按收益/风险排序，下一步应严格执行：

```text
P0  TA-Lib semantic contract 全绿
P0  Python direct ndarray output + borrowed NumPy input
P0  Top 20 canonical *_into kernels
P0  public installed-wheel performance gate

P1  Formula ValueHandle / 消除 clone
P1  BufferArena overwrite path
P1  DMI / MACD / BBANDS / extrema family fusion
P1  compute_many + CSE

P2  StateArena + true streaming eval_last
P2  Formula compiler -> ComputePlan
P2  register/slot executor

P3  SIMD runtime dispatch
P3  LTO/PGO
P3  common-period specialization
```

这一顺序的核心目的只有一个：**先让公开单指标 API 真实超过 TA-Lib，再利用统一执行计划、共享状态和增量计算把优势扩大到 TA-Lib 架构难以覆盖的场景。**
