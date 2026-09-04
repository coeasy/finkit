# Finkit Performance Architecture 2.0 全面升级改造计划

> 状态：Architecture 2.0 主计划  
> 日期：2026-09-04  
> 目标分支：`fix/talib-performance-plan-20260904`  
> 适用范围：Finkit Core / Formula / Python FFI / Batch / Streaming / Benchmark / Release Gate  
> 前序文档：`docs/finkit-vs-talib-performance-optimization-plan.md`、`docs/finkit-vs-talib-expanded-benchmark-results.md`

本文档是 Finkit 新一轮性能与架构升级的主计划。前序 TA-Lib 性能计划保留作为问题发现、基线和历史记录；Architecture 2.0 不再以“逐个指标补优化”为中心，而是把 **Standalone、Batch、Formula、Streaming 四种入口统一到同一个执行引擎**，从根本上解决 Python ABI、重复计算、重复分配、Formula 中间结果、增量重算和多指标共享状态问题。

---

## 1. 最终目标

Finkit 的长期性能目标明确为：

1. **单指标性能 ≈ / > TA-Lib**
2. **多指标联合计算明显快于重复调用 TA-Lib**
3. **复杂公式明显快于等价的多次 TA-Lib + Python 组合调用**
4. **增量计算远快于每根新 Bar 重复全量调用 TA-Lib**
5. 在取得性能优势的同时，保持 TA-Lib 兼容语义、国内外公式系统扩展能力、稳定 ABI 和跨语言可安装能力

Architecture 2.0 不追求某几个 Benchmark 的局部胜利，而是建设一个可以持续扩展指标、公式和因子的高性能计算底座。

### 1.1 硬性性能目标

性能统一使用：

```text
speedup = TA-Lib time / Finkit time
```

`speedup > 1.0` 表示 Finkit 更快。

| 场景 | Architecture 2.0 硬性目标 | Stretch Goal |
|---|---:|---:|
| 单指标，100K / 1M bars | 几何均值 >= 1.0x | 1.1x ~ 1.5x |
| 单指标 p90 | >= 0.9x | >= 1.0x |
| 核心指标最差项 | 不低于 0.8x | 不低于 0.9x |
| 相关多指标组合 | >= 2.0x | 3x ~ 5x |
| 复杂 Formula / DAG | >= 2.0x | 3x ~ 5x |
| Incremental / eval_last | >= 10x | 20x ~ 100x |
| 热路径重复调用分配次数 | 下降 >= 90% | 接近零临时堆分配 |
| Result-only 峰值额外内存 | <= 1.2x 理论必要量（可行场景） | 接近输入 + 输出下界 |

这些门槛必须建立在 **公开 API 对公开 API、Core 对 Core** 的公平比较上，禁止通过隐藏数据转换成本获得虚假优势。

---

## 2. 当前真实基线

截至 2026-09-04，`fix/talib-performance-plan-20260904` 最新验证状态如下：

- Python Binding 编译：通过
- Core regression：通过
- Python/API/TA-Lib 迁移脚本：已进入真实编译和测试链路
- Core Into / Shared-State Hotpath 迁移：已进入流水线
- Formula runtime reuse：已进入流水线
- Formula result-only fast path：已进入流水线
- 当前阻塞点：**TA-Lib semantic contract 尚未全绿**
- 因 semantic contract 未通过，生成源码自动提交和后续 Release Gate 不应被视为完成

因此 Architecture 2.0 的 Phase 0 不是重新发明已有工作，而是先把当前语义契约彻底收口，建立可靠的正确性地基。

### 2.1 已知性能事实

历史扩展 Benchmark 已经证明：

- 旧公开 Python API 的主要瓶颈不是 Rust 算术本身，而是 `Vec<f64> -> Python list/float -> NumPy ndarray` 的对象物化路径
- 1M bars 时大量单输出指标集中在约 230–250ms，MACD 三输出约 3 倍，显示 Python 对象构造是主导成本
- Formula `eval_zero_copy()` 的 MA/EMA/RSI 已经能达到接近甚至超过 TA-Lib 的 Core 级表现

