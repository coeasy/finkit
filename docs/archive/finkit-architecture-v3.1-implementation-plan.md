# Finkit Architecture v3.1 优化改进实施计划

## 一、重新定义本轮目标

本轮不再追求“增加更多 fast path”，而是完成一次真正的执行架构收敛。

最终目标只有五个：

1. **Correctness 全绿**
   - Core parity = 0 failure
   - Streaming 与 Batch 完全一致
   - Formula 与 standalone kernel 完全一致
   - Python batch 与 standalone warm-up / NaN / seed 完全一致

2. **执行引擎唯一化**
   - Standalone
   - `compute_many`
   - Formula
   - Factor
   - Streaming

   最终全部落到同一个：

   `HotExecutionPlan → UnifiedExecutor → KernelRegistry → BufferArena / StateArena`

3. **Python Public API 真正零冗余**
   - NumPy input borrowed
   - NumPy output direct
   - 不再出现 Vec → Python list → ndarray
   - 支持 caller-owned `out=`
   - GIL 在纯 Rust kernel 阶段释放

4. **利用共享计算形成结构性优势**
   - 不再仅和 TA-Lib 拼单个 kernel 的几个百分点
   - DMI、MACD、Rolling Statistics、Extrema 等家族通过共享状态获得 2x～5x 优势
   - Streaming / incremental 获得数量级优势

5. **Release Gate 作为唯一完成标准**
   - Core tests
   - Python installed-wheel tests
   - cross-platform wheels
   - parity
   - installed-wheel benchmark
   - Architecture v3 performance gate

   任何一项失败，PR #28 都不能进入合并阶段。

---

# 二、调整实施优先级

新的优先级应固定为：

```text
P0 Correctness / Wheel contract
        ↓
P1 Unified Executor
        ↓
P2 Shared State / Kernel Families
        ↓
P3 Python FFI Zero-Copy
        ↓
P4 Algorithm Hotspots
        ↓
P5 Formula / Factor / Streaming convergence
        ↓
P6 Allocation / SIMD / Parallel tuning
        ↓
P7 Release Gate
```

必须避免继续出现：

```text
某 benchmark 慢
→ 新增一个专项 fast path
→ Formula 再写一份
→ Python 再写一份
→ Streaming 再维护一套状态
→ 语义产生分叉
```

Architecture v3.1 的核心原则是：

> **先统一，再加速。**

---

# 三、P0：先把正确性和安装包合同全部修平

## P0.1 修复 OhlcFamilyState

重点统一：

- TR
- ATR
- NATR
- +DM
- -DM
- +DI
- -DI
- DX
- ADX

禁止维护“近似相同”的共享状态算法。

应抽出唯一：

```text
DirectionalFamilyState
```

由它同时服务：

- Batch
- Streaming
- compute_many
- Formula
- Factor

需要严格复刻 canonical batch：

- seed 范围
- Wilder 初始值
- smooth 更新顺序
- previous OHLC 更新时间
- +DM/-DM tie 规则
- period 边界
- warm-up index

验收：

```text
shared state output == canonical batch output
parity failures = 0
```

不得通过放宽 epsilon 解决。

---

## P0.2 SAR 只保留一个状态机

目前 Batch SAR 与 Streaming SAR 不应该分别实现状态转换。

重构为：

```rust
struct SarState { ... }

impl SarState {
    fn seed(...)
    fn update(...)
}
```

Batch：

```text
for each bar
    SarState.update()
```

Streaming：

```text
SarState.update(current_bar)
```

重点统一：

- reversal 条件
- extreme point
- AF reset
- AF increment
- 前两根 high / low clamp
- reversal bar 输出
- update 顺序

目标：

```text
batch == streaming == pointer streaming
```

---

## P0.3 修 Python batch multi-output warm-up

当前 wheel 可以成功生成，但安装 wheel 后测试失败，应优先修复，而不是把它视为 packaging 次要问题。

尤其检查：

```text
compute_indicators(["macd", ...])
```

和：

```text
macd(...)
```

的 warm-up index 是否来自两个实现。

正确架构必须是：

```text
compute_indicators
       ↓
ComputePlan
       ↓
same MACD kernel
```

而不是：

```text
compute_indicators → batch MACD implementation
macd               → standalone implementation
```

这是 Architecture v3.1 最重要的架构信号之一。

### P0 完成条件

必须同时：

```text
cargo test --workspace
Python binding tests
installed wheel tests
Windows wheel tests
Linux wheel tests
macOS Intel wheel tests
macOS ARM wheel tests
```

全部通过。

