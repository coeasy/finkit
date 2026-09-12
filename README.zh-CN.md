# Finkit

**面向量化研究与实时分析的高性能多语言金融计算引擎。**

Finkit 以 Rust 作为统一计算内核，把技术指标、公式引擎、因子 DAG、流式计算、特征工程、研究分析与多语言 SDK 收敛到同一套数值语义和运行时契约中。

它适合成为量化研究工具、行情分析服务、选股/筛选系统、因子平台、数据流水线、实时看板以及多语言金融 SDK 的底层计算引擎，而不是把项目绑定到某个券商、交易终端或实盘执行系统。

[English README](README.md) · [产品说明](docs/product-overview-zh.md) · [宣传文稿](docs/promotion-zh.md) · [完整文档](docs/README.md)

当前正式发布版本：**v0.1.15**。

## 产品定位

Finkit 不是“再做一套指标函数集合”，而是希望解决量化系统里更长期的问题：

- 同一个指标在 Python、Rust、Node、Java、C++ 中重复实现，语义逐渐漂移；
- 公式、因子、流式、研究模块分别维护自己的 DAG、缓存与中间结果；
- 每次请求重新解析表达式、重新发现依赖、重新分配缓冲区；
- 增量执行只停留在接口命名，实际上仍然全量重算；
- 因子研究、特征工程、风险分析重复实现统计与回归内核；
- 多语言绑定有代码，却缺少统一的发布、验证和兼容合同。

Finkit 的答案是：**一个 Rust 核心、一套 canonical kernel、一套执行计划、一套运行时语义，再向不同语言和场景输出能力。**

## 核心价值

### 1. 一套内核，多种使用方式

批量指标、公式、因子、流式指标、研究计算都围绕同一个核心能力组织。多语言绑定是“交付层”，不是重新实现算法的第二套系统。

### 2. 从函数调用升级为可复用执行计划

公式和因子依赖可以提前编译、提前校验并重复执行。对于高频重复请求，不需要每次重新解析字符串和发现依赖关系。

### 3. 从技术指标扩展到完整研究计算链路

除传统技术分析外，Finkit 已包含或正在整合：

- Formula / terminal-style 公式；
- Factor DAG；
- Feature Engineering；
- Labeling；
- 统计、回归、排序、分位数；
- Risk / lightweight backtest；
- Purged KFold / Embargo / CPCV 等验证能力；
- Factor Research / Alphalens-style 研究分析；
- 多因子、组合、归因、容量、漂移与在线监控能力。

### 4. 性能优化不以牺牲语义为代价

Finkit 使用 SIMD、零拷贝/借用输入、`_into` 输出复用、流式状态、持久化执行计划、局部重算等优化，但前提是结果语义可被证明安全。

如果一个因子包含横截面依赖、动态 lookback 或其他无法安全局部执行的行为，运行时会保守回退到 full execution，而不会为了“看起来是增量”而产生错误结果。

## 产品能力地图

| 能力层 | 主要能力 | 当前状态 |
| --- | --- | --- |
| 技术分析 | 趋势、动量、波动率、成交量、周期、统计、价格变换、K 线/图形模式、A 股扩展 | 稳定核心 |
| Formula | 解析、编译、缓存、Bytecode/JIT、`eval_range`、`eval_last`、append | 稳定核心 |
| Streaming | 单 Bar 更新、状态保持、批量/流式一致性验证 | 稳定核心 |
| Feature | 多周期、lag/lead、rolling stats、归一化、标签、组合、选择、导出 | 稳定核心 |
| Factor | 命名因子、依赖 DAG、借用输入、`FactorPlan` | 稳定核心 / 持续收敛 |
| Unified Runtime | 统一执行边界、typed artifact、DirtyRange、full/range/into | 下一版本架构 |
| Factor Research | Prepare / Validate / Analyze / Multi-factor / Portfolio / Report | 下一版本扩展 |
| 多语言 | Rust、Python、CLI，以及 Node/Java/C/C++/Go/.NET/Android/iOS/WASM | 发布状态不同，详见文档 |

## 统一运行时

当前重构正在把 Factor / Research 的关键执行能力接入统一 Runtime：

```text
输入数据变化
    ↓
DirtyRange(input)
    ↓
依赖链传播 + 累计 lookback
    ↓
Affected Output Range
    ↓
Historical Recompute Window
    ↓
Unified Runtime
    ↓
只重算安全且真正受影响的区间
    ↓
写回 retained materialization
```

