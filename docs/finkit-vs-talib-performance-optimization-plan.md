# Finkit vs TA-Lib 性能对标与优化改进方案

> 基线版本：Finkit `v0.1.4` vs TA-Lib Python/Core `0.7.1`  
> 对标层级：Rust Core、Python 已发布 wheel 公共 API、CompiledFormula、增量 Runtime、精度/暖机语义  
> 目标：让 Finkit 的核心计算优势真正传递到最终用户 API，并建立长期、可重复、可阻断回归的性能门禁。

## 1. 结论摘要

Finkit 当前存在明显的“核心计算性能”和“最终 Python 用户性能”分裂。

仓库历史 Rust/C Criterion 快照显示，在 10K bars 下，SMA、EMA、RSI、MACD、BBANDS、ATR 等核心算法可以达到或超过 TA-Lib C；但 2026-09-04 使用正式发布 wheel 的真实 Python API 基准显示，Finkit `v0.1.4` 在 SMA、EMA、RSI、MACD、ATR、OBV × 1K/10K/100K/1M 共 24 个观测点中没有一个快于 TA-Lib `0.7.1`，整体几何平均 `TA-Lib/Finkit = 0.0176x`，即 TA-Lib 在当前最终 Python 公共 API 层约快 `56.8x`。

这不是一个应该优先通过“继续优化 SMA 内循环”解决的问题。当前第一优先级必须是 Python Binding 与结果物化路径。

已经从源码确认的关键原因：

1. 大量 Python 指标绑定返回 `PyResult<Vec<f64>>`；PyO3 会把 Rust `Vec<f64>` 物化为 Python list。
2. `ffi/python-binding/finkit/__init__.py` 又对 list 执行 `np.asarray()`，形成 `Rust Vec -> Python list/float objects -> NumPy ndarray` 的二次物化链路。
3. 多输出指标会把这个成本按输出数量放大。MACD 三数组输出在 1M bars 的耗时接近单输出指标的约 3 倍，与该模型高度一致。
4. `CompiledFormula.eval_zero_copy()` 已经能够直接借用 NumPy 输入并直接返回 `PyArray1`，说明仓库已有正确技术基础；但普通指标绑定还没有统一走这条路径。
5. `CompiledFormula.eval()` 与 `eval_range()` 仍会通过 `slice.to_vec()` 复制全部 OHLCV 输入；`result_dict()` 对上下文变量还存在 clone，复杂公式会产生额外内存流量。
6. 当前 CI 的 `performance_regression` 主要验证 Rust 内部相对性能，不会发现 Python wheel 公共 API 的这种数量级回归。

因此，下一阶段优化顺序必须是：

**Python NumPy ABI 直出 -> Formula 输入/输出复制收敛 -> TA-Lib 语义/暖机对齐 -> 中间缓冲复用与 DAG 融合 -> SIMD/算法级热点优化 -> 多资产批处理并行。**

---

## 2. 对标范围升级

### 2.1 已完成真实安装包基准

环境：

- Ubuntu 22.04 GitHub hosted runner
- Python 3.12.14
- NumPy 2.3.3
- Finkit `0.1.4` 官方 Linux ABI3 wheel
- TA-Lib Python `0.7.1`
- TA-Lib Core `0.7.1`
- 同一进程、同一连续 `numpy.float64` 输入、warm-up 后交替执行、取多轮中位数

第一轮覆盖：

- SMA20
- EMA20
- RSI14
- MACD(12,26,9)
- ATR14
- OBV
- 1K / 10K / 100K / 1M bars

1M bars 代表性结果：

| 指标 | Finkit v0.1.4 | TA-Lib 0.7.1 | TA-Lib 更快约 |
| --- | ---: | ---: | ---: |
| SMA20 | 266.31 ms | 3.32 ms | 80.3x |
| EMA20 | 256.13 ms | 3.21 ms | 79.9x |
| RSI14 | 254.36 ms | 6.93 ms | 36.7x |
| MACD | 780.70 ms | 19.82 ms | 39.4x |
| ATR14 | 248.76 ms | 7.88 ms | 31.6x |
| OBV | 248.70 ms | 2.40 ms | 103.7x |