---

# 四、P1：建立真正的 UnifiedExecutor

目前 `HotExecutionPlan` 不应该继续只作为“编译结果结构体”。

增加核心：

```rust
pub struct UnifiedExecutor {
    buffers: BufferArena,
    states: StateArena,
}
```

核心接口：

```rust
execute(plan, inputs, outputs)

execute_range(plan, start, end)

execute_last(plan, bar)

reset(plan)

rebind_inputs(...)
```

执行循环只允许出现：

```text
KernelId
InputSlot
BufferSlot
StateSlot
ParameterSlot
```

禁止：

```text
String
HashMap<String, ...>
operation parsing
AST matching
indicator name matching
dynamic DAG construction
```

---

# 五、重新定义 HotExecutionPlan

建议从目前的简单结构继续升级：

```text
HotExecutionPlan
 ├─ KernelStep[]
 ├─ InputLayout
 ├─ OutputLayout
 ├─ BufferLayout
 ├─ StateLayout
 ├─ ParameterArena
 ├─ DependencyLifetime
 └─ ExecutionMetadata
```

`KernelStep`：

```rust
struct KernelStep {
    kernel: KernelId,
    inputs: SmallVec<BufferSlot>,
    outputs: SmallVec<BufferSlot>,
    states: SmallVec<StateSlot>,
    params: ParameterRange,
}
```

进一步避免：

- 每节点 Vec allocation
- runtime parameter boxing
- runtime lookup

对于绝大多数指标：

```text
inputs <= 4
outputs <= 3
states <= 4
```

可使用 SmallVec 或固定小数组。

---

# 六、P2：StateArena 从“状态存储”升级成“状态共享系统”

目前“一节点一个状态”仍不足以实现真正的 v3 优势。

增加：

```text
StateKey
```

例如：

```text
EMA(close, 12)
EMA(close, 26)
Wilder(TR, 14)
RollingMin(low, 14)
RollingMax(high, 14)
```

编译期建立：

```text
StateKey → StateSlot
```

相同 state 自动 intern。

---

## 第一批必须共享的 Family

### 1. DMI Family

共享：

```text
TR
+DM
-DM
Wilder(TR)
Wilder(+DM)
Wilder(-DM)
DX
ADX
```

一次遍历派生：

- TRANGE
- ATR
- NATR
- PLUS_DM
- MINUS_DM
- PLUS_DI
- MINUS_DI
- DX
- ADX
- ADXR

目标组合性能：

```text
ATR + NATR + ADX + DX + PLUS_DI + MINUS_DI
>= 2.5x TA-Lib sequential calls
```

---

### 2. EMA / MACD Family

共享：

```text
EMA12
EMA26
MACD
EMA9(MACD)
```

同时供：

- EMA12
- EMA26
- MACD
- MACD signal
- histogram

组合目标：

```text
>= 2.0x
```

---

### 3. Rolling Statistics Family

共享：

```text
rolling sum
rolling sum squared / variance state
mean
variance
stddev
```

派生：

- SMA
- VAR
- STDDEV
- BBANDS

不要为 BBANDS 再重复 SMA + STD 两次扫描。

组合目标：

```text
>= 2.0x
```

---

### 4. Extrema Family

建立：

```text
RollingExtremaState
 ├─ min deque
 └─ max deque
```

共享给：

- MIN
- MAX
- MIDPOINT
- MIDPRICE
- WILLR
- STOCH
- AROON

禁止窗口极值过期后重新扫描整个 period。

目标复杂度必须稳定为：

```text
O(n)
```

---

# 七、P3：Python Binding 从“优化脚本”迁回 SSOT

当前的长期目标不能依赖：

```text
generate Rust
→ optimize_python_bindings.py
→ patch generated Rust
```

这类 post-process 只能作为迁移工具。

最终：

```text
indicator_registry.json
        ↓
sync_bindings.py
        ↓
直接生成最终 PyO3 ndarray binding
```

SSOT metadata 应描述：

```text
input dtype
input count
output count
contiguous fast path
parameters
output dtype
supports out=
GIL-safe
kernel id
```

生成器直接输出：

```rust
PyReadonlyArray1<f64>
PyArray1<f64>
Py<PyArray1<f64>>
```

而不是先：

```rust
Vec<f64>
```

再包装。

---

# 八、Python API 增加 out= 模型

高频场景：

```python
out = np.empty_like(close)

finkit.ema(
    close,
    20,
    out=out,
)
```

多输出：