这里的关键不是增加一个 `range` API，而是建立完整的正确性边界：

- `ArtifactHash` 使用 typed、稳定、跨进程可复现的身份；
- `FactorPlan` 通过统一 Runtime 执行；
- `DirtyRange` 同时表达输入变化、输出影响和历史计算窗口；
- 只有完整依赖链都声明为 incremental + fixed lookback + time-series safe，才允许局部执行；
- cross-sectional / dynamic lookback / unknown contract 自动 full fallback。

## 典型场景

### 量化研究

使用指标、公式、特征、标签、因子和研究统计构建统一实验链路，减少 Notebook 和生产服务之间的语义差异。

### 行情分析 / 选股服务

将常用公式和因子预编译，重复处理大量证券/时间序列，降低重复解析和依赖发现成本。

### 实时分析

Streaming 指标可以逐 Bar 更新；对于可证明安全的依赖链，DirtyRange 运行时进一步减少局部数据修订后的重算范围。

### 特征与机器学习数据流水线

利用 lag、rolling stats、normalization、labels、mutual information、PCA、CV 等能力构建研究数据集，同时复用 canonical 数学内核。

### 多语言金融 SDK

在 Rust 内核中维护算法语义，再通过 Python、Node、Java、C/C++、Go、.NET、移动端和 WASM 接入不同产品，而不是维护多套独立实现。

## 快速开始：Python

v0.1.15 的权威二进制发行渠道是 GitHub Release。下载对应平台 wheel 后：

```bash
python -m pip install ./finkit-0.1.15-<platform>.whl
```

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, 12, 26, 9)

print(sma20[-1], rsi14[-1], macd[-1])
```

## 快速开始：Rust

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.15" }
```

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close: Vec<f64> = (1..=100).map(|v| v as f64).collect();
    let sma20 = moving_avg::sma(&close, 20)?;
    let rsi14 = indicators::rsi(&close, 14)?;
    println!("{:?} {:?}", sma20.last(), rsi14.last());
    Ok(())
}
```

## 数据与正确性约定

- 时间序列按最旧 → 最新排列；
- OHLCV 等关联数组必须保持严格对齐；
- rolling 输出保留输入长度，预热区使用 `NaN`；
- 多个输出组合时使用联合 finite mask，不要分别 drop 行；
- zero-copy 输入在同步计算期间不得 resize 或并发修改；
- 预测性研究必须满足 point-in-time / no-lookahead；
- 增量执行无法证明安全时必须 full fallback。

## 性能设计

Finkit 的性能策略不是只优化某个 `sma()` 函数，而是优化完整计算路径：

- SIMD hot kernels；
- `_into` caller-owned output；
- borrowed / zero-copy inputs；
- persistent compiled formula；
- streaming state；
- FactorPlan dependency reuse；
- DirtyRange local recompute；
- BufferArena / scratch 生命周期复用；
- CI benchmark / relative performance gates。

任何性能数据都受 CPU、编译器、特性开关和数据规模影响，因此仓库中的 benchmark 应视为可复现测试快照，而不是对所有机器的固定性能承诺。

## 多语言发布说明

Finkit 区分五种状态：

1. source exists；
2. CI validated；
3. package candidate；
4. GitHub Release asset；
5. public registry package。

一个语言已经有绑定源码，不代表对应 npm / Maven / NuGet / Go module / Android Maven / Swift Package 已公开发布。

完整状态请看 [docs/language-bindings.md](docs/language-bindings.md)。

## Finkit 不做什么

Finkit 不定位为：

- OMS；
- 券商交易接口；
- 撮合引擎；
- 交易所 Gateway；
- 一站式实盘交易平台。

Finkit 专注的是这些系统共同需要的 **金融计算、因子研究、实时分析和多语言运行时底座**。

## 进一步阅读

- [产品说明](docs/product-overview-zh.md)
- [宣传文稿](docs/promotion-zh.md)
- [完整文档索引](docs/README.md)
- [Runtime 与 Factor](docs/runtime-and-factors.md)
- [Factor Research 架构](docs/factor-research-architecture.md)
- [Formula Runtime](docs/formula-runtime.md)
- [多语言绑定](docs/language-bindings.md)
- [性能基准](docs/benchmark-results.md)

## License

Finkit 使用 MIT OR Apache-2.0 双许可证。详见 [LICENSE-MIT](LICENSE-MIT) 与 [LICENSE-APACHE](LICENSE-APACHE)。
