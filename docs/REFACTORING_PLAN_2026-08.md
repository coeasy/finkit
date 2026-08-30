# AlphaTA（finkit）深度审计与重构优化方案

> **版本**：v1.1 ｜ **日期**：2026-08-30 ｜ **状态**：待评审
> **审计范围**：`P:\llm_code\finkit` 全 workspace（13 crates，core/src 302 文件 / 132,039 LOC）
> **前置说明**：本方案基于**实测数据**而非既有文档推断。审计中发现多份既有文档与代码现状已漂移（§2.3），所有结论均已回源码复核。
>
> **v1.1 修订**：建立可复现度量后，覆盖度结论发生**反转**。v1.0 的「流式覆盖 64%、需大规模补实现」是错误的（分母口径 + 漏扫目录所致）。实测：batch **100%**、流式按声明口径 **98%**、**公式层仅 61%** —— 真·最大缺口在公式层。度量工具 `scripts/check_coverage.py` 已交付。

---

## 0. 执行摘要

审计共定位 **16 类问题**，其中：

| 级别 | 数量 | 代表问题 |
|---|---:|---|
| 🔴 P0 正确性/UB | **3** | Android JNI 空指针 UB；4 个公式函数定义后未注册；`Williams %R` 全层缺失 |
| 🟠 P1 结构性 | **6** | **公式层仅覆盖 61%（92 个指标在公式里调不到）**；core 混入交易系统层；流式全有全无；公式注册表手工维护；热路径重复建表；计划文档泛滥 |
| 🟡 P2 一致性 | **5** | 生产 658 处 unwrap；EMA 种子显式化未完成；80% 脚本无引用；包名不一致；仓库卫生 |
| 🟢 P3 工程化 | **3** | 审计脚本缺陷（本轮已修）；本地扫描污染；测试覆盖不均 |

**三条最关键的结论：**

1. **覆盖度的真相与既有认知完全相反。** 初稿曾得出「流式覆盖仅 64%，需大规模补实现」——这是错的。按注册表声明口径（`streaming=true` 的 145 项），流式覆盖是 **98%**；batch 覆盖是 **100%**（235/236）。**真正的最大缺口在公式层：仅 61%**，即 92 个已实现（多数 batch + 流式都有）的指标在 Pine 公式里根本调不到。
2. **「补实现」不是重点，「补接线」才是。** 43 个核心指标（Keltner/Force Index/Vortex/JMA/MAMA/Ulcer…）batch 与流式都齐备，只差一个 `fn_xxx` + 一行注册。这是低风险高收益的机械工作。
3. **在动手重构前，先合并既有的 9 份计划文档**（§3.1）——文档泛滥本身已是主要维护成本。

---

## 1. 现状量化画像

### 1.1 代码规模分布

| 模块 | 文件数 | LOC | 占比 | 备注 |
|---|---:|---:|---:|---|
| `formula/` | 27 | 31,062 | 23.5% | 公式引擎 + Pine 兼容层 |
| `indicators/` | 35 | 30,336 | 23.0% | batch 指标 |
| `streaming/` | 161 | 30,305 | 23.0% | 流式增量指标（文件数最多） |
| `patterns/` | 10 | 12,209 | 9.2% | 形态识别 |
| `math/` | 13 | 11,102 | 8.4% | 数学内核 + SIMD |
| `features/` | 31 | 10,268 | 7.8% | 特征工程 |
| `transforms/` | 9 | 748 | 0.6% | |
| `core/src/*.rs` | 13 | 5,892 | 4.5% | **含非指标交易系统层** |
| **合计** | **302** | **132,039** | 100% | |

**最大 10 个文件**：

| 文件 | LOC | 性质 |
|---|---:|---|
| `formula/functions.rs` | 6,496 | 293 个 `fn_*` 函数平铺 + 299 行手工注册表 |
| `math/simd_ops.rs` | 4,372 | SIMD 内核 |
| `patterns/candlestick.rs` | 3,749 | K 线形态 |
| `indicators/momentum.rs` | 3,326 | 动量指标 |
| `formula/templates.rs` | 3,310 | 公式模板库 |
| `formula/simd.rs` | 3,146 | SIMD 求值 |
| `formula/executor.rs` | 2,869 | 字节码 VM |
| `indicators/momentum_ext.rs` | 2,443 | |
| `indicators/cycle.rs` | 2,285 | 周期指标（HT_* 系列） |
| `indicators/overlap.rs` | 2,205 | 均线族 |

> 前 10 个文件合计 33,501 LOC，占 25%。其中 **5 个属于公式引擎**（合计 20,131 LOC）。

### 1.2 覆盖度矩阵（实测，已消除命名假阳性）