观察：SMA/EMA/RSI/ATR/OBV 虽然底层算法差异很大，但 Finkit 单输出 API 在 1M bars 上都集中在约 250 ms；这强烈说明公共输出物化成本已经压过算法本身。MACD 三输出约 780 ms，进一步支持“按输出数组数量支付 Python list 物化成本”的判断。

### 2.2 扩展指标矩阵

后续长期基准必须至少覆盖以下 32 个常用指标，不再只展示 6 个：

#### Overlap

- SMA20
- EMA20
- WMA20
- DEMA20
- TEMA20
- KAMA20
- BBANDS20
- SAR
- MIDPOINT14
- MIDPRICE14

#### Momentum

- RSI14
- MACD(12,26,9)
- STOCH(5,3,3)
- ADX14
- CCI14
- MOM10
- ROC10
- WILLR14
- CMO14
- MFI14
- PLUS_DI14
- MINUS_DI14

#### Volume

- OBV
- AD
- ADOSC(3,10)

#### Volatility

- ATR14
- NATR14
- TRANGE

#### Statistics / price action

- STDDEV20
- VAR20
- CORREL30
- BOP

规模统一为：

- 1K：Python 调用固定开销
- 10K：常用策略/交互工作负载
- 100K：单品种长历史
- 1M：大历史/分钟线压力测试

至少记录：

- p50 / median latency
- ns/bar 或 Mbar/s
- TA-Lib/Finkit speedup
- 最大绝对误差
- 最大相对误差
- finite mask / warm-up mask 是否一致
- 输出数组数量与类型
- 峰值 RSS / 分配次数（第二阶段加入）

### 2.3 核心公式矩阵

Finkit 公式引擎当前有约 230 个内置函数，公式性能不能只测单一 `MA()`。下一套固定公式集：

| 公式 | 对标方式 | 重点 |
| --- | --- | --- |
| `MA(CLOSE,20)` | TA-Lib SMA | 最简单 rolling builtin |
| `EMA(CLOSE,20)` | TA-Lib EMA | 状态平滑 |
| `RSI(CLOSE,14)` | TA-Lib RSI | Wilder 类状态 |
| `ATR(HIGH,LOW,CLOSE,14)` | TA-Lib ATR | 多输入 |
| `ROC(CLOSE,10)` | TA-Lib ROC | 简单窗口 |
| `MA(CLOSE,20)+2*STD(CLOSE,20)` | TA-Lib SMA + STDDEV + NumPy | 多节点组合与中间数组 |
| `CROSS(MA(CLOSE,5),MA(CLOSE,20))` | TA-Lib 两次 SMA + NumPy crossing | DAG/CSE/布尔信号 |
| `REF(CLOSE,1)` | NumPy shift | 历史引用基础操作 |

每条公式同时比较：

1. `CompiledFormula.eval()`
2. `CompiledFormula.eval_zero_copy()`
3. 对应 TA-Lib/NumPy 组合
4. 后续加入 `eval_range()`
5. 后续加入 `append_bar() + eval_last()` 增量模式

编译必须在计时区间外完成，避免把一次性 parse/compile 成本混进稳定态吞吐。

---

## 3. 根因分析

## 3.1 P0 根因：Python 输出链路产生 Python list

当前生成的 Python FFI 中，大量函数形态为：

```rust
fn sma(...) -> PyResult<Vec<f64>>
fn ema(...) -> PyResult<Vec<f64>>
fn wma(...) -> PyResult<Vec<f64>>
```

随后包级 Python wrapper 对 list 再做：

```python
np.asarray(value)
```

百万元素下这意味着：