这说明 Architecture 2.0 应优先解决 **数据流、ABI、内存复用、共享状态和执行计划**，而不是一开始就把大量精力投入 SIMD 微优化。

---

## 3. Architecture 2.0 总体架构

核心原则：

> **一个执行引擎，四种入口。**

```text
Standalone API        compute_many()
      \                  /
       \                /
        FormulaPlan    StreamingSession
             \          /
              \        /
        Typed ComputePlan / StateGraph
                    |
        +-----------+-----------+
        |                       |
  Kernel Registry         Shared State Registry
        |                       |
        +-----------+-----------+
                    |
          Liveness / Buffer Assignment
                    |
          BufferArena / RollingState
                    |
              Unified Executor
                    |
       CPU Scalar Baseline Backend
                    |
       optional SIMD / Parallel Backend
```

四种入口最终不能拥有四套计算语义：

- Standalone API：兼容 TA-Lib 风格、用户简单直接调用
- `compute_many()`：多指标一次规划、共享中间状态
- `FormulaPlan`：公式编译成同一个 ComputePlan
- `StreamingSession`：同一个计划使用持久化 RollingState 增量更新

---

## 4. 统一执行模型

### 4.1 Kernel：唯一数值实现

每一个热点指标最终必须收敛到 canonical kernel ABI：

```rust
fn sma_into(
    input: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()>;

fn macd_into(
    input: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
    macd: &mut [f64],
    signal_out: &mut [f64],
    hist: &mut [f64],
) -> Result<()>;
```

原则：

- `*_into` 是性能路径和语义实现的唯一来源
- `Vec` 返回 API 只作为 convenience wrapper
- Batch / Formula / Streaming 不再分别维护数值实现
- 多输出指标直接写入预分配 buffer
- 不允许热点路径先创建 Vec 再复制到最终输出

### 4.2 State：增量计算的唯一状态实现

为可状态化指标提供统一 Stateful Operator：

```rust
trait IndicatorState {
    type Input;
    type Output;

    fn update(&mut self, input: Self::Input) -> Self::Output;
    fn reset(&mut self);
}
```

后续可扩展：

```rust
fn snapshot(&self) -> StateSnapshot;
fn restore(&mut self, snapshot: &StateSnapshot) -> Result<()>;
```

优先状态化：

- EMA
- SMA / rolling sum
- RSI
- ATR / TRANGE
- MACD
- OBV
- ADX / +DI / -DI
- rolling variance / stddev / covariance / correlation
- rolling min/max

### 4.3 ComputePlan：共享的执行计划

建议新增统一计划层：

```rust
struct ComputePlan {
    nodes: Vec<PlanNode>,
    outputs: Vec<NodeId>,
    schedule: Vec<NodeId>,
    buffers: BufferAssignment,
    warmup: WarmupPlan,
    state_slots: Vec<StateSlot>,
}
```

`PlanNode` 不应只代表“指标函数”，而应代表可共享的计算原语和状态，例如：

```text
INPUT(close)
TRANGE(high, low, close)
EMA(close, 12)
EMA(close, 26)
SUB(ema12, ema26)
EMA(macd, 9)
ROLLING_SUM(volume, 20)
ROLLING_STD(close, 20)
```

这样 MACD、BBANDS、ADX 等组合指标可以共享底层状态，而不是互相调用完整指标函数。

---

## 5. 单一事实源 SSOT 2.0

当前指标 Registry、Rust、Python Binding、`.pyi`、文档、测试和 Benchmark 必须进一步统一。

每个公开指标至少维护以下元数据：

```yaml
name: macd
aliases: []
inputs:
  - close
parameters:
  fastperiod:
    type: int
    default: 12
outputs:
  - macd
  - signal
  - hist
warmup: ...
semantics:
  reference: talib
  nan_policy: ...
  seed_policy: ...
ffi:
  python: ...
  javascript: ...
benchmark:
  enabled: true
  sizes: [1000, 10000, 100000, 1000000]
```