以 `docs/indicator_registry.json`（**236 项**，非旧文档记载的 78 项）为基准。
度量脚本 `scripts/check_coverage.py`（本轮新建，见 §3.5）。

| 维度 | 命中 | 覆盖率 | 说明 |
|---|---:|---:|---|
| **batch（Rust 原生 API）** | **235 / 236** | **100%** | exact 匹配 91%，其余经人工核验别名确认 |
| **流式（全 236 项口径）** | 166 / 236 | 70% | 分母含 91 个声明「不需要流式」的项 |
| **流式（声明口径：145 项）** | **142 / 145** | **98%** | ← **有效数字** |
| **公式层（Pine 可调）** | **144 / 236** | **61%** | ← **真·最大缺口** |

> **为什么两个流式数字差这么多**：236 项注册表里有 91 项标注 `streaming: false`（主要是 52 个 `CDL_*` K 线形态——它们本质是单 K 判定函数，不需要增量状态；以及 `astock`/`sentiment`/`breadth` 的部分横截面指标）。用 236 当分母会把这些「按设计不需要」的算成缺口，**严重高估工作量**。正确的分母是 145。

**按类别分布**：

| category | total | stream | batch | formula | stream% |
|---|---:|---:|---:|---:|---:|
| pattern | 56 | 5 | 56 | **7** | 9% |
| astock | 9 | 1 | 9 | 9 | 11% |
| sentiment | 5 | 3 | 5 | **1** | 60% |
| breadth | 6 | 4 | 6 | **1** | 67% |
| price_transform | 5 | 4 | 5 | 4 | 80% |
| statistics | 15 | 12 | 15 | 12 | 80% |
| volume | 20 | 19 | 19 | **10** | 95% |
| overlap | 28 | 27 | 28 | 22 | 96% |
| momentum | 49 | 48 | 47 | 40 | 98% |
| volatility / cycle / fibonacci / math_* | 43 | 43 | 43 | 38 | 100% |

**结论**：batch 与流式**已接近完备**，核心矛盾不在「实现缺失」而在「**层间接线缺失**」——公式层是唯一的大面积洼地。

### 1.3 FFI 绑定现状

| 绑定 | `#[no_mangle]` | src LOC | generated.rs | 备注 |
|---|---:|---:|---:|---|
| c-binding | 81 | 2,772 | 2,350 | 最完整 |
| java-binding | 109 | 2,852 | 795 | |
| dotnet-binding | 58 | 2,147 | 1,246 | |
| go-binding | 55 | 1,775 | 947 | |
| ios-binding | 19 | 454 | 275 | 覆盖最低之一 |
| android-binding | 3 | 194 | 36 | **仅 15 指标**（shim 宏） |
| python-binding | 0（pyo3） | 9,287 | 1,938 | LOC 最高 |
| node-binding | 0（napi） | 3,861 | 1,170 | |

### 1.4 错误处理分布（生产代码，已排除 test/ffi）

| crate | `unwrap()` | `expect(` | `unreachable!` | `panic!` |
|---|---:|---:|---:|---:|
| core | 471 | 68 | 0 | **0** |
| cli | 1 | 110 | 0 | 0 |
| visualization | 0 | 3 | 2 | 0 |
| python-binding | 2 | 0 | 0 | 0 |
| ffi-common | 0 | 1 | 0 | 0 |
| **合计** | **474** | **182** | **2** | **0** |

**生产危险点 Top 文件**（排除 test 后重排，与旧报告差异很大）：

| 文件 | 危险点 | unwrap | expect |
|---|---:|---:|---:|
| `formula/functions.rs` | 154 | 154 | 0 |
| `cli/src/main.rs` | 111 | 1 | 110 |
| `formula/simd.rs` | 71 | 71 | 0 |
| `formula/pine/parser.rs` | 59 | 2 | 57 |
| `formula/bytecode.rs` | 38 | 38 | 0 |
| `patterns/chart.rs` | 34 | 34 | 0 |
| `formula/jit.rs` | 31 | 31 | 0 |
| `formula/executor.rs` | 25 | 25 | 0 |
| `indicators/momentum.rs` | 19 | 19 | 0 |

> **结论**：公式引擎独占约 **376 / 658（57%）** 的生产危险点。`indicators/` 层仅约 45 处——此前记录的「indicators 866 处」是未排除 test 模块的误计（见 §2.3）。

---

## 2. 问题清单

### 🔴 P0 — 正确性与未定义行为

#### P0-1 Android 绑定存在必然崩溃的 UB

**位置**：`ffi/android-binding/src/lib.rs:125`

```rust
let mut env = unsafe { std::mem::zeroed::<JNIEnv>() };
to_double_array(&mut env, v.to_vec())
```