1. Rust 结果数组/Vec 已经存在；
2. PyO3 为每个元素生成 Python float/list 表示；
3. NumPy 再逐元素读取 list；
4. NumPy 再申请连续 `float64` buffer；
5. Python list/float objects 最终等待回收。

算法即使只有几毫秒，也会被数百毫秒的语言边界物化覆盖。

### 必须改成

```text
Rust Vec/Array1
    -> PyArray1<f64>
    -> Python ndarray
```

不允许经过 Python list。

---

## 3.2 多输出指标重复支付转换成本

MACD、BBANDS、STOCH、MAMA 等返回 2-3 个数组。当前 `Tuple<Vec<f64>, ...>` 会对每个输出重复执行 list 物化，再由 Python wrapper 转 ndarray。

因此多输出指标必须直接返回：

```rust
PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)>
```

或等价的 `Py<PyArray1<f64>>` 所有权形式。

---

## 3.3 Formula `eval()` 全量输入复制

`CompiledFormula.eval()` 当前对 OPEN/HIGH/LOW/CLOSE/VOLUME 分别执行 `slice.to_vec()`，然后建立 owned `FormulaContext`。

这对需要 `append_bar()` 的 streaming context 是合理的所有权设计，但不应该成为所有“普通批量公式执行”的默认性能路径。

建议把语义拆清楚：

- `eval()`：保持兼容，但明确是 owned/context-retaining 模式；
- `eval_zero_copy()`：默认高性能批量模式；
- 新增/统一 `eval_owned()`：显式表达要保留上下文；
- Python 文档和示例默认批量计算改用 `eval_zero_copy()`；
- streaming 初始化才使用 `eval_owned()/eval()`。

---

## 3.4 Formula `eval_range()` 仍复制完整 OHLCV

当前 `eval_range()` 进入 Rust 后先把完整数组 `to_vec()`，之后 core 才根据 `[start,end)` 和 lookback 处理范围。

这会让“只计算尾部 100 bars”仍支付整个 1M OHLCV 的 Python->Rust复制成本。

必须实现真正 range borrowed path：

```text
NumPy full input
 -> borrow slices
 -> resolve dependency window [effective_start, end)
 -> only materialize unavoidable intermediates
 -> output requested [start,end)
```

验收要求：当输入从 100K 扩到 1M、但请求 range 长度固定为 1K 时，`eval_range()` 延迟不应近似 10x 增长。

---

## 3.5 `result_dict()` 对变量 clone

当前公式结果字典会遍历 context variables，并对变量 `value.clone()` 后转成 `PyArray1`。复杂公式若产生多个 named variables，会引入额外 O(number_of_variables × n) 内存带宽。

建议：

- 默认 fast path 仅返回 `__result__`；
- 增加 `return_variables=False/True`；
- 调试/交互模式才展开全部 named outputs；
- 可转移所有权时 move 而不是 clone；
- 对 CSE/Internal variables 永远不暴露；
- 多输出需要时使用显式 selection，例如 `outputs=["MA5", "MA20", "__result__"]`。

---

## 3.6 Python wrapper 双层通用装饰器

`__init__.py` 当前会给 native callable 再统一套错误翻译和 NumPy 结果转换 wrapper。

修复 native NumPy 输出后：

- `_as_numpy_result()` 对 native ndarray 不再有价值，应删除或退化为仅处理兼容旧路径；
- 参数错误最好在 Rust binding 中直接映射稳定 Python exception；
- 避免所有 hot-path 函数再经过 Python 层递归容器检查。

固定调用开销虽然不是 1M bars 的主要矛盾，但对 1K/更小数组和实时循环很重要。

---

## 4. P0：必须先完成的改造

## P0-1 全部数值 Python 指标直接返回 NumPy

涉及：

- `ffi/python-binding/src/generated.rs`
- `ffi/python-binding/src/lib.rs`
- 生成器/SSOT：`scripts/sync_bindings.py`、indicator registry 对应 Python body template
- `ffi/python-binding/finkit/__init__.py`