```python
macd = np.empty_like(close)
signal = np.empty_like(close)
hist = np.empty_like(close)

finkit.macd(
    close,
    12,
    26,
    9,
    out=(macd, signal, hist),
)
```

这对：

- 策略循环
- 因子批量计算
- Streaming bridge
- 大规模数据处理

会比单纯 kernel SIMD 更有价值。

---

# 九、P4：集中处理真正 benchmark 热点

不要平均优化所有指标。

第一梯队：

```text
TRANGE
MIDPOINT
MIDPRICE
WILLR
MFI
PLUS_DI
MINUS_DI
WMA
BBANDS
OBV
AD
ADOSC
```

---

## P4.1 MIDPOINT / MIDPRICE / WILLR

实现 fused extrema：

```text
one input traversal
two monotonic deques
direct output
```

避免：

```text
rolling_min → Vec
rolling_max → Vec
third loop combine
```

---

## P4.2 WMA

必须使用 O(1) recurrence：

维护：

```text
sum
weighted_sum
```

每个 bar O(1)，而不是每个 window O(period)。

---

## P4.3 MFI

共享：

```text
typical price
raw money flow
positive rolling sum
negative rolling sum
```

使用 ring buffer / rolling sums。

禁止重复窗口扫描。

---

## P4.4 AD / ADOSC

ADOSC 应直接复用 AD accumulation/state。

结构：

```text
MoneyFlowVolumeState
      ↓
AD
├─ EMA fast
└─ EMA slow
      ↓
ADOSC
```

---

## P4.5 BBANDS

BBANDS 应成为 rolling-statistics family 的 consumer：

```text
RollingStats(mean, variance)
          ↓
 upper / middle / lower
```

而不是独立重新计算 SMA/STD。

---

# 十、P5：五大前端完全统一

## Standalone

```text
API
→ compile/cache tiny plan
→ UnifiedExecutor
```

常用单指标可缓存计划。

---

## compute_many

完全淘汰主路径中的：

```text
String + Box<dyn Fn()>
```

保留旧 BatchBuilder 仅作为 compatibility facade：

```text
BatchBuilder
    ↓
ComputePlanBuilder
    ↓
HotExecutionPlan
    ↓
UnifiedExecutor
```

---

## Formula

Formula 只负责：

```text
parse
semantic normalization
CSE
compile
```

运行时不允许：

```text
uppercase()
match "EMA"
match "ROC"
```

字符串只存在于 compile phase。

---

## Factor

Factor 与 Formula 使用同一 DAG compiler。

区别只在：

```text
output contract
metadata
grouping
```

不能再建立新的计算 runtime。

---

## Streaming

Streaming 不应该拥有第二套指标算法。

使用同一个：

```text
KernelId
StateSlot
```

区别只是 executor mode：

```text
BatchExecutor
RangeExecutor
LastExecutor
```

---

# 十一、P6：BufferArena 和 Allocation Gate

BufferArena 增加：

```text
overwrite buffers
filled buffers
pinned outputs
borrowed outputs
reusable scratch
high-water mark
allocation counters
```

编译期执行：

```text
dependency use count
last use
buffer lifetime
scratch slot reuse
```

目标：

```text
重复 Formula/compute_many 执行
临时 heap allocation ↓ >=95%
```

CI 记录：

```text
allocations/call
allocated bytes
arena hit ratio
peak buffers
clones
```

这些应成为性能 regression signal。

---

# 十二、SIMD 放到正确的位置

SIMD 应是 P6，而不是 P0。

适合 SIMD：

- arithmetic transforms
- elementwise binary ops
- normalization
- vector combination
- final band generation

不应为了 SIMD 强行改写：

- recursive EMA
- Wilder
- rolling state
- branching state machine

优先：

```text
algorithmic complexity
memory
allocation
fusion
cache locality
```

然后才是 SIMD。

---

# 十三、Parallel 策略也要统一

不要默认所有指标 Rayon 并行。

Planner 根据 DAG 判断：

### 小任务

串行。

### 多个独立大 kernel

并行。

### Shared-state family

必须融合后串行单扫，不能拆开并行造成重复计算。

### 大 Formula DAG

按 dependency layer 并行。

---

# 十四、Benchmark 体系重新分成 5 层

## Gate A：Semantic

必须：

```text
parity_failures = 0
errors = 0
```

---

## Gate B：Core Kernel

Top20：

```text
geomean >= 1.25x
```

---

## Gate C：Installed Public API

必须保持：

```text
overall >= 1.15x
100K >= 1.15x
1M >= 1.20x
Top20 each >= 1.05x
no persistent result < 0.95x
```

不能用内部 Rust benchmark 替代。