`JNIEnv` 内部持有指向 JNI 函数表的裸指针；全零填充后调用 `env.new_double_array()` 等于**通过空函数表指针做虚调用**，必然段错误。该函数 `finkit_android_version()` 是 `#[no_mangle] pub extern "system"` 导出符号，Android 侧可直接调到。

**修复**：版本号不应走 JNI 数组分配。改为返回 `jint` 编码的 `major*10000+minor*100+patch`，或提供 `GetVersion` 风格的字符串返回（需真实 `env`）。全仓库仅此 1 处 `zeroed::<JNIEnv>()`。

**风险**：低（改动 <20 行）｜**收益**：消除导出函数必崩

---

#### P0-2 四个已实现的指标未接入公式层

`core/src/formula/functions.rs` 定义了 293 个 `fn_*` 函数，其中 **5 个从未注册**到 `get_builtin_functions()`，且经全仓扫描确认 **4 个零调用点**（真孤儿）：

| 函数 | 对应指标 | 影响 |
|---|---|---|
| `fn_rocp` | ROCP | 注册表已声明，公式层调不到 |
| `fn_rocr` | ROCR | 同上 |
| `fn_rocr100` | ROCR100 | 同上 |
| `fn_macdfix` | MACDFIX | 同上 |
| `fn_kdj_line` | — | 内部 helper，正常 |

**根因**：注册靠 **299 行手工 `map.insert()`**（`functions.rs:5252` 起）。新增函数漏注册不会报错，只会静默不可达——这正是已发生的 4 例。

**修复**：① 补注册 4 个；② 长期改为**声明式注册**（§3.4）。

---

#### P0-3 `Williams %R` 在三个层次全部缺失

注册表声明 `streaming: true`，但实测三层状态：

| 层 | 状态 | 证据 |
|---|---|---|
| batch（Rust API） | ❌ 无 | 全 `core/src` 无 `willr` 函数 |
| 流式 | ❌ 无 | 无 `StreamingWillr` |
| 公式层 | ✅ 有（仅此一处） | `fn_willr` 注册为 `WILLR` / `WR`（`functions.rs:1939`、`:5362-5363`） |

**矛盾点**：这是**唯一只能通过公式引擎访问、却没有 Rust 原生 API** 的注册表指标。8 个 FFI 绑定走的是 Rust API 而非公式层，**因此全部无法调用 WILLR**。

**修复**：补 `indicators/momentum.rs::willr()` + `streaming/momentum/willr.rs`。公式层已有实现可直接作为数值对照基准。约 0.5 天。

> `scripts/check_coverage.py` 的「三处全无（真·完全缺失）」检查会持续守住这条线。

---

### 🟠 P1 — 结构性

#### P1-0 【最大缺口】公式层仅覆盖 61%，92 个已实现指标在公式里调不到

这是本次审计**最重要、也最反直觉**的发现：项目不缺实现，缺的是**从「已实现的 Rust 指标」到「公式层」的接线**。

| 类别 | 注册表项数 | 公式层可调 | 缺口 |
|---|---:|---:|---:|
| pattern（`CDL_*` 形态） | 56 | 7 | **49** |
| volume | 20 | 10 | **10** |
| momentum | 49 | 40 | **9** |
| overlap | 28 | 22 | **6** |
| breadth | 6 | 1 | **5** |
| volatility | 11 | 6 | **5** |
| sentiment | 5 | 1 | **4** |
| statistics | 15 | 12 | 3 |
| price_transform | 5 | 4 | 1 |

**其中 43 个核心指标 batch + 流式都已齐备，只差一个 `fn_xxx` 包装 + 一行注册**：

`MAMA`、`Ichimoku Cloud`、`ELDERRAY`、`Elder Ray`、`LINREG_INTERCEPT`、`LINREG_ANGLE`、`AD_RATIO`、`McClellan Oscillator`、`TRIN`、`Fear & Greed Index`、`Put/Call Ratio`、`VR`、`ENE`、`Ulcer Index`、`RVI`、`McGinley`、`VIDYA`、`Force Index`、`EOM`、`NVI`、`PVI`、`PVT`、`KVO`、`AO`、`KST`、`Vortex`、`Inertia`、`QStick`、`JMA`、`Efficiency Ratio`、`CFO`、`Twiggs MF`、`Keltner Channel`、`ADR`、`Chaikin Volatility`、`VZO` 等。

**为什么重要**：

- 公式引擎是本库对外的**主要用户入口**（Pine 兼容、`talib` 风格字符串调用）
- 这些指标 batch 侧早已实现并被测试覆盖，**接线风险极低**
- 单个指标工作量约 8–12 行（`fn_xxx` 参数提取 + 委托调用 + 注册），43 个约 **400 行**

**根因与 P0-2 同源**：注册全靠手工 `map.insert`，没有任何机制把「注册表里有这个指标」与「公式层能调到这个指标」关联起来。