实现原则：

```rust
fn sma<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let slice = close.as_slice()?;
    let result = py.detach(|| moving_avg::sma(slice, timeperiod))?;
    Ok(PyArray1::from_vec(py, result.into_raw_vec()))
}
```

多输出直接创建 tuple of PyArray；整数 pattern 直接 `PyArray1<i32>`。

### 验收

- `type(finkit.sma(...)) is np.ndarray`
- native extension 直接调用也返回 ndarray，不依赖 package wrapper 修正类型
- 1M SMA 不允许再出现 ~250 ms 的与算法无关固定线性转换坡度
- public Python API benchmark 至少下降一个数量级后再进入下一阶段

---

## P0-2 生成器一次性修复，禁止手工改 100+ binding

不能只修 SMA/EMA/RSI。

必须修改 Python binding generator，让所有 registry 生成项统一输出 NumPy；然后重新生成并用 CI 校验生成文件无 drift。

新增生成器 contract：

- scalar -> Python scalar
- `Array1<f64>`/`Vec<f64>` -> `PyArray1<f64>`
- integer signal array -> `PyArray1<i32/i64>`
- tuple arrays -> tuple of PyArray
- dict/named outputs -> PyDict values are PyArray

---

## P0-3 Formula `eval_range_zero_copy()` / 真正 range borrowing

新增明确 API（命名可最终统一）：

```python
plan.eval_range_zero_copy(open_, high, low, close, volume, start, end)
```

或者直接让现有 `eval_range()` 在 contiguous NumPy 输入上进入 borrowed fast path。

需要把 lookback planner 下沉到“选择 slice 之前”，避免完整历史 copy。

---

## P0-4 Formula 输出选择与零 clone

建议 API：

```python
plan.eval_zero_copy(..., return_variables=False)
plan.eval(..., outputs=["MA5", "MA20", "__result__"])
```

默认生产路径只返回最终结果。

---

## P0-5 建立 Python wheel 级性能门禁

当前 Rust `performance_regression` 只能证明内部 optimized SMA 没退化，不能代表用户安装 wheel 后的性能。

新增永久 workflow：

```text
build/install current wheel
install pinned TA-Lib reference
run public Python benchmark
check precision + warm-up mask
publish JSON/Markdown artifact
apply regression gate
```

PR 不需要跑完整 1M × 全矩阵，可分为：

- PR fast gate：1K + 100K，核心 10-12 指标
- main/release gate：10K + 100K + 1M，32 指标 + 公式矩阵
- weekly/manual deep gate：完整等价 TA-Lib 指标、patterns、内存 profile

---

## 5. P1：Binding 修复后再做的 Core 优化

只有 P0 完成后，才能准确看到真正算法热点。

## P1-1 `*_into` 输出缓冲 API

为高频核心指标逐步提供：

```rust
sma_into(input, period, output)
ema_into(...)
rsi_into(...)
atr_into(...)
```

优势：

- Python/native binding 可一次申请最终 NumPy buffer；
- Rust 直接写入 NumPy backing memory；
- 进一步消除 `Array1 -> Vec -> PyArray` 的最后一次 move/allocation；
- Formula engine 可以从 buffer pool 获取目标 buffer 并复用。

优先顺序：SMA/EMA/RSI/ATR/OBV -> MACD/BBANDS -> STOCH/ADX/CCI -> statistics。

---

## P1-2 融合多阶段指标

### MACD

不要构造不必要的完整 EMA 中间数组后再重复分配。

目标：

- 一次遍历/状态机维护 fast EMA、slow EMA；
- MACD line 写入目标 buffer；
- signal EMA 直接消费 MACD stream；
- hist 同步写入；
- 三个输出只各分配一次。

### BBANDS

共享 rolling sum/mean/variance 状态，避免 SMA 和 STDDEV 分别扫描相同窗口。

### ADX / DI / ATR

共享 TR、+DM、-DM Wilder smoothing 中间状态，避免多个派生指标各自重复生成相同序列。

---

## P1-3 rolling extrema 改为单调队列

STOCH、WILLR、AROON、MIDPOINT、MIDPRICE、HHV/LLV 等窗口最大最小值应确保 O(n) rolling deque，而不是 O(n × period) 重扫窗口。

基准必须增加 period 维度：14 / 64 / 252 / 1024，防止只在固定 period=14 时隐藏复杂度问题。

---

## P1-4 statistics rolling state

STDDEV、VAR、CORREL、BETA、LINEARREG 系列需要复用 rolling aggregates：

- `sum(x)`
- `sum(x^2)`
- `sum(y)`
- `sum(y^2)`
- `sum(xy)`

数值稳定性要求高的路径可使用补偿求和、two-pass fallback 或周期性 rebase，不能为了速度破坏与 TA-Lib/公式系统的容差合同。

---

## P1-5 Formula DAG/CSE 从“表达式级”提升到“kernel 级”

现有 optimizer 已有 CSE 思路，但需要把它变成可观测 contract：

公式：

```text
MA5 := MA(CLOSE,5);
MA20 := MA(CLOSE,20);
A := CROSS(MA5,MA20);
B := MA5 / MA20;
A + B
```

同一个 `MA5/MA20` 必须只计算一次。

更进一步，`MACD`、`BOLL`、`ADX/+DI/-DI` 等可共享底层 kernel state，而不只是缓存最终 Array1。

新增 debug counters：

- executed kernels
- reused kernels
- intermediate allocations
- pooled buffer hits/misses
- bytes materialized

然后在 benchmark 中断言优化确实生效。

---

## 6. P2：面向因子计算的大吞吐优化

## P2-1 多指标批量 API

对因子/特征工程，逐个 Python 调用：

```python
sma = ta.sma(close)
rsi = ta.rsi(close)
atr = ta.atr(high, low, close)
...
```

会反复跨语言边界。

增加：

```python
result = ta.compute_many(
    {"sma20": ("SMA", {"timeperiod": 20}), ...},
    open=open_, high=high, low=low, close=close, volume=volume,
)
```

内部：

- 输入验证一次
- OHLCV borrow 一次
- DAG 构建一次
- 中间结果共享
- 一次跨语言返回

这会比单个指标继续挤 5%-10% SIMD 更符合 Finkit “因子计算基础库”的定位。

---

## P2-2 多资产并行优先于单序列乱并行

单个 rolling 指标具有状态依赖，不应强行按 bar 切线程。

并行维度优先：

- symbol
- factor
- independent formula plan
- independent parameter sweep

提供明确的 batch scheduler，避免 Python 用户自己 ThreadPool + GIL/内存复制。

---

## P2-3 streaming/incremental contract

`append_bar()` + `eval_last()` 应成为实时场景首选：

- 追加一根 bar 不复制完整历史；
- 固定 lookback 指标 amortized O(1)；
- capacity 可提前 reserve；
- ring buffer/rolling state 受控；
- 不因为连续 append 重新构造整个 FormulaContext。

增量 benchmark：

- 初始化 100K bars
- append 10K bars
- 每根调用 eval_last
- 对比“每根重新全量 TA-Lib”的数量级优势

这个场景是 Finkit 应该显著领先 TA-Lib 的核心差异化能力之一。

---

## 7. 精度与 TA-Lib 兼容性必须与性能一起验收

不能用“更快”换语义漂移。

每一个可对标指标同时检查：

1. shape 一致
2. warm-up finite mask 一致
3. max absolute error
4. max relative error
5. 多输出每个 component 单独检查
6. 特殊输入：constant / monotonic / gaps / zero volume / extreme magnitude / NaN policy

默认目标：

