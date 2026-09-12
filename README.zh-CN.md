# Finkit

**面向量化研究、实时分析与多语言产品集成的高性能金融计算引擎。**

Finkit 以 Rust 作为统一计算内核，把技术指标、公式、因子 DAG、流式计算、特征工程、研究分析和多语言 SDK 收敛到同一套数值语义与运行时契约中。

它不是又一套“指标函数集合”，而是一层可复用的 **Quant Compute Runtime**：让研究 Notebook、行情服务、选股系统、因子平台、实时看板、数据流水线和多语言产品共享同一套金融计算能力。

[English README](README.md) · [产品说明](docs/product-overview-zh.md) · [宣传文稿](docs/promotion-zh.md) · [品牌与媒体素材](docs/media-kit-zh.md) · [完整文档](docs/README.md)

当前正式发布版本：**v0.1.15**。PR #29 中的 Unified Runtime / Factor Research 扩展属于下一版本候选能力，只有同一 SHA 全部门禁通过后才进入发布判断。

## Finkit 帮你解决什么

量化系统真正难维护的，通常不是“少一个指标”，而是同一套计算逻辑在不同模块、语言和生命周期里不断分叉：研究代码一套、实时计算一套、因子平台一套、SDK 又一套；公式重复解析、依赖重复发现、缓存各自为政、数据修订后整段重算，最终带来语义漂移、性能浪费和验证成本。

Finkit 的产品原则很简单：**一个 Rust 核心、一套 canonical kernel、一套执行计划、一套正确性语义，再面向不同场景和语言交付。**

## 为什么团队会选择 Finkit

| 产品价值 | 对使用者意味着什么 |
| --- | --- |
| 统一计算语义 | Python、Rust、CLI 以及其他绑定共享同一核心算法，减少跨语言结果漂移 |
| 可复用执行计划 | 公式与因子依赖提前编译、校验和复用，不必每次重新解析与发现 DAG |
| 批量与实时统一 | 批量计算、流式状态、局部重算围绕同一数值合同组织 |
| 研究链路更完整 | 从指标、公式、特征、标签到因子研究、风险和验证能力逐步贯通 |
| 性能可解释 | SIMD、零拷贝、`_into`、Streaming、DirtyRange 等优化都有明确安全边界 |
| 多语言可交付 | 算法留在 Rust 核心，语言绑定负责产品接入，而不是复制算法实现 |

## 你可以用 Finkit 构建什么

**量化研究平台**：统一计算指标、公式、特征、标签、因子和研究统计，减少 Notebook 与生产服务之间的实现差异。

**行情分析与选股服务**：预编译常用公式和因子，重复处理大量证券和时间序列，降低重复解析与依赖发现成本。

**实时分析系统**：Streaming 指标逐 Bar 更新；对于能够证明安全的依赖链，DirtyRange 进一步缩小数据修订后的重算范围。

**因子与机器学习流水线**：复用 rolling statistics、normalization、labels、regression、mutual information、PCA、CV 等 canonical 内核，避免研究模块重复造轮子。

**多语言金融 SDK**：在一个 Rust 核心里维护数值语义，再面向 Python、Node、Java、C/C++、Go、.NET、移动端和 WASM 输出产品接口。

## 产品能力地图

| 能力层 | 主要能力 | 当前定位 |
| --- | --- | --- |
| Technical Analysis | 趋势、动量、波动率、成交量、周期、统计、价格变换、形态、A 股扩展 | 稳定核心 |
| Formula Engine | Parser、Compiler、Bytecode/JIT、缓存、`eval_range`、`eval_last`、append | 稳定核心 |
| Streaming Engine | 单 Bar 更新、状态保持、批量/流式一致性验证 | 稳定核心 |
| Feature Engineering | lag/lead、rolling stats、归一化、标签、组合、选择、导出 | 稳定核心 |
| Factor Engine | 命名因子、依赖 DAG、借用输入、`FactorPlan` | 稳定核心 / 持续收敛 |
| Unified Runtime | typed artifact、full/borrowed/range/into、DirtyRange | 下一版本候选 |
| Factor Research | Prepare、Validate、Analyze、Multi-factor、Portfolio、Report | 下一版本扩展 |
| Multi-language Delivery | Rust、Python、CLI 以及多种 native/mobile/WASM 绑定 | 发布状态分层 |

## 统一运行时：为什么重要

Finkit 正在把“执行”本身变成产品能力，而不是每个模块各自调用函数：

```text
Market data / arrays
        │
        ▼
MarketFrame / typed inputs
        │
        ├──────── Formula
        ├──────── Factors
        ├──────── Streaming
        └──────── Research
                 │
                 ▼
          ComputePlan / FactorPlan
                 │
                 ▼
           Unified Runtime
      full / borrowed / range / into
                 │
       ┌─────────┴─────────┐
       ▼                   ▼
   retained state      typed artifacts
```

下一版本候选架构已经引入 typed `ArtifactHash`、`FactorPlan → UnifiedRuntime` 和真正的 `DirtyRange` 局部执行。只有完整依赖链都能证明 `incremental + fixed lookback + time-series safe` 时才允许局部重算；横截面、动态 lookback 或未知合同会保守回退到 full execution。

这意味着 Finkit 优先保证 **结果正确，再追求更快**。

## 选择路径

如果你主要做研究，先看 [完整使用指南](docs/usage.md) 与 [Factor Research 架构](docs/factor-research-architecture.md)。如果你要把计算能力嵌入服务或产品，先看 [Runtime 与 Factor](docs/runtime-and-factors.md) 与 [多语言绑定](docs/language-bindings.md)。如果你正在评估是否适合团队采用，先读 [产品说明](docs/product-overview-zh.md)。如果你需要对外介绍项目，可直接复用 [宣传文稿](docs/promotion-zh.md) 和 [品牌与媒体素材](docs/media-kit-zh.md)。

## 快速开始

Python 的 v0.1.15 权威二进制发行渠道是 GitHub Release。下载对应平台 wheel 后：

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

Rust 可直接使用发布 tag：

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.15" }
```

## 正确性与性能原则

Finkit 的性能优化覆盖完整执行路径：SIMD kernels、borrowed/zero-copy inputs、caller-owned `_into` outputs、persistent compiled plans、streaming state、dependency reuse、DirtyRange local recompute 与 buffer 生命周期复用。

但所有优化都受正确性合同约束：时间序列按最旧到最新排列，OHLCV 必须对齐，rolling 输出保留 warm-up `NaN`，预测研究必须满足 point-in-time / no-lookahead，无法证明安全的增量路径必须 full fallback。

严格性能回归由专用 benchmark / relative-performance 门禁承担；普通单元测试不应把共享 CI runner 的短时抖动误判为产品性能回归。

## 多语言发布不是一个布尔值

Finkit 区分 source exists、CI validated、package candidate、GitHub Release asset、public registry package。一个语言存在绑定源码，不代表已经公开发布到 npm、Maven、NuGet、Go module、Android Maven 或 Swift Package。

准确状态请以 [多语言绑定说明](docs/language-bindings.md) 为准。

## Finkit 的边界

Finkit 不定位为 OMS、券商交易接口、交易所 Gateway、撮合引擎或一站式实盘平台。它专注于这些系统都会依赖的 **金融计算、因子研究、实时分析与多语言运行时底座**。

## License

Finkit 使用 MIT OR Apache-2.0 双许可证。详见 [LICENSE-MIT](LICENSE-MIT) 与 [LICENSE-APACHE](LICENSE-APACHE)。