从 SSOT 生成或验证：

- Rust public registration
- Python module export
- Python `.pyi`
- JavaScript/WASM 声明
- docs API table
- API contract tests
- TA-Lib semantic tests
- benchmark case registry

### 5.1 明确语义策略

以下行为必须显式记录，不能散落在实现中：

- warm-up 长度
- NaN 传播与前导 NaN
- EMA seed
- Wilder smoothing seed
- variance/stddev 使用 population 还是 sample convention
- divide-by-zero
- unstable period
- multi-output warmup
- SAR 的公开输出与内部状态
- 参数非法范围

任何性能优化不得偷偷改变上述语义。

---

## 6. 内存模型升级

### 6.1 BufferArena 2.0

`BufferArena` 从“减少部分申请”升级为统一执行引擎的内存分配器。

职责：

- 按 shape / dtype 获取可复用 buffer
- 依据 liveness 回收临时 buffer
- 输出 buffer pin 住，不参与提前复用
- 支持调用方提供 output buffer
- 收集 allocation / reuse / peak-live-buffer 指标

建议的数据结构：

```rust
struct BufferArena {
    free_f64: Vec<Vec<f64>>,
    live: HashMap<BufferId, BufferHandle>,
    stats: ArenaStats,
}
```

后续优化为 size class / slab，不要求首阶段一步到位。

### 6.2 Liveness + Buffer Assignment

Formula/Batch 编译结束后执行：

```text
Node graph
  -> last-use analysis
  -> liveness intervals
  -> buffer assignment
  -> execution schedule
```

当某个中间结果最后一次使用结束后，buffer 立即复用。

例如：

```text
A = EMA(CLOSE, 12)
B = EMA(CLOSE, 26)
C = A - B
D = EMA(C, 9)
OUT = C - D
```

`A`、`B` 在 `C` 生成后即可释放；最终只保留 C/D/OUT 必要生命周期。

### 6.3 Move/Consume 代替 Clone

Formula VM 和 ComputeIR 必须建立 ownership-aware execution：

- 单消费者临时节点：move/consume
- 多消费者节点：共享只读引用
- 只有必须持久化/跨执行生命周期时才 clone

CI 中增加 allocation counter，防止以后重新引入隐性 clone。

---

## 7. Standalone 单指标优化

单指标不是单独一套引擎，而是统一 Kernel 的最薄入口。

### 7.1 Rust 路径

```text
public indicator()
  -> validate args
  -> allocate output once
  -> kernel_into()
  -> return
```

### 7.2 Python 路径

必须变为：

```text
NumPy contiguous float64
  -> borrow slice where safe
  -> release GIL
  -> Rust kernel writes ndarray-compatible output
  -> ndarray directly returned
```

禁止：

```text
Rust Vec
  -> Python list
  -> Python float objects
  -> np.asarray
```

非 contiguous / 非 float64 输入可以使用一次性转换 fallback，但 Benchmark 必须分别记录：

- contiguous zero/low-copy case
- conversion case

### 7.3 `out=` / `compute_into`

为循环和高频调用增加可复用输出：

```python
finkit.ema(close, 20, out=buffer)
```

或 native API：

```python
plan.eval_into(inputs, outputs)
```

目标是热路径重复计算时无需每次分配新数组。

---

## 8. Multi-Indicator Planner

新增 Finkit-native API：

```python
result = finkit.compute_many(
    {"high": high, "low": low, "close": close, "volume": volume},
    [
        ("atr", {"period": 14}),
        ("adx", {"period": 14}),
        ("plus_di", {"period": 14}),
        ("minus_di", {"period": 14}),
        ("macd", {}),
    ],
)
```

### 8.1 必须共享的计算族

#### DMI / ATR family

共享：

- previous close
- true range
- +DM / -DM
- Wilder smoothing

覆盖：

- TRANGE
- ATR
- NATR
- +DI
- -DI
- DX
- ADX
- ADXR

#### EMA / MACD family

共享：

- 相同 period EMA state
- MACD line

覆盖：

- EMA
- MACD
- MACDEXT / MACDFIX（后续）
- Formula 中相同 EMA 子表达式

#### SMA / variance / BBANDS family

共享：

- rolling sum
- rolling sum of squares 或稳定 variance state

覆盖：

- SMA
- VAR
- STDDEV
- BBANDS
- Formula 中 MA/STD/BOLL

#### extrema family

用 monotonic deque 或统一 rolling extrema state：

- MIN/MAX
- MININDEX/MAXINDEX
- STOCH
- WILLR
- AROON 等适用节点

### 8.2 Planner 去重规则

相同节点由 canonical key 去重：

```text
operator + input node ids + normalized parameters + semantic version
```

例如公式中出现 5 次 `EMA(CLOSE, 20)`，计划中只能存在一个节点。

---

## 9. Formula Compiler / VM 2.0

Formula 不再是“解释器调用多个数组函数”，而是 Compile-to-Plan。

完整编译流水线：

```text
Formula source
  -> AST
  -> type / shape inference
  -> Typed ComputeIR
  -> normalize
  -> constant folding
  -> CSE
  -> DCE
  -> warmup analysis
  -> kernel selection
  -> safe fusion
  -> liveness
  -> buffer assignment
  -> ComputePlan
  -> Executor
```

### 9.1 CSE

例如：

```text
A := EMA(CLOSE,20);
B := EMA(CLOSE,20) - EMA(CLOSE,60);
C := CROSS(CLOSE, EMA(CLOSE,20));
```

`EMA(CLOSE,20)` 只能计算一次。

### 9.2 DCE / Result-only

用户只请求最后一个结果时：

- 不需要返回的中间变量不 materialize 为最终数组
- 支持只保留 state / last value 的节点不生成完整历史数组

### 9.3 Safe Kernel Fusion

仅针对已经证明等价的热点模式：

- MACD family
- BBANDS family
- DMI family
- rolling z-score family
- REF + arithmetic / elementwise chain

融合必须有独立 semantic equivalence test，禁止为了 Benchmark 直接改语义。

### 9.4 Formula Plan Cache

缓存 key 至少包含：

```text
formula source hash
+ input schema
+ compile options
+ semantic version
+ engine version
```

热路径重复执行时禁止重复 parser/compiler 成本。

---

## 10. Streaming / Incremental Engine

这是 Finkit 相对 TA-Lib 最应建立的结构性优势。

目标调用：

```python
plan = finkit.FormulaPlan.compile(formula)
plan.seed(history)

for bar in stream:
    plan.append_bar(bar)
    value = plan.eval_last()
```

或：

```python
session = finkit.StreamingSession(specs)
session.seed(history)
result = session.update(bar)
```

### 10.1 复杂度目标

禁止：

```text
append_bar
  -> append full arrays
  -> recalculate all history
  -> slice last value
```

正确路径：

```text
append_bar
  -> update affected state nodes
  -> propagate changed scalar/state values
  -> emit requested last outputs
```

目标：

- EMA/RSI/ATR/MACD/OBV：接近 O(1)
- rolling min/max：amortized O(1)
- 某些窗口统计：O(1) 或 O(log W)
- 不再随历史总长度 N 线性增长

### 10.2 Ring Buffer

窗口函数使用固定容量 ring buffer，不因 append 持续复制历史数组。

### 10.3 修订历史数据

需要支持现实行情中的补数据/纠错：

- 当前 bar 修订：局部 state 修正
- 短历史修订：bounded replay
- 长历史修订：checkpoint + replay

不要求任意历史位置完全 O(1)，但禁止默认整段全量重建。

---

## 11. 并行与 SIMD 策略

顺序必须是：

```text
语义正确
-> 消除 Python 对象物化
-> 消除重复分配
-> 共享状态 / DAG
-> 增量状态化
-> profiling
-> 并行 / SIMD
```