---

## Gate D：Structural advantage

新增长期 CI：

### DMI group

```text
>= 2.5x
```

### MACD group

```text
>= 2.0x
```

### Rolling Statistics

```text
>= 2.0x
```

### Extrema group

```text
>= 2.0x
```

### Formula CSE

```text
>= 3x
```

### Streaming

```text
>= 10x
```

---

## Gate E：FFI overhead

分别测：

```text
Rust Core
PyO3 direct
installed Python API
```

目标：

```text
Py FFI overhead < 10%
```

否则禁止用 Core benchmark 宣称 Python 性能已经达标。

---

# 十五、CI 重构

PR #28 最终建议形成以下清晰 pipeline：

```text
1. Format / Clippy
2. Core correctness
3. Semantic parity
4. Generator SSOT check
5. Python compile
6. Python source tests
7. Cross-platform ABI3 wheel build
8. Installed-wheel contract tests
9. Installed-wheel parity
10. Core benchmark
11. Public API benchmark
12. Architecture v3 structural benchmark
13. Allocation regression
14. Release gate summary
```

任何前置 correctness 失败：

```text
performance jobs skip
```

这样可以避免在错误实现上浪费 benchmark 资源。

---

# 十六、清理 migration 技术债

Architecture v3 正式落地以后应删除：

```text
arch-v3-*-once.yml
一次性 repair workflow
一次性 patch workflow
已经完成使命的 source transformer
```

真正保留：

```text
registry SSOT
binding generator
benchmark tools
release gate
parity tools
architecture validation
```

目标是让代码仓库最终看不到“迁移过程中发生过多少补丁”。

---

# 十七、PR #28 建议拆分逻辑提交

虽然仍可保持一个 PR，但后续提交必须按逻辑分组：

```text
1. fix: restore canonical parity
2. fix: unify wheel batch semantics
3. refactor: introduce unified executor
4. refactor: migrate batch to execution plan
5. refactor: migrate formula executor
6. refactor: unify streaming state
7. perf: intern shared indicator states
8. perf: fuse extrema family
9. perf: optimize rolling/DMI families
10. perf: generate direct ndarray bindings
11. ci: enforce architecture v3 gates
12. chore: remove migration scaffolding
```

禁止再提交大而混杂的“一键 architecture patch”。

---

# 十八、最终 Architecture v3.1 形态

最终架构应收敛成：

```text
             Python
               │
Rust API ──────┼──── Node / WASM
               │
      Standalone / compute_many
               │
        Formula / Factor
               │
         Semantic DAG
               │
      Normalize / CSE / DCE
               │
       Lifecycle Analysis
               │
       HotExecutionPlan
               │
        UnifiedExecutor
        ┌──────┴──────┐
        │             │
   BufferArena    StateArena
        │             │
        └──────┬──────┘
               │
        Kernel Registry
               │
 ┌─────────────┼─────────────┐
 │             │             │
Scalar       SIMD      Stateful Kernel
                             │
                     Streaming eval_last
```

核心原则：

```text
一个语义
一个 Kernel
一个 State
一个 Executor
五个 Frontend
```

---

# 十九、最终 Release Definition of Done

PR #28 只有全部满足以下条件才能进入 release-ready：

### Correctness

- Core tests 100%
- Formula tests 100%
- Streaming tests 100%
- Python tests 100%
- parity failures = 0

### Build

- Linux wheel
- Windows wheel
- macOS Intel wheel
- macOS ARM wheel
- Python ABI3 compatibility

全部 green。

### Architecture

- 热路径无 String
- 无 runtime parsing
- BufferArena / StateArena 完全分离
- compile-time lifetime
- state interning
- compute_many 使用 HotExecutionPlan
- Formula 使用 HotExecutionPlan
- Streaming 使用相同 Kernel/State
- Python binding 由 SSOT 直接生成 ndarray path

### Performance

- overall ≥ 1.15x
- 100K ≥ 1.15x
- 1M ≥ 1.20x
- Top20 每项 ≥ 1.05x
- persistent floor ≥ 0.95x
- Core Top20 ≥ 1.25x
- FFI overhead <10%
- multi-indicator structural gates 全部达到设计要求

### Repository hygiene

- 删除一次性 migration workflow
- 删除 obsolete patch scripts
- 没有 orphan code
- 没有重复 indicator implementation
- 没有 unused compatibility execution path
- 文档与最终实现一致

满足以上条件后，PR #28 才定义为：

```text
Architecture v3 complete
Release candidate ready
```

而不是“CI 大部分绿色”。