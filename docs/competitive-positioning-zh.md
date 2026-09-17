# Finkit 竞品对比与超越路线

> 更新日期：2026-09-12。本文用于定义“Finkit 超越竞品”到底意味着什么，以及如何用可复现证据证明，而不是用单一机器上的宣传数字替代工程事实。

## 1. 竞争目标

Finkit 不以“指标数量最多”或“某一次 benchmark 最快”作为唯一目标。

真正的产品目标是成为一个 **Quant Compute Runtime**：在同一套 Rust canonical core 上，同时提供技术指标、公式执行、Factor DAG、Streaming、Feature / Research、增量重算和多语言交付，并让这些入口共享数值语义、执行计划和性能门禁。

因此，超越竞品必须同时覆盖五个维度：

1. **正确性**：TA-Lib parity、warm-up、NaN、lookback、no-lookahead 等语义可验证；
2. **执行效率**：批量、Streaming、重复计划、局部重算、内存分配和 FFI 开销分别测量；
3. **运行时复用**：Formula / Factor / Research 不各自维护第二套 DAG、缓存和中间结果；
4. **多语言一致性**：Rust 核心是 SSOT，绑定层不重新实现算法；
5. **产品可集成性**：不仅能算一个函数，还能作为研究服务、筛选器、实时分析和 SDK 的计算底座。

## 2. 当前主要对照对象

### TA-Lib

TA-Lib 是最重要的数值兼容和性能基准之一。其优势包括长期生产验证、广泛使用的技术指标集合、C/C++ 核心，以及不断扩展的原生语言和 Streaming 能力。

Finkit 不应该再把 TA-Lib 描述成“只有老 C 接口的静态指标库”。当前竞争重点应放在：

- 同指标数值 parity；
- 同输入、同进程、同硬件的真实吞吐；
- caller-owned output / allocation；
- 多指标公共中间结果复用；
- Formula / Factor 预编译计划；
- DirtyRange 局部失效与重算；
- Research 与多语言产品链路。

官方参考：<https://ta-lib.org/>、<https://ta-lib.org/api/>。

### VectorBT

VectorBT 是强大的 Python 量化研究与回测产品，围绕 Pandas / NumPy、Numba 以及可选 Rust kernel 构建，特别擅长参数扫描、组合模拟、研究交互和可视化。

Finkit 不应声称在 VectorBT 的完整回测产品、Notebook 交互或可视化生态上已经全面替代它。Finkit 的差异化重点是：

- Rust-first canonical computation；
- 非 Python 产品也能复用同一计算语义；
- Formula / Factor / Streaming / DirtyRange 统一执行合同；
- 可作为其他研究或回测系统的底层计算引擎。

官方参考：<https://vectorbt.dev/>。

### Pandas TA Classic

Pandas TA Classic 的优势是 DataFrame-first 易用性、非常广的指标覆盖、动态指标发现，以及可选 Numba 性能路径。

Finkit 与其竞争的重点不是 DataFrame accessor 体验，而是：

- Rust 核心与跨语言复用；
- 低分配 caller-owned output；
- Streaming 与持久化执行计划；
- Factor / Research 统一运行时；
- 生产服务侧更明确的 ABI / package contract。

官方参考：<https://xgboosted.github.io/pandas-ta-classic/>。

### ta-rs

`ta` crate 采用非常清晰的 `Next` / `Reset` 状态型模型，适合轻量 Rust Streaming 指标。

Finkit 的目标是在保留同类低开销 Streaming 能力的同时，额外覆盖 batch、Formula、Factor、Research、多语言和增量局部重算。

官方参考：<https://docs.rs/ta/latest/ta/>。

## 3. 能力对比矩阵

下表用于产品定位，不等同于性能排名。竞品能力会持续变化，必须定期复核。

| 维度 | Finkit | TA-Lib | VectorBT | Pandas TA Classic | ta-rs |
| --- | --- | --- | --- | --- | --- |
| canonical native core | Rust | C/C++ + expanding native ports | Python/Numba + optional Rust | Python/Numba | Rust |
| batch technical indicators | ✅ | ✅ | ✅ | ✅ | 以 stateful/streaming 为主 |
| streaming state | ✅ | ✅ / 持续扩展 | 可通过 compiled paths / simulation 使用 | 非核心定位 | ✅ |
| formula parser / compiled plan | ✅ | 非核心定位 | indicator factory / Python expression ecosystem | strategy/DataFrame layer | ❌ |
| factor dependency DAG | ✅ | ❌ | 可组合研究对象，但模型不同 | ❌ | ❌ |
| proven DirtyRange local recompute | ✅，仅安全依赖链 | 非 Finkit 同类合同 | 未按 Finkit DirtyRange 合同建模 | ❌ | 单状态推进，不是历史修订模型 |
| caller-owned `_into` hot path | ✅，选定 kernel | C API 输出 buffer | 后端依实现而异 | Python array/DataFrame 语义 | stateful scalar output |
| Factor Research / validation | ✅，持续扩展 | ❌ | ✅ 强研究/组合生态 | 指标与策略辅助 | ❌ |
| portfolio/backtest product | lightweight / 非核心 | ❌ | ✅ 强项 | 可集成第三方 | ❌ |
| multi-language product delivery | ✅ 多绑定路径 | ✅ 多原生/Wrapper | Python-first | Python-first | Rust-first |
| same-core semantic contract across bindings | 目标为强合同 | 各原生/Wrapper 状态不同 | Python 产品内统一 | Python 产品内统一 | Rust only |

“✅”只表示存在对应能力，不表示成熟度、覆盖面和速度完全相同。

## 4. Finkit 必须建立的可证明优势

### 4.1 TA-Lib parity：先一致，再更快

性能比较只对数值语义一致的结果有意义。Finkit 的 TA-Lib 对比必须先满足：

- 同数据；
- 同参数；
- 明确 warm-up；
- 多输出逐数组验证；
- `max_abs` / `max_rel` 在对应容差内；
- 失败不能通过修改 benchmark 输入或过滤结果隐藏。

### 4.2 Batch：在 canonical 热路径上争取不慢于 TA-Lib

对于 SMA、EMA、WMA、RSI、MACD、BBANDS、ATR、TRANGE、rolling statistics 等高频核心指标，目标是：

- 同机 Criterion 配对；
- Finkit / TA-Lib 同一数据生命周期；
- 对外分配与 FFI 成本明确说明；
- 关键 watchlist 最终收敛到 `Finkit <= TA-Lib`。

当前定期门禁先阻止 **超过 TA-Lib 25% 的严重退化**；严格“所有追踪指标均不慢于 TA-Lib”作为持续优化目标，不能在尚未真实跑绿之前写成已实现事实。

### 4.3 Repeated execution：用计划复用拉开差异

对研究服务、筛选器和 API 来说，一次函数调用不是完整成本。

必须增加这些场景的 benchmark：

- parse + execute；
- compile once + execute N times；
- Formula direct vs `CompiledFormula`；
- Factor discovery vs `FactorPlan`；
- shared plan / scratch reuse；
- single target vs compute-many shared intermediates。

目标是证明：**请求越重复，Finkit 的预编译计划和复用收益越明显。**

### 4.4 Historical correction：DirtyRange 必须比 full recompute 更有价值

Streaming 解决“新 Bar 到来”，DirtyRange 解决“历史数据某个局部被修订”。两者不是同一个问题。

应固定以下 benchmark：

- 100K / 1M rows；
- dirty 1 / 10 / 100 rows；
- fixed lookback 5 / 20 / 60 / 252；
- full execution vs range execution vs range-into；
- 记录 `recomputed_rows / total_rows`、wall time、allocation。

同时必须验证 range 结果与 full 结果一致；任何 cross-sectional、dynamic-lookback 或未知依赖都必须保守 full fallback。

### 4.5 Allocation：不仅比较 CPU 时间

每个核心热路径应该明确属于以下哪类：

- allocating convenience API；
- borrowed/zero-copy input；
- caller-owned `_into` output；
- persistent state / buffer arena；
- unavoidable allocation。

对于 `_into` 热路径，目标是使用 memory regression 锁住 **0 hot-path allocation**，而不是只依赖微秒级 wall clock。

### 4.6 Multi-language：把绑定成本纳入产品 benchmark

最终产品比较需要测：

- Rust native；
- Python NumPy → Rust；
- Node typed array → Rust；
- C ABI；
- JVM/.NET native boundary；
- WASM。

需要分别报告 kernel time 与 end-to-end binding time，避免只拿 Rust 内核数字宣传 Python/Node 实际体验。

## 5. Benchmark 分层

### L0 — correctness

- TA-Lib parity；
- batch / streaming convergence；
- full / range equivalence；
- FFI golden vectors。

### L1 — algorithmic regression

- O(n) vs O(n·period) relative test；
- DirtyRange recomputed-row contract；
- zero-allocation `_into`；
- HT_SINE release throughput budget。

### L2 — direct competitor

- Finkit vs TA-Lib C；
- Finkit vs `ta` crate（等价 Streaming / batch 场景）；
- 后续增加 Python end-to-end 与 Pandas TA Classic / VectorBT 的同任务测试。

### L3 — product workload

- 100–1000 symbols scanner；
- repeated formula service；
- factor DAG compute-many；
- 1M-row research pipeline；
- local historical correction；
- cross-language service call。

只要 L0 不通过，L1–L3 的速度结果都不得用于宣传。

## 6. 当前自动化

仓库现在包含：

- `core/tests/memory_regression.rs`：allocation gate；
- `core/tests/performance_regression.rs`：release-mode 性能合同；
- `core/benches/talib_c_comparison.rs`：TA-Lib C 配对 benchmark；
- `core/benches/competitive_bench.rs`：Rust `ta` crate 对照；
- `scripts/bench-vs-talib.sh`：本地一键配对运行；
- `scripts/bench_report.py`：结果解析、schema 与 gate；
- `.github/workflows/competitive-benchmark.yml`：每周 TA-Lib 0.7.1 真实 head-to-head。

报告必须记录 commit、CPU/平台、Rust、TA-Lib 版本与生成时间；没有有效 paired rows 时必须失败，不能生成“空报告成功”。

## 7. 宣传口径

允许：

> 在仓库固定的同机配对 benchmark 中，Finkit 对若干核心指标达到或超过 TA-Lib；完整结果、环境和失败项可复现查看。

允许：

> Finkit 的差异化不只在单指标速度，还包括持久化 Formula/Factor 计划、DirtyRange 局部重算、caller-owned output 和多语言统一语义。

暂不允许：

> Finkit 全面比 TA-Lib / VectorBT / Pandas TA 快。

暂不允许：

> Finkit 是最快的量化库。

除非有覆盖对应产品工作负载、硬件、版本、内存与 end-to-end binding 成本的公开可复现证据。

## 8. 下一阶段优化优先级

1. 把 TA-Lib watchlist 中所有 `speedup < 1.0x` 的核心指标逐项收敛；
2. 为 Formula compile-once / eval-many 建立竞争基准；
3. 为 FactorPlan full / range / range-into 建立 100K–1M rows 基准；
4. 将 `BufferArena` / scratch reuse 的 allocation 数纳入门禁；
5. 建立 Python end-to-end 对 Pandas TA Classic / VectorBT 同任务 benchmark；
6. 建立 ta-rs Streaming 等价 benchmark 的机器可读报告；
7. 所有对外性能结论自动引用 commit + environment + results artifact。

这才是 Finkit “超越竞品”的长期工程合同：**不是一句最快，而是让正确性、吞吐、增量效率、内存和集成成本都可以重复证明。**