### 11.1 并行优先级

优先并行：

1. 多 symbol
2. 多独立 FormulaPlan
3. ComputePlan 中无依赖的粗粒度分支
4. 足够大的单个 kernel

避免：

- 小数组每个指标都开 Rayon task
- nested parallelism
- Python 层线程 + Rust 内部线程池叠加 oversubscription

统一线程池和阈值策略。

### 11.2 SIMD

只对 Profiling 证明占比高的 primitive 做 SIMD：

- elementwise arithmetic
- difference / abs / max/min
- rolling primitive 的可向量化部分
- dot-like reductions

要求：

- scalar fallback
- feature detection
- scalar/SIMD equivalence test
- 不得牺牲 NaN/边界语义

---

## 12. Python / FFI 2.0

### 12.1 公共 API 分层

保留 TA-Lib-compatible API：

```python
finkit.ema(close, timeperiod=20)
finkit.macd(close, ...)
```

新增 Finkit-native 高性能 API：

```python
finkit.compute_many(inputs, specs)

plan = finkit.FormulaPlan.compile(source)
plan.eval(inputs)
plan.eval_into(inputs, outputs)
plan.eval_last()
plan.append_bar(bar)

session = finkit.StreamingSession(specs)
session.seed(history)
session.update(bar)
```

### 12.2 GIL

只在参数转换与 Python 对象构造阶段持有 GIL；Rust 数值执行阶段释放 GIL。

### 12.3 输出类型

- 数值数组：直接 NumPy ndarray
- 多输出：tuple / typed result 中每项都是 ndarray
- index 类输出：正确 integer ndarray
- scalar metadata：Python scalar

### 12.4 跨语言一致性

Python 首先作为性能基准入口，但 Architecture 2.0 的 Kernel / Plan / State 不能依赖 Python。

后续 Node/WASM/Java/.NET/Go 均复用相同：

```text
Core Kernel + ComputePlan + StateGraph
```

而不是重新实现指标。

---

## 13. 推荐代码结构

目标结构：

```text
core/src/
  kernel/
    mod.rs
    moving.rs
    momentum.rs
    volatility.rs
    volume.rs
    rolling.rs
    elementwise.rs

  state/
    mod.rs
    ema.rs
    wilder.rs
    rolling_sum.rs
    rolling_stats.rs
    extrema.rs
    dmi.rs

  plan/
    mod.rs
    node.rs
    builder.rs
    dependency.rs
    cse.rs
    liveness.rs
    buffer_assignment.rs
    schedule.rs

  formula/
    parser/...
    compile.rs
    compute_ir.rs
    lowering.rs

  executor.rs
  batch.rs
  indicators/
```

职责收敛：

- `kernel/`：数值真源
- `state/`：增量真源
- `plan/`：依赖、共享、生命周期和调度
- `formula/`：语法和编译，最终 lower 到 plan
- `executor.rs`：统一执行
- `batch.rs`：薄 facade
- `indicators/`：公开 standalone wrapper，不再复制核心算法

迁移过程中不要求一次重命名全部文件，但最终职责必须收敛到上述边界。

---

## 14. Benchmark Architecture 2.0

以后 Release Gate 必须包含四类一等公民 Benchmark。

### Suite A：Standalone

覆盖至少：

- SMA
- EMA
- RSI
- ATR
- MACD
- BBANDS
- ADX
- STOCH
- KAMA
- TRANGE
- OBV
- ADOSC

规模：

```text
1K / 10K / 100K / 1M
```

可增加 10M 作为 scaling test，不要求每次 PR 都跑。

### Suite B：Multi-indicator

至少三组：

```text
DMI set:
ATR + NATR + ADX + +DI + -DI

Trend set:
SMA20 + SMA60 + EMA12 + EMA26 + MACD + BBANDS

Market feature set:
trend + momentum + volatility + volume 15~30 outputs
```

对手基线是连续多次调用 TA-Lib，而不是一个人为放慢的参考实现。

### Suite C：Formula / DAG

至少覆盖：

- 多次重复公共子表达式
- MA/EMA/STD/BOLL 混合
- ATR/DMI 混合
- REF/CROSS/HHV/LLV
- 20+ nodes 复杂公式
- 50+ nodes 压力公式

比较：

```text
Finkit compiled FormulaPlan
vs
等价 TA-Lib + NumPy/Python 组合调用
```

### Suite D：Incremental

场景：

- 先 seed 10K / 100K / 1M bars
- 连续 append 1K / 10K bars
- 每次获取最新指标/公式值

比较：

```text
Finkit update/eval_last
vs
每次调用 TA-Lib 重新计算历史窗口/完整序列的典型用户路径
```

同时报告绝对 latency，不只报告倍数。

### 14.1 Benchmark 方法

- 固定随机种子
- warm-up 后多次运行
- 使用 median / p90，而不是单次最小值
- compile/cold-start 与 warm execution 分开
- 输出 CPU / OS / Python / Rust / TA-Lib / Finkit 版本
- 保存 JSON artifact，支持跨 commit 趋势比较
- 避免同时运行会干扰 CPU 的工作流任务

---

## 15. Allocation / Memory Gate

仅看时间不够。

新增指标：

- allocations / call
- bytes allocated / call
- BufferArena reuse ratio
- peak live buffers
- peak RSS（长 benchmark）
- Python object creation count（可测场景）

关键回归示例：

- Formula 优化后时间变快，但临时内存翻倍：不通过
- 单指标 `out=` 仍每次创建同大小 Vec：不通过
- `compute_many()` 只是循环 standalone：不通过架构验收

---

## 16. Release Gate 2.0

顺序必须固定：

```text
1. format / generated source stability
2. Rust compile
3. Core regression
4. TA-Lib semantic contract
5. Build real wheel
6. Install wheel in clean environment
7. Python public API contract
8. Standalone benchmark gate
9. Multi-indicator benchmark gate
10. Formula benchmark gate
11. Incremental benchmark gate
12. Allocation/memory regression gate
13. package/version/docs consistency
```

**正确性 Gate 必须先于性能 Gate。**

禁止为了让性能测试绿而：

- 降低精度
- 缩短 warmup
- 改 NaN 行为
- skip 失败指标
- 将真实 public API 成本移出计时范围
- 临时降低阈值掩盖回归

---

## 17. 分阶段实施计划

## Phase 0 — Correctness Baseline

### 目标

把现有 TA-Lib 性能分支先变成可靠地基。

### 工作

1. 修完当前 TA-Lib semantic contract 真实失败项
2. 所有 Core regression 保持全绿
3. 修复 public API / `.pyi` / registry 漂移
4. 真实 wheel build + clean install
5. BBANDS/STOCH/stddev/correl/SAR 等 public contract 全绿
6. semantic contract 覆盖 warmup/mask/value
7. Registry SSOT 2.0 元数据规范确定

### Exit Gate

- Core green
- semantic green
- wheel green
- public API green
- 无 skip/mock

---

## Phase 1 — Canonical Kernel ABI + Memory Baseline

### 目标

彻底消除旧 Python 230–250ms 对象物化平台，并让所有热点指标拥有 `*_into` 真路径。

### 工作

1. 12 个核心指标先迁移 canonical `*_into`
2. Vec API 降级为 wrapper
3. Python 直接 ndarray 输出
4. contiguous float64 借用
5. GIL release
6. BufferArena 2.0
7. `out=` / eval_into
8. allocation benchmark

### Exit Gate

- 12 核心指标 public Python API 不再有 Python float-list materialization
- Standalone 100K/1M 几何均值 >= 0.8x，冲刺 >=1.0x
- 重复调用 allocation 下降 >=90%

Phase 1 允许部分指标尚未全面超越 TA-Lib，但必须证明架构成本已消失。

---