**验收**：`scripts/check_coverage.py` 的 `FORMULA coverage` 从 61% 提升，且新增 CI 门禁不允许回退。

---

#### P1-1 core 混入了与指标无关的交易系统层

`core/src/` 根目录下有 **2,655 LOC** 非技术分析代码：

| 文件 | LOC | 职责 |
|---|---:|---|
| `talib_ffi.rs` | 1,155 | TA-Lib C FFI（基准对比用，有 `talib-c` feature 门控 ✅） |
| `backtest.rs` | 438 | 向量化回测引擎 |
| `risk.rs` | 374 | 组合风险指标 |
| `sector.rs` | 355 | 申万行业轮动 |
| `selectors.rs` | 349 | 选股因子合成 |
| `multi_period_resonance.rs` | 284 | 多周期形态联动 |
| `metrics.rs` | 250 | Prometheus metrics |
| `batch.rs` | 269 | 并行批处理（有 `rayon` 门控 ✅） |
| `circuit_breaker.rs` | 169 | 熔断器（有 `circuit-breaker` 门控 ✅） |

**问题**：`backtest/risk/sector/selectors/multi_period_resonance/metrics` 在 `lib.rs:79-113` 仅由 `#[cfg(feature = "std")]` 门控，而 `std` 在 `default` 中——**永远无法关闭**。这些模块让「技术分析库」变成「交易系统框架」，膨胀二进制、引入 `tracing`/`metrics` 依赖。

**建议**：新增 feature 门控（短期），或拆分为 `alpha-ta-select`/`alpha-ta-backtest`/`alpha-ta-obs` 独立 crate（中期）。

---

#### P1-2 流式模块被绑死在 `indicators-all` 上

`core/src/lib.rs:53`：
```rust
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub mod streaming;
```

30,305 LOC 的流式子系统**全有或全无**。用户无法只启用「overlap + 对应流式」。

**建议**：按类别拆分 `streaming-*` feature（工作量取决于跨类依赖，需先做依赖图分析）。

---

#### P1-3 公式函数注册表手工维护

- `formula/functions.rs` 6,496 LOC，单文件承载 293 个函数
- 注册靠 299 行手写 `map.insert("X".to_string(), fn_x)`（其中 7 个函数有别名，如 `fn_log`→`LOG`/`LN`）
- 漏注册 = 静默不可达（P0-2 已实证 4 例）

**建议**：
1. 拆分为 `functions/{math,stat,overlap,momentum,volume,tdx,dzh,fox}.rs`（按文件内已有的分节注释切分）
2. 改用 `inventory`/`linkme` 或声明式宏自动注册，让「定义即注册」

---

#### P1-4 热路径重复构建函数表

`functions.rs:5252` 的 `get_builtin_functions()` 每次调用都新建 299 项 `HashMap<String, FormulaFn>`（约 299 次 `String` 堆分配）。调用点在三条引擎构造路径：

- `formula/bytecode.rs:387`
- `formula/executor.rs:42`
- `formula/jit.rs:73`

即**每次公式编译/求值实例化都要重建函数表**。

**修复**：`OnceLock<HashMap<...>>` 缓存（1 行改动量级），或直接改静态表 + `phf`。

---

#### P1-5 计划文档泛滥（本项目当前最大维护成本）

**9 份**计划/路线图文档，合计约 **150 KB**：

| 文档 | 大小 |
|---|---:|
| 根 `OPTIMIZATION_PLAN.md` | 9.4 KB |
| 根 `REFACTORING_PLAN.md` | 15.3 KB |
| `docs/OPTIMIZATION_PLAN.md` | 3.0 KB |
| `docs/OPTIMIZATION_PLAN_2026.md` | 8.7 KB |
| `docs/OPTIMIZATION_REFACTORING_PLAN.md` | 26.6 KB |
| `docs/PLANNING.md` | 27.9 KB |
| `docs/PRD.md` | 30.8 KB |
| `docs/PROGRESS.md` | 11.9 KB |
| `docs/UPGRADE_PLAN_2026.md` | 16.2 KB |

**突出矛盾**：`OPTIMIZATION_PLAN.md` 在**根目录与 `docs/` 下同名但内容不同**（9,410 vs 2,964 字节）——任何人打开都可能读错版本。此外 7 份 `docs/*.md` 未被 `INDEX.md` 收录。

---

### 🟡 P2 — 一致性

#### P2-1 流式覆盖的 3 个违约项
见 §1.2 —— 按声明口径流式已达 98%，**不是优先事项**。需处理的仅 3 项注册表标了 `streaming: true` 却没有流式实现：

| 指标 | 类别 | batch | 公式层 | 动作 |
|---|---|---|---|---|
| `Williams %R` | momentum | ❌ | ✅ | 见 P0-3（batch 也要补） |
| `LINEAR_REG` | statistics | ✅ | ✅ | 补 `StreamingLinearReg` |
| `MONEY_FLOW` | astock | ✅ | ✅ | 补 `StreamingMoneyFlow` |

#### P2-2 生产 unwrap/expect 658 处
见 §1.4。集中在公式引擎（57%）。`cli/src/main.rs` 的 110 处 `expect` 已是改造范本（带原因字符串）。

#### P2-3 EMA 种子显式化只完成一半
- ✅ `EmaSeed` API 已落地（`math/moving_avg.rs:26`，streaming `ema.rs:6` 再导出）
- ❌ **`ema_with_seed` 在 `core/src/indicators/` 中零调用**，该目录仍有 **22 处裸 `ema()`** 依赖隐式默认 `Sma`
- 即：种子约定仍靠"记住默认值"，A2 目标未达成

#### P2-4 80% 脚本无引用
`scripts/` 共 114 文件（34 在 `archive/`），**80 个活跃脚本中 64 个在 Makefile/CI/docs 中零引用**，含 16 个 `debug_macd_*`/`test_macd_*` 一次性调试脚本、多个一次性迁移脚本（`convert_macros.py`、`rename_java_packages.ps1`、`rewrite_go_mod.py` 等）与救火脚本（`_recover_v3.py`、`_rebalance_java.py`）。

#### P2-5 FFI 包名不一致
同一代码库并存 `com.finkit.indicators` 与遗留 `com.rusttalib.Indicators`（`android-binding/src/lib.rs` 注释与 java 绑定）。

#### P2-6 仓库卫生
- 根目录 3 个 PowerShell 输出转储（`ps_out.txt`、`ps2_233047.txt`、`ps3_233246.txt`）**未被 gitignore**
- `.forge/`（175 KB `events.jsonl` 运行时事件日志）**被 git 跟踪**，每次运行产生 diff 噪音

---

### 🟢 P3 — 工程化

#### P3-1 审计脚本缺陷（本轮已修复 ✅）
`scripts/_a5_scan.py` 存在两个 bug，导致既有 `docs/A5_UNWRAP_AUDIT.md` 结论失真：
1. **Top 文件列表未做 test 分桶** —— 把 `parser.rs` 的 48 处（全在 `#[cfg(test)]` 内）`panic!` 计入「生产 Top」，与 TOTAL 表「生产 panic=0」自相矛盾
2. **`crate_of()` 对 Windows 绝对路径取 `parts[0]`** —— 得到盘符 `P:`，导致 core/cli/wasm 全部坍缩成一行

已修复并补充「生产 Top 文件」表格到 Markdown 报告。修正后：`indicators/cycle.rs`(83)、`patterns/candlestick.rs`(66) 等从生产榜消失（全是 test 代码）。

#### P3-2 本地扫描污染
`.aza/worktrees/`（12 MB / 1,438 文件，仓库完整副本）与 `dist/`（55 MB）虽已 gitignore，但会污染所有 `find`/`grep`/全仓扫描。136 个重名文件主要源于此。

#### P3-3 测试覆盖不均

| crate | 集成测试文件 | src 内 `#[cfg(test)]` |
|---|---:|---:|
| core | 33 | 284 |
| visualization | 2 | 23 |
| cli | **0** | 1 |
| wasm | **0** | 1 |
| python-binding | **0** | **0** |
| node-binding | **0** | **0** |
| go / dotnet / ios | 0 | 3 each |
| java | 0 | 2 |
| android | 0 | 1 |

Python / Node 绑定**零测试**（两者 src 合计 13,148 LOC）。

---

### 2.3 ⚠️ 已确认的文档漂移（勿再采信旧记录）

审计中发现既有文档/记忆与代码现状不符，均已回源码复核：

| 项 | 旧记录 | 实测（2026-08-30） |
|---|---|---|
| MACD EMA 种子 | `input[0]` FirstValue，被 golden 锁定不可改 | **`SMA` 种子**（`momentum.rs:547-550`、`:2650-2653`），已改 TA-Lib 兼容；流式 `streaming/momentum/macd.rs:21` 同为 SMA，**两侧一致** |
| 流式 MACD 须用 FirstValue | 是 | **否**（同上，已对齐 SMA） |
| 注册表指标数 | 78 | **236** |
| 生产 unwrap 分布 | indicators 866 处 | **core 全库 471**，indicators 层仅约 45（旧数未排除 test） |
| 本地是否装 clippy | 未装 | **已装**（`stable-x86_64-pc-windows-msvc/bin/`） |
| 生产 `panic!` | 0 | **确认为 0**（107 处全在 test，含 `parser.rs` 48 处）✅ |
| **流式覆盖率** | — | 全口径 70%，**声明口径 98%**；用 236 当分母会严重高估缺口 |
| **batch 覆盖率** | — | **100%**（235/236）。此前报 66% 是漏扫 `patterns/`、`math/`、`features/` 所致 |