- `max_abs <= 1e-9`
- `max_rel <= 1e-12`
- 或按指标建立经过审查的独立 tolerance
- warm-up mask 对于宣称 TA-Lib-compatible 的 API 必须一致

第一轮实际基准已经发现 MACD 重叠区域数值差很小，但 finite-mask/warm-up 位置不完全一致，因此 MACD compatibility 必须单独修复，不能只看最大误差。

---

## 8. 新性能目标与分阶段 Gate

当前不应直接要求一次从 50x 差距跳到全面快于 TA-Lib；门禁分阶段收敛。

## Stage A：Binding 修复验收

- public Python 1M 单输出指标不再出现 list 物化型 ~250 ms 基线
- 32 指标中至少 90% 相比 v0.1.4 提速 >= 10x
- 多输出指标提速应与输出数组数量显著相关
- 无 API dtype/shape 退化

## Stage B：达到 TA-Lib 同量级

核心 12 指标：

- 几何平均 `TA-Lib/Finkit >= 0.8x`
- 任一核心指标不得慢于 TA-Lib 2x
- 1M bars 无 O(n×period) 异常坡度

## Stage C：核心指标整体领先

目标：

- 核心 12 指标几何平均 `TA-Lib/Finkit >= 1.0x`
- 至少 70% 核心指标快于 TA-Lib
- SMA/EMA/RSI/ATR/MACD/BBANDS 重点目标 >= 1.1x
- 增量/多指标 DAG 场景目标 >= 2x，因为这是 Finkit 的结构性优势场景

这里 speedup 定义为：

```text
speedup = TA-Lib time / Finkit time
```

因此 `>1.0x` 表示 Finkit 更快。

---

## 9. CI 与 Benchmark 架构改造

建议最终保留三层 benchmark，不再混为一个“权威性能数字”。

### Layer 1：Rust Core vs TA-Lib C

目的：验证算法/kernel。

- Criterion
- 同进程 C FFI
- 不经过 Python
- 10K/100K/1M
- 输出 ns/bar、allocation、speedup

### Layer 2：Installed Python wheel vs TA-Lib wheel

目的：验证用户真实体验。

- 先 build/install wheel 或安装 release candidate wheel
- TA-Lib 固定版本
- 从仓库源码目录外执行，防止 import shadow
- 连续 NumPy float64
- public `finkit` namespace，不允许偷偷调用内部 Rust API

### Layer 3：Formula/Factor workload

目的：验证 Finkit 差异化能力。

- compiled simple formula
- complex formula DAG
- eval_zero_copy
- eval_range
- eval_last/append_bar
- compute_many/factor plan
- 统计 parse/compile 与 steady-state 分离数字

所有层输出统一 JSON schema，Markdown 只由 JSON 自动生成。

---

## 10. 文档治理

当前 `docs/BENCHMARK_REPORT.md` 和 `docs/formula-performance.md` 中存在较早环境下生成的性能数字，它们不能继续被读者理解为 v0.1.4 当前已发布 Python wheel 的用户性能。

改造：

1. 报告头必须包含：commit/tag、wheel SHA256、TA-Lib version/core version、OS、CPU、Python、NumPy、日期。
2. 明确 `Core` 与 `Installed Python API` 是不同层级。
3. 自动生成报告，禁止手改宣称“所有核心指标均超越 TA-Lib”。
4. release notes 只引用当前 release gate 实测结果。
5. 未达到 Python public gate 前，不对外使用笼统“整体快于 TA-Lib”的描述。

---

## 11. 实施顺序

### Phase 0：基线冻结

- 固定 `v0.1.4` vs TA-Lib `0.7.1`
- 固定 32 指标 + 8 公式
- 保存 JSON artifact
- 将第一轮 24 点结果作为 before baseline

### Phase 1：Python NumPy 直出

- 修改 generator
- generated.rs 全量重生成
- hand-written bindings 同步迁移
- 删除 list -> np.asarray hot-path
- wheel 安装测试
- 扩展 benchmark 重跑

### Phase 2：Formula Copy Elimination

- eval_range borrowed path
- result selection / remove clones
- 中间 buffer pool
- complex formula benchmark

### Phase 3：真实 Core 热点

- flamegraph/perf
- rolling extrema
- statistics rolling state
- MACD/BBANDS/ADX kernel fusion
- `*_into` API

### Phase 4：因子/批处理差异化

- compute_many
- cross-indicator DAG reuse
- multi-symbol parallel scheduler
- streaming benchmark

### Phase 5：Release Gate

- Linux/Windows/macOS 至少各跑 public wheel smoke/perf sanity
- Linux canonical performance runner 生成正式报告
- precision/warm-up green
- no >10% regression against accepted new baseline

---

## 12. 代码改动清单

| 优先级 | 文件/模块 | 改造 |
| --- | --- | --- |
| P0 | `ffi/python-binding/src/generated.rs` | Vec 返回改为直接 PyArray |
| P0 | Python binding generator / registry template | 从生成源永久修复返回 ABI |
| P0 | `ffi/python-binding/src/lib.rs` | 手写指标/统计/形态返回 PyArray |
| P0 | `ffi/python-binding/finkit/__init__.py` | 删除通用 list -> ndarray 热路径 |
| P0 | `ffi/python-binding/src/formula_plan.rs` | range zero-copy、输出选择、减少 clone |
| P0 | Python tests | ndarray dtype/shape/error contract |
| P0 | GitHub Actions | installed-wheel benchmark gate |
| P1 | `core/src/indicators/*` | `*_into`、kernel fusion、rolling state |
| P1 | `core/src/formula/*` | buffer pool、DAG/CSE kernel reuse |
| P1 | `core/benches/*` | 32 指标、period scaling、allocation |
| P2 | runtime/factor plan | compute_many、多资产并行、streaming |
| Docs | benchmark/formula performance docs | Core/API 分层并自动生成 |

---

## 13. 发布条件

下一次宣称性能优化完成前，至少满足：

- [ ] Python native 指标不经过 Python list
- [ ] 32 指标 public wheel 基准完成
- [ ] 8 条核心公式 eval/eval_zero_copy 基准完成
- [ ] 1K/10K/100K/1M scaling 无异常
- [ ] 核心指标 precision/warm-up contract 通过
- [ ] MACD finite mask 对齐问题处理
- [ ] Stage A 达成
- [ ] performance JSON + Markdown artifact 可复现
- [ ] CI 对新的 Python public benchmark 有回归阻断
- [ ] 旧性能文档不再把 Rust Core 数字描述成 Python 用户性能

达到 Stage B 后才能恢复“与 TA-Lib 同量级”的公开表述；达到 Stage C 后才能用当前 release 的实测数据声明“核心指标整体领先 TA-Lib”。

---

## 14. 推荐的立即执行任务

1. **直接修 Python binding generator，而不是逐函数手改。**
2. 全量将 `PyResult<Vec<f64>>` / tuple Vec 改成 PyArray 返回。
3. 移除 Python `_as_numpy_result()` 对 hot-path 的二次容器遍历。
4. 重建 Linux wheel，先重跑 SMA/EMA/RSI/MACD/ATR/OBV 24 点基准。
5. 扩到本文 32 指标矩阵。
6. 修 `CompiledFormula.eval_range()` 全量 copy。
7. 增加 formula output selection，消除不需要的 context variable clone。
8. 重跑 8 条公式矩阵。
9. 只有此时再开始 flamegraph 与 Core 算法级优化。
10. 将新的 wheel-level benchmark 纳入 release gate。

这条顺序可以避免团队花大量时间优化一个只占最终耗时 1%-5% 的 Rust kernel，而真正的 30x-100x 语言边界成本继续存在。