## Phase 2 — Shared-State Multi-Indicator Engine

### 目标

从“每个指标独立扫描”升级为“一个依赖图共享计算”。

### 工作

1. `ComputePlan` / `PlanNode`
2. canonical node key
3. dependency graph
4. shared-state registry
5. `compute_many()`
6. DMI family fusion/shared state
7. EMA/MACD family
8. SMA/STD/BBANDS family
9. extrema family
10. liveness + buffer assignment

### Exit Gate

- `compute_many()` 不再等价于 standalone loop
- 三组 Multi benchmark >=2x repeated TA-Lib
- memory 不随指标数线性重复膨胀

---

## Phase 3 — Formula Compiler 2.0

### 目标

让 Formula 成为 ComputePlan 的高级前端，而不是独立执行世界。

### 工作

1. Typed ComputeIR 完善
2. Formula lowering 到 PlanNode
3. constant folding
4. CSE
5. DCE
6. warmup/shape analysis
7. liveness/buffer reuse
8. move/consume intermediate
9. safe kernel fusion
10. FormulaPlan cache
11. eval / eval_into / result-only 统一

### Exit Gate

- 复杂 Formula 100K/1M >=2x equivalent TA-Lib+NumPy
- 重复子表达式只执行一次
- 20/50 nodes 压力公式无显著临时内存爆炸

---

## Phase 4 — Streaming / Incremental 2.0

### 目标

形成 Finkit 最明显的结构性性能优势。

### 工作

1. IndicatorState abstraction
2. EMA/Wilder/rolling state
3. Formula persistent state graph
4. `seed()`
5. `append_bar()`
6. `eval_last()`
7. `StreamingSession`
8. ring buffer
9. checkpoint/replay
10. incremental semantic differential tests

### Exit Gate

- 常用状态指标 append complexity 与总历史长度基本解耦
- Incremental benchmark >=10x repeated TA-Lib
- 长历史场景冲刺 20x~100x
- streaming 输出与 full batch 输出严格语义等价

---

## Phase 5 — Parallel / SIMD / Advanced Fusion

### 目标

在正确架构之上吃掉剩余 CPU 热点。

### 工作

1. profile top kernels
2. symbol-level parallelism
3. independent graph branch scheduling
4. large-array threshold
5. unified Rayon/thread-pool policy
6. targeted SIMD
7. advanced family fusion
8. cache locality / SoA-AoS review

### Exit Gate

- 单指标最终 geomean >=1.0x
- p90 >=0.9x
- 无核心指标严重倒退
- Multi/Formula 优势继续保持
- 线程数扩大时 scaling 合理、无 oversubscription

---

## Phase 6 — Public API Stability / Release

### 目标

把 Architecture 2.0 从实验分支变成可长期维护的正式基础设施。

### 工作

1. API compatibility matrix
2. Python / Rust / Node/WASM 接口文档
3. Formula compatibility matrix
4. benchmark report 固化
5. release notes
6. migration guide
7. RC wheel/packages
8. 全平台 CI
9. version consistency
10. final release gate

### Exit Gate

所有 Architecture 2.0 质量门槛全绿后才允许合并/发布。

---

## 18. 推荐实施批次

为了避免一个超大 PR 既不可 review 又无法定位性能回归，建议在同一架构分支上按可验证批次推进。

### Batch A — Correctness closeout

- semantic contract
- API contract
- SSOT metadata

### Batch B — Kernel / Python memory path

- 12 core `*_into`
- direct ndarray
- GIL/out/buffer reuse

### Batch C — Planner / compute_many

- PlanNode
- shared families
- liveness

### Batch D — Formula convergence

- lowering
- CSE/DCE
- buffer assignment

### Batch E — Streaming

- StateGraph
- append/eval_last
- differential tests

### Batch F — Hardware optimization

- parallel
- SIMD
- fusion

每个 batch 都必须独立可 benchmark、可 regression，不等待最后才发现某阶段改变了语义。

---

## 19. 测试策略