> 建议在 §3.1 文档中统一更正，避免后续基于错误前提决策。**度量必须能复现**：以上数字均由 `scripts/check_coverage.py` 产出。

---

## 3. 重构方案

### 总体原则
1. **先止血，再减重，后优化**：P0 修复 → 结构解耦 → 性能/覆盖
2. **每个改动都有可验证门禁**：`cargo check` + 既有测试 + `sync_bindings --check`
3. **不新增计划文档**：所有内容并入本文件，旧计划文档归档

---

### 3.1 【先决】计划文档合并（0.5 天，最高优先级）

| # | 动作 |
|---|---|
| 1 | 确立**本文件** `docs/REFACTORING_PLAN_2026-08.md` 为唯一现行计划 |
| 2 | `git mv` 归档：`docs/PLANNING.md`、`docs/PRD.md`、`docs/PROGRESS.md`、`docs/OPTIMIZATION_PLAN_2026.md`、`docs/OPTIMIZATION_REFACTORING_PLAN.md`、`docs/UPGRADE_PLAN_2026.md` → `docs/archive/plans-2026-07/` |
| 3 | **消除同名冲突**：`docs/OPTIMIZATION_PLAN.md` 与根目录版合并（保留根目录版，docs 版删除或改名为 `docs/archive/OPTIMIZATION_PLAN_draft.md`） |
| 4 | 根 `REFACTORING_PLAN.md` 标记为历史（顶部加「已由 2026-08 方案取代」横幅） |
| 5 | 补全 `docs/INDEX.md` 缺失的 7 个链接 |

**验收**：`docs/` 下计划类文档 ≤ 2 份；`INDEX.md` 无遗漏。

---

### 3.2 【P0 止血】1.5 天

| # | 任务 | 文件 | 验收 |
|---|---|---|---|
| 2.1 | 修复 `zeroed::<JNIEnv>()` UB：改返回 `jint` 编码版本号 | `ffi/android-binding/src/lib.rs:114-127` | 无 `zeroed::<JNIEnv>`；导出函数可安全调用 |
| 2.2 | 注册 4 个孤儿函数（ROCP/ROCR/ROCR100/MACDFIX） | `formula/functions.rs` | 新增测试断言 `get_builtin_functions()` 含这 4 个键 |
| 2.3 | 加**注册完整性测试**：扫描文件内所有 `fn fn_*`，断言每个都已注册或标记为内部 helper | `formula/functions.rs` 测试模块 | 漏注册即测试失败（防复发） |
| 2.4 | 补 `willr()` batch + `StreamingWillr`（P0-3） | `indicators/momentum.rs`、`streaming/momentum/willr.rs` | 与 `fn_willr` 数值一致；`check_coverage.py` 的「真·完全缺失」归零 |

**基线**：本轮已确认 `cargo check --workspace` **0 error**（见 §5），上述改动需在同基线上保持通过。

---

### 3.3 【P1-a】core 职责解耦（3–5 天）

| # | 任务 | 说明 |
|---|---|---|
| 3.1 | 为 `backtest`/`risk`/`sector`/`selectors`/`multi_period_resonance`/`metrics` 增加专属 feature（如 `domain-ext`），默认关闭 | 短期，低风险 |
| 3.2 | 验证 `cargo check --no-default-features --features std,indicators-all` 通过 | 证明可裁剪 |
| 3.3 | （中期）评估拆分为 `alpha-ta-select` / `alpha-ta-backtest` / `alpha-ta-obs` 三个 crate | 需先确认无跨依赖 |

---

### 3.4 【P1-b】公式引擎重构（5–8 天，最大单项）

| # | 任务 | 说明 |
|---|---|---|
| 4.1 | 按文件内既有分节注释拆分 `functions.rs`（6,496 LOC）为 6–8 个子模块 | 机械拆分，零行为变化 |
| 4.2 | 引入声明式注册（宏或 `inventory`），消灭 299 行手工 `map.insert` | 根治 P0-2 类问题 |
| 4.3 | `OnceLock` 缓存 `get_builtin_functions()` | 消除每次引擎构造的 299 次分配 |
| 4.4 | 治理 `simd.rs`(71)、`bytecode.rs`(38)、`jit.rs`(31)、`executor.rs`(25) 的生产 unwrap | 按「是否用户可达」分类：`pine/parser.rs` 的 57 处 `expect` 优先（用户输入路径） |

**顺序**：4.3（1 行，立即收益）→ 4.2 → 4.1 → 4.4

---