### 19.1 Differential Testing

对 TA-Lib 兼容指标：

```text
random input
edge input
NaN input
short input
period boundary
large input
```

对比：

- finite mask
- warmup
- value tolerance
- output length
- output dtype

### 19.2 Batch / Streaming Equivalence

所有状态化指标必须满足：

```text
batch(history)[-1]
==
seed(history[:-1]); update(history[-1])
```

并扩展为逐 bar 全序列 differential test。

### 19.3 Formula Equivalence

同一公式同时走：

- unoptimized reference evaluator
- optimized ComputePlan
- streaming state graph

三者输出必须一致。

### 19.4 Property Tests

适合增加：

- constant series
- monotonic series
- scale/translation invariants（适用指标）
- rolling window boundary
- reset/seed/state snapshot consistency

---

## 20. 可观测性

高性能引擎需要可解释。

Debug/benchmark 模式增加：

```text
plan nodes
CSE eliminated nodes
fused kernels
allocated buffers
reused buffers
peak live buffers
state slots
parallel tasks
kernel timings
```

建议提供：

```python
plan.explain()
```

示例输出：

```text
nodes: 37 -> 21 after CSE/DCE
shared EMA states: 4
buffers: 21 logical -> 7 physical
fused groups: 3
streaming state slots: 11
```

这对后续定位“为什么某公式没有变快”非常重要。

---

## 21. 明确禁止事项

Architecture 2.0 期间禁止：

1. 为 Benchmark 单独写不可复用的特殊分支
2. 将 Finkit Core 与 TA-Lib Python public API 做不公平比较
3. Standalone/Formula/Streaming 各复制一套指标语义
4. SSOT 未声明的 public API 偷偷暴露
5. 没有 equivalence test 的 kernel fusion
6. 为速度改变 warmup / NaN / seed 语义
7. Python hot path 重新出现 list-of-float materialization
8. `compute_many()` 内部仅循环 standalone APIs
9. `eval_last()` 内部偷偷执行完整 `eval()`
10. nested Rayon / thread oversubscription
11. 通过降低性能 Gate 掩盖回归
12. 在已知 Release Gate 失败时合并 main

---

## 22. Definition of Done

Architecture 2.0 只有同时满足以下条件才定义为完成。

### Correctness

- Core regression 全绿
- TA-Lib semantic contract 全绿
- Python public API contract 全绿
- Formula optimized/reference equivalence 全绿
- Streaming/batch equivalence 全绿

### Performance

- Standalone 100K/1M geomean >=1.0x TA-Lib
- Standalone p90 >=0.9x
- Multi-indicator >=2x repeated TA-Lib
- Complex Formula >=2x equivalent TA-Lib + NumPy
- Incremental >=10x repeated TA-Lib full recomputation

### Memory

- 热路径无 Python float-list materialization
- Buffer reuse 可测
- allocations 相对旧架构下降 >=90%
- Formula peak live buffer 受 liveness 控制

### Architecture

- 四种入口共享 ComputePlan/Kernel/State/Executor
- 无孤立语义实现
- SSOT 能生成/验证 bindings/docs/tests/bench registry

### Release

- real wheel clean install 成功
- release gate 全绿
- version/docs/packages 一致
- CI 无 skip/mock 绕过失败

---

## 23. 最终定位

完成 Architecture 2.0 后，Finkit 不应只是“Rust 版 TA-Lib”。

目标产品形态应是：

```text
TA-Lib compatible standalone indicator library
+
high-performance shared multi-indicator engine
+
compiled domestic/international formula runtime
+
stateful incremental factor engine
+
stable multi-language compute core
```

TA-Lib 是单指标兼容和性能基线，但 Finkit 的真正优势来自 **共享计算、编译公式、增量状态和跨语言统一执行引擎**。

因此最终性能策略不是在每个函数上与 TA-Lib 进行无止境的微优化，而是：

> **单指标不输，多指标拉开差距，复杂公式形成优势，实时增量形成数量级优势。**