### 3.5 【已完成 ✅】覆盖度度量基建

**本轮已交付 `scripts/check_coverage.py`**，覆盖了原计划的全部目标：

| 原计划任务 | 状态 |
|---|---|
| 消除命名不匹配假阳性 | ✅ 三级匹配（exact → prefix → substring）+ 人工核验 `ALIASES` 表 + `CDL_` 前缀剥离 |
| 输出真实覆盖矩阵 + 缺失清单 | ✅ 三维度（batch / streaming / formula）+ 按类别分布 + 违约项清单 |
| 接入 CI | ⬜ 见下方 5.1 |

**用法**：

```bash
python scripts/check_coverage.py            # 汇总 + 缺失清单
python scripts/check_coverage.py --strict   # 只认 exact 匹配（下界估计）
python scripts/check_coverage.py --json     # 机器可读（供 CI 消费）
```

**踩过的两个坑**（写在脚注里，避免后续重犯）：

1. **batch 侧不能只扫 `indicators/`**。`CDL_*` 形态在 `patterns/candlestick.rs`，回归斜率在 `features/rolling_stats.rs`。只扫 `indicators/` 会得到「batch 覆盖 66%」的错误结论，真值是 **100%**。
2. **`ALIASES` 查表必须用归一化键**。`norm("CFO")="cfo"` 要映射到 `norm("chande_forecast_oscillator")`，直接用原始名查表会静默失配。

**剩余任务**：

| # | 任务 |
|---|---|
| 5.1 | 接入 CI：以 `--json` 输出为准，断言 `batch == 100%`、流式声明口径 `>= 98%`、公式层**只增不减** |
| 5.2 | （可选）在 `indicator_registry.json` 补 `batch_symbol` / `streaming_symbol` / `formula_name` 显式字段，彻底摆脱启发式匹配 |

---

### 3.6 【P1-d】公式层接线：把 43 个已实现指标暴露给公式引擎（3–4 天）

对应 P1-0。**这是本方案里投入产出比最高的一块**：实现已存在、已被测试覆盖，只差包装与注册。

| # | 任务 | 说明 |
|---|---|---|
| 7.1 | 按类别分批补 `fn_xxx`：`volume`(10) → `momentum`(9) → `overlap`(6) → `volatility`(5) → 其余(13) | 每批独立 commit，逐批跑 `check_coverage.py` 看 `FORMULA coverage` 上升 |
| 7.2 | 统一参数提取模式：复用既有 `ensure_args_len` / `extract_n`，签名保持 `(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>` | 与 293 个既有函数完全一致 |
| 7.3 | 多返回值指标（Ichimoku、Keltner、Bollinger）需套用既有 tuple 返回约定 | 先读 `fn_boll` 作为范本 |
| 7.4 | 每个新增函数配 1 个数值断言测试（对照 batch 侧同参数结果） | 防止接线时参数顺序错位 |
| 7.5 | `CDL_*` 49 个形态单独评估：是否需要全部暴露给公式层 | 量大但形态统一，可考虑用宏批量生成 |

**风险与缓解**：

| 风险 | 缓解 |
|---|---|
| 参数顺序/默认值与 TA-Lib 不一致 | 每个函数对照 batch 侧写数值断言（7.4） |
| 多返回值约定不统一导致解析失败 | 先读 `fn_boll` / `fn_macd` 两个既有范本再动手 |
| 一次性改 43 个，回归难定位 | 严格按类别分批，每批独立 commit + 独立跑测 |

---

### 3.7 【P2】一致性收尾（持续）

| # | 任务 | 优先级 |
|---|---|---|
| 6.1 | `indicators/` 的 22 处裸 `ema()` 显式改为 `ema_with_seed(.., EmaSeed::Sma)` | 高（消除隐式约定） |
| 6.2 | 归档 64 个无引用脚本（尤其 16 个 `debug_macd_*`）到 `scripts/archive/` | 中 |
| 6.3 | 统一 Java 包名为 `com.finkit.indicators` | 中（需同步 Kotlin/Java 侧与文档） |
| 6.4 | gitignore 补 `ps*.txt`；`.forge/` 移出 git 跟踪（`git rm --cached` + gitignore） | 中 |
| 6.5 | 为 `cli`/`wasm`/`python`/`node` 补基础 smoke 测试 | 中 |
| 6.6 | 流式 feature 按类别拆分（依赖 P1-2 的依赖图分析） | 低（高工作量） |

---

## 4. 执行路线图

```
第 0 步  计划文档合并（3.1）              ← 必先做，0.5 天
   │
   ▼
第 1 步  P0 止血（3.2）                    ← 1.5 天，含防复发测试
   │
   ▼
第 2 步  公式层接线（3.6）                ← 3–4 天，43 个指标，低风险高收益
   │      · 立竿见影：用户可调指标 +30%
   │
   ├──────────────┬──────────────┐
   ▼              ▼              ▼
第 3 步         第 4 步         第 5 步
core 解耦      公式引擎重构     一致性收尾
(3.3, 3–5天)   (3.4, 5–8天)    (3.7, 持续)
```

**总工作量估算**：约 15–20 人天

**建议起步顺序**：

1. **3.1** 文档合并（0.5 天，解锁后续所有讨论的前提）
2. **3.2** P0 止血（1.5 天）
3. **3.4.3** `OnceLock` 缓存函数表（**1 行改动换即时性能收益**）
4. **3.6** 公式层接线 43 个指标（**投入产出比最高的一块**）
5. **3.4.2** 声明式注册（做完第 4 步后，这一步的收益会明显变大）
6. **3.3** core 解耦

> 原计划把「补流式实现」排在高位，实测后已移除——流式按声明口径已达 98%，不值得投入。

---

## 5. 验收门禁

### 5.0 当前基线（2026-08-30 实测）

```
cargo check --workspace   →  0 error  ✅
                             8 warnings（alpha-ta-ffi）：
                               · 2 × unsafe-op-in-unsafe-fn（Rust 2024 待迁移）
                                 c-binding/src/lib.rs:166, :172
                               · 4 × dead_code（ffi_catch_i64 / ffi_catch_void 未使用）
                               · 2 × deprecated / unused import
```

> 增量缓存已热时 `cargo check --workspace` 约 **9 秒**；冷启动仍需 15+ 分钟。

### 5.1 每个阶段完成后必须通过

```bash
# 1. 全工作区编译
export PATH="/c/Users/Administrator/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
cargo check --workspace

# 2. 绑定漂移检查（8 语言）
python scripts/sync_bindings.py --check

# 3. 注册表一致性
cargo test -p alpha-ta-core --lib test_docs_json_matches_registry

# 4. 审计脚本（本轮已修正）
python scripts/_a5_scan.py        # 生产 panic 必须恒为 0

# 5. 覆盖度（本轮新增）
python scripts/check_coverage.py   # batch=100%，流式声明口径>=98%，公式层只增不减
```

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 拆分 `functions.rs` 破坏公式求值 | 高 | 拆分前先补 golden 公式测试；纯机械移动、零行为变化 |
| 声明式注册改动影响别名（7 个函数有多别名） | 中 | 先为 11 个别名写断言测试 |
| core 解耦引发跨模块依赖断裂 | 中 | 逐 feature 验证编译；`REFACTORING_PLAN` 已记录 libm/类别依赖边等既有坑 |
| ~~流式覆盖数据不准导致误判工作量~~ | — | ✅ **已消除**：`check_coverage.py` 已交付，度量问题不再是风险 |
| **公式层接线时参数顺序/默认值错位** | **高** | 每接一个指标对照 batch 侧写数值断言（§3.6 7.4）；按类别分批独立 commit |
| 文档合并丢失历史决策 | 低 | 全部 `git mv` 到 `archive/`，不删除 |
| `check_coverage.py` 的启发式匹配产生假阴性 | 低 | `--strict` 模式给出下界；长期按 §3.5 5.2 在注册表补显式符号字段 |

---

## 7. 附：审计方法与可复现命令

本次审计的数据采集方式（供复核）：

```bash
# 规模分布
find core/src -name '*.rs' -exec wc -l {} + | sort -rn | head -26

# 生产危险点（已修正分桶）
python scripts/_a5_scan.py

# 覆盖度（三维度，已消除命名假阳性）
python scripts/check_coverage.py
python scripts/check_coverage.py --strict   # 下界估计
python scripts/check_coverage.py --json     # 供 CI 消费

# 孤儿函数检测
#   对比 `^fn (fn_*)` 与 `map.insert("X".to_string(), fn_y ... )`
#   注意注册有两种形式：直接传函数名 / 带 `as FormulaFn` 转换
#   —— 漏掉 `as FormulaFn` 形式会把已注册函数误判为孤儿（本次审计踩过）
```

**环境注意事项**：

- `cargo` **不在 bash 默认 PATH 里**，需先
  `export PATH="/c/Users/Administrator/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"`
- 冷启动 `cargo check --workspace` 约 **15+ 分钟**（132k LOC / 13 crates），务必后台运行；缓存热后约 9 秒
- clippy **本地已装**（同 toolchain 目录下有 `cargo-clippy`）
- **沙箱限制**：Python 内用 `subprocess` 调 `bash -c` 会被拦截并返回空结果，易误判为「零匹配」；改用纯 `os.walk` 遍历

---

*本方案取代此前 9 份计划文档。执行中请持续更新本文件，不要新建平行计划文档。*
