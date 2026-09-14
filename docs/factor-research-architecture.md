# Finkit Factor Research Architecture — Reuse-First Expansion Plan

> 状态：Canonical architecture / refactor plan  
> 基线：`main`，workspace `0.1.15`  
> Alphalens 兼容基线：`alphalens-reloaded==0.4.6`  
> 原则：**优先复用、单一事实源、兼容迁移、No-Lookahead、先门禁后扩展**。  
> 本文取代早期 `factor-research-expansion-plan-v2.md`，后续 Factor Research / Alpha Research 的设计与实施以本文为准。

---

## 1. 总目标

Finkit 不再只把“因子研究”理解为 Alphalens API 的 Rust 重写，而是把现有指标、Formula、Factor、Feature、Risk、Backtest、Calendar、Visualization、FFI 能力重新组合成一个统一的 **Factor Research Layer**。

目标链路：

```text
MarketFrame / external data
        ↓
Indicators / Formula / Composite / FactorEngine
        ↓
FeatureMatrix + PanelIndex
        ↓
ResearchFrame / FactorPanelView
        ↓
Prepare / Validate / Analyze
        ↓
Multi-Factor / Neutralization / Portfolio / Risk / Event
        ↓
FactorStudyReport
        ↓
Rust / Python / CLI / JSON / HTML / Web / compact FFI
```

本轮扩展需要同时满足：

1. 覆盖 Alphalens 全部核心研究能力；
2. 增加多因子、风险模型、组合、归因、容量、统计验证、因子挖掘、在线监控等能力；
3. 尽可能复用当前代码，不复制已有算法；
4. 把已经存在的重复实现收敛到 canonical kernel；
5. 保持现有公开 API 可用，优先内部委托而不是破坏式删除；
6. 不把 Finkit 扩成 OMS / 撮合 / 实盘交易系统，研究组合只服务因子评价和组合模拟；
7. 所有预测性研究都必须通过 no-lookahead / point-in-time 门禁。

---

## 2. 当前已有能力与复用策略

当前仓库已经具备大量可直接复用的基础，不应重新实现。

| 能力 | 当前实现 | 新研究层处理方式 |
| --- | --- | --- |
| 统一计算规划 | `core/src/compute.rs`：`ComputePlan` / `FactorPlan` | **直接复用**拓扑排序、依赖验证、能力描述；禁止再做第四套 DAG planner |
| Factor DAG | `core/src/factors.rs` | 继续负责因子定义、依赖、方向、批量计算；研究层只消费结果 |
| Composite DAG | `core/src/composite.rs` | 继续负责组合表达式及 revision cache；因子挖掘不重新做表达式解释器 |
| Formula runtime | `core/src/formula/` | 继续作为公式候选因子的生成入口，不新建 research DSL |
| Scratch memory | `core/src/buffer_arena.rs` | 研究热路径优先复用 `BufferArena`，禁止再建 ad-hoc `Vec` 池 |
| 交易日历 | `core/src/calendar.rs` | Forward Return、Event、Holding Period 全部复用，不引入第二套 calendar semantics |
| 基础统计 | `core/src/math/statistics.rs` | 作为 mean/variance/correlation/Spearman/Kendall 等 SSOT 基础 |
| 线性回归 | `core/src/math/linear.rs` | 扩展为回归 canonical kernel；其它模块不保留私有重复回归 |
| Feature 容器 | `core/src/features/matrix.rs` | 继续作为 numeric column 容器；研究层增加 index/view，而不是复制一套 matrix |
| CV | `core/src/features/cv_split.rs` | Purged KFold、Embargo、CPCV、WalkForward 直接复用并扩展 interval-aware 版本 |
| Stability | `core/src/features/stability.rs` | PSI / CSI / rolling PSI 可直接用于 factor drift；统一 turnover 命名语义 |
| Regime | `core/src/features/regime.rs` | HMM / regime signal 复用；预测模式增加 fit/transform 防止全样本阈值泄漏 |
| PCA | `core/src/features/pca.rs` | Multi-factor 降维与冗余分析直接复用 |
| Feature Selection | `features/selection.rs` | API 保留，底层改委托统一 MI/correlation kernel |
| Importance | `features/importance.rs` | API 保留，与 selection 共用 MI/discretization |
| Labeling | `features/labels.rs` | forward return / triple barrier / fixed horizon 复用 shared return kernel |
| Meta Label | `features/meta_labels.rs` | 直接接入事件研究与 ML validation |
| Parallel | `features/parallel.rs` | 沿用 `rayon` feature；研究层增加 segmented executor 而不是另一套线程池 |
| Portfolio risk | `core/src/risk.rs` | Sharpe/Sortino/Drawdown/VaR/CVaR 作为 portfolio/report SSOT |
| Lightweight backtest | `core/src/backtest.rs` | 保留 signal validation；风险统计改委托 `risk.rs`；不替代 factor holding engine |
| Metrics | `core/src/metrics.rs` | 扩展 research counters/histograms，不创建新的可观测性体系 |
| Store | `features/store.rs` | 复用 `FeatureStore` 接口；Research Artifact Store 作为 typed adapter |
| Visualization | `visualization/` | 只消费 report model，不拥有研究算法 |

---

## 3. 已确认需要优先消除的重复实现

### 3.1 Mutual Information / discretization 重复

目前：

- `features/selection.rs` 自己实现 `compute_mi` + `discretize`；
- `features/importance.rs` 又实现一套 `compute_mi` + `discretize`。

目标：

```text
math::information::mutual_information_*
math::quantile::discretize
        ↑
selection.rs
importance.rs
factor-analysis/multifactor
factor-analysis/mining
```

现有 public API 不删除，只改为薄包装。

### 3.2 Regression 重复

目前：

- `math/linear.rs` 已有正式 linear regression；
- `features/rolling_stats.rs` 存在私有 `linear_regression_slope`；
- `factors.rs::neutralize` 又手写单 exposure OLS。

目标：

```text
math::regression
  ├─ simple_ols
  ├─ multi_ols
  ├─ weighted_ls
  ├─ residualize
  ├─ rolling_slope
  └─ diagnostics
```

`math::linear` 保持兼容 facade；Hurst、neutralization、alpha/beta、Fama-MacBeth 全部复用这一内核。

### 3.3 Return 计算重复

目前至少存在：

- `factors::time_series_return`；
- `features::labels::forward_return`；
- `features::labels::forward_return_arithmetic`；
- `backtest.rs` 内部 bar return；
- `regime.rs` 内部 log return。

新增 canonical：

```text
core/src/returns.rs
  ├─ simple_returns
  ├─ log_returns
  ├─ lagged_return
  ├─ forward_return
  ├─ forward_returns_many
  └─ cumulative_return
```

通过 policy 区分 arithmetic/log、lag/lead、NaN、zero denominator、execution lag，不再复制循环。

### 3.4 Correlation / rank 重复

目前：

- `math/statistics.rs` 有 Pearson / Spearman / Kendall；
- `features/selection.rs` 有私有 Pearson；
- `features/combinations.rs` 有 rolling correlation；
- Factor/Research 未来还会大量使用 rank/correlation。

目标：

```text
math::rank
math::statistics
math::rolling
        ↑
features
factors
factor-analysis
```

必须只有一套 tie-aware fractional rank 实现。

### 3.5 Quantile / percentile / binning 重复

目前 percentile、排序、histogram/bins 分散在 `factors.rs`、`normalization.rs`、`stability.rs`、`regime.rs`、`selection.rs`、`importance.rs`。

新增统一：

```rust
pub enum TiePolicy { Average, Min, Max, First, Dense }
pub enum QuantileInterpolation { Linear, Lower, Higher, Midpoint, Nearest }
pub enum BinPolicy { EqualWidth, EqualFrequency, ExplicitEdges }
```

所有 Alphalens compatibility 差异通过 policy 表达，不通过复制函数表达。

### 3.6 Risk metric 重复

`backtest.rs` 当前内部再次计算 Sharpe / Sortino / Max Drawdown，而 `risk.rs` 已有同类正式 API。

目标：

- `risk.rs` 为唯一实现；
- backtest 只生成 `strat_returns` / equity / trades；
- report / factor portfolio / capacity 同样调用 `risk.rs`；
- annualization 不允许散落硬编码。

### 3.7 Graph / cache 不再扩散

当前已有：

- `ComputePlan`；
- `FactorPlan`；
- `FactorEngine` dependency cache；
- `CompositeEngine` revision cache；
- Formula execution plan。

研究层禁止再实现 topology sort / cycle detection。

Research orchestration 应采用：

```rust
pub struct ResearchPlan {
    graph: ComputePlan,
    stages: BTreeMap<ComputeNodeId, ResearchStage>,
}
```

Research-specific stage metadata独立保存，但依赖排序、cycle gate、determinism/streaming capability 直接复用 `ComputePlan`。

如需要跨请求缓存，先从 `CompositeEngine` 的 revision/signature 机制抽取通用 `BoundedRevisionCache<K, V>`，再由 Composite 与 Research 共用。

---

## 4. 新架构总览

建议只新增一个主要 workspace crate：

```text
factor-analysis/
```

避免把研究层拆成十几个 crates，降低 workspace、release、FFI 与版本同步成本。

依赖方向：

```text
                 ┌───────────────────┐
                 │     core/finkit   │
                 │ math / factor /   │
                 │ formula / feature │
                 │ risk / calendar   │
                 └─────────┬─────────┘
                           │
                 ┌─────────▼─────────┐
                 │ finkit-factor-    │
                 │ analysis          │
                 └───────┬───────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
 visualization       python/ffi        cli/json
```

禁止反向依赖：`core` 不依赖 `factor-analysis`。

---

## 5. Core 层新增的共享 primitive

为了让现有 Features 和新 Research 共用逻辑，新增的真正通用算法应落到 core，而不是 factor-analysis。

建议：

```text
core/src/
  returns.rs
  math/
    rank.rs
    quantile.rs
    regression.rs
    information.rs
    segmented.rs
```

### 5.1 `math::segmented`

所有横截面操作的核心都不是 pandas `groupby`，而是预计算 segment offsets：

```rust
pub struct SegmentLayout {
    offsets: Vec<usize>,
}

impl SegmentLayout {
    pub fn range(&self, segment: usize) -> Range<usize>;
}
```

复用场景：

- date-wise rank；
- date/group quantile；
- group demean；
- IC；
- quantile return；
- exposure regression；
- group attribution；
- turnover。

增加：

```rust
pub fn for_each_segment(...)
pub fn map_segments(...)
pub fn reduce_segments(...)
pub fn par_map_segments(...)
```

Rayon feature 打开时走 parallel；否则 serial fallback，沿用当前 feature 策略。

### 5.2 统一 numerical semantics

避免 shared kernel 合并之后改变旧 API 行为，需要显式 policy：

```rust
pub struct NumericSemantics {
    pub nan_policy: MissingPolicy,
    pub variance_ddof: usize,
    pub tie_policy: TiePolicy,
    pub quantile_interpolation: QuantileInterpolation,
    pub zero_division: ZeroDivisionPolicy,
}
```

旧 API 传 `Legacy` policy；Alphalens adapter 传 `AlphalensCompat` policy；新 Rust API 使用 `Native` policy。

---

## 6. Research 数据模型：复用 FeatureMatrix，不再复制通用矩阵

`FeatureMatrix` 已经是 column-oriented numeric feature container。研究层不要再创建一个同能力的 `Vec<Vec<f64>>`。

新增索引与视图：

```rust
pub struct PanelIndex {
    pub timestamps: Vec<i64>,
    pub assets: Vec<AssetId>,
    pub date_segments: SegmentLayout,
}

pub struct ResearchFrame {
    pub index: PanelIndex,
    pub numeric: FeatureMatrix,
    pub groups: CategoricalFrame,
    pub validity: ValidityMap,
}

pub struct ResearchFrameView<'a> { /* borrowed */ }
pub struct FactorPanelView<'a> { /* factor column + index + returns */ }
```

`numeric` 可保存：

- factor columns；
- forward returns；
- numeric exposures；
- weights；
- residuals；
- derived metrics。

`groups` 保存 dictionary-encoded：

- industry；
- country；
- exchange；
- style bucket；
- custom group。

### 6.1 Multi-factor 不建第二套容器

```text
ResearchFrame.numeric
  ├─ factor_value
  ├─ value
  ├─ momentum
  ├─ quality
  ├─ size
  ├─ return_1d
  ├─ return_5d
  └─ beta
```

单因子 `FactorPanelView` 只是对其中一列的 view。

### 6.2 Borrowed-first

Python/Arrow/Polars/外部 owner 优先构建 borrowed view；只有生命周期或 reorder 必须时才 materialize。

禁止：

```text
NumPy → Vec → DataFrame → Vec → Report
```

目标：

```text
NumPy/Arrow → borrowed column view → Rust kernel → owned final outputs
```

---

## 7. Prepare / Data Quality 模块

目录：

```text
factor-analysis/src/prepare/
  alignment.rs
  universe.rs
  forward_returns.rs
  quantize.rs
  clean.rs
  groups.rs
  quality.rs
```

### 7.1 Point-in-time universe

新增：

```rust
pub struct UniverseSnapshot { ... }
pub trait UniverseProvider { ... }
pub struct EffectiveDatedGroupMap { ... }
```

支持：

- 动态股票池；
- 上市/退市；
- effective-dated industry；
- as-of joins；
- missing asset；
- sparse factor observation。

禁止把未来行业分类/未来股票池成员回填到历史。

### 7.2 ForwardReturnEngine

底层调用 `core::returns`，面板层只负责 calendar/index mapping。

```rust
pub enum ForwardPeriod {
    Bars(u32),
    Duration(CalendarDuration),
}

pub enum PriceConvention {
    ProvidedPrice,
    SameBarClose,
    NextBarOpen,
    NextAvailable,
}
```

必须支持：

- 1/5/10/20 bars；
- intraday 30m/3h；
- custom trading calendar；
- cumulative / non-cumulative；
- explicit execution lag；
- Alphalens compatibility semantics。

### 7.3 DataQualityReport

新增结构化质量报告，不在内核里打印：

```rust
pub struct DataQualityReport {
    pub input_rows: usize,
    pub output_rows: usize,
    pub missing_factor: usize,
    pub missing_price: usize,
    pub missing_group: usize,
    pub invalid_numeric: usize,
    pub duplicate_keys: usize,
    pub forward_return_loss: usize,
    pub quantization_loss: usize,
    pub total_loss_ratio: f64,
}
```

扩展：

- coverage by date；
- coverage by group；
- stale price count；
- cross-sectional breadth；
- duplicate `(date, asset)`；
- timestamp ordering；
- large-gap diagnostics；
- survivorship-risk warning；
- point-in-time warning。

---

## 8. Alphalens Compatibility 模块

保持之前目标：`alphalens-reloaded==0.4.6` numerical parity。

目录：

```text
factor-analysis/src/compat/alphalens/
  utils.rs
  performance.rs
  tears.rs
  schema.rs
```

不要复制核心算法：compat 层只负责参数解释、默认值、DataFrame schema、错误行为、输出命名。

内部全部委托 Native analyzers：

```text
alphalens::factor_information_coefficient
       ↓
InformationAnalyzer::ic + AlphalensSemantics
```

```text
alphalens::factor_weights
       ↓
PortfolioWeightKernel + AlphalensSemantics
```

这样 Alphalens 与 Native API 不形成两套算法。

---

## 9. Returns / IC / Turnover / Event 基础研究模块

```text
factor-analysis/src/analysis/
  returns.rs
  information.rs
  turnover.rs
  event.rs
  decay.rs
  diagnostics.rs
```

### 9.1 Returns

包含：

- factor weights；
- factor returns；
- mean return by quantile；
- quantile spread；
- cumulative returns；
- alpha / beta；
- by-group returns；
- by-date returns；
- equal-weight / factor-weight / rank-weight。

Risk summary 直接调用 `core::risk`。

### 9.2 Information Coefficient

支持：

- Spearman IC；
- Pearson IC；
- Kendall IC；
- mean IC；
- IC by group；
- IC by time；
- rolling IC；
- conditional IC；
- partial IC；
- IC decay curve；
- IC breadth/coverage。

Factor ranks 对多个 forward horizons 只计算一次。

### 9.3 HAC / Newey-West inference

Alphalens 之外必须增加：

- autocorrelation-aware standard errors；
- Newey-West t-stat；
- confidence intervals；
- overlapping forward-return aware inference。

长 holding horizon 的 5D/10D/20D forward returns 高度重叠，不能只依赖 IID t-stat。

### 9.4 Quantile diagnostics

新增：

- monotonicity score；
- top-bottom consistency；
- quantile slope；
- quantile hit ratio；
- tail asymmetry；
- quantile dispersion；
- group monotonicity。

### 9.5 Turnover

严格区分两个概念：

1. **rank movement turnover**：当前 `features::stability::turnover_ratio` 风格；
2. **quantile membership turnover**：Alphalens 集合成员进出比例。

两个 API 禁止共用模糊的 `turnover_ratio` 名称。

### 9.6 Event Study

复用 `labels/meta_labels`：

- common-start returns；
- pre/post event return；
- average cumulative event return；
- positive/negative event；
- event magnitude；
- group event；
- event distribution；
- triple-barrier event outcome；
- meta-label conditional event analysis。

---

## 10. Multi-Factor Research

目录：

```text
factor-analysis/src/multifactor/
  correlation.rs
  redundancy.rs
  pca.rs
  orthogonalize.rs
  conditional.rs
  combination.rs
  fama_macbeth.rs
```

### 10.1 直接复用现有能力

- PCA → `features::IncrementalPCA`；
- correlation → shared statistics kernel；
- static weighted composite → `FactorEngine::composite` / CompositeEngine；
- feature importance → shared MI kernel；
- feature selection →现有 selection facade。

### 10.2 新增功能

- factor × factor Pearson/Spearman matrix；
- factor × horizon IC matrix；
- VIF；
- redundancy graph；
- hierarchical clustering；
- PCA loading / explained variance；
- orthogonalization；
- incremental IC；
- partial IC；
- conditional IC；
- marginal factor contribution；
- factor crowding correlation；
- factor ensemble stability。

### 10.3 Fama-MacBeth

增加 cross-sectional factor premium estimation：

```text
每个日期 cross-sectional regression
        ↓
时间序列 factor premia
        ↓
mean premium + HAC t-stat
```

复用统一 multivariate regression + segmented executor。

---

## 11. Exposure / Neutralization / Risk Model

当前 `factors::neutralize` 只处理一个连续 exposure。

新模块：

```text
factor-analysis/src/exposure/
  matrix.rs
  neutralize.rs
  risk_model.rs
  attribution.rs
```

支持：

- industry fixed effects；
- log market cap；
- beta；
- volatility；
- liquidity；
- momentum/style exposures；
- user-defined numeric/categorical exposure；
- multi-exposure OLS；
- WLS；
- ridge fallback；
- residual factor；
- residual return。

风险模型：

```text
Asset Return ≈ Exposure × Factor Return + Specific Return
```

输出：

- factor covariance；
- specific variance；
- portfolio factor exposure；
- factor risk contribution；
- specific risk contribution；
- tracking error decomposition。

基础 volatility/VaR/CVaR/Drawdown 继续调用 `core::risk`，不复制。

---

## 12. Validation / Overfitting Control

目录：

```text
factor-analysis/src/validation/
  split.rs
  leakage.rs
  bootstrap.rs
  permutation.rs
  multiple_testing.rs
  significance.rs
  pbo.rs
```

### 12.1 复用 `features::cv_split`

直接封装：

- `PurgedKFold`；
- `EmbargoKFold`；
- `CombinatorialPurgedCV`；
- `WalkForwardSplit`。

### 12.2 扩展 interval-aware purge

当前 index gap purge 对固定 horizon 很有用，但研究事件可能有 `[label_start, label_end]` 区间。

新增：

```rust
pub struct LabelInterval {
    start: i64,
    end: i64,
}
```

训练样本与 test label interval 相交时必须 purge。

### 12.3 统计显著性

新增：

- bootstrap CI；
- block bootstrap；
- stationary bootstrap；
- permutation test；
- Bonferroni；
- Holm；
- Benjamini-Hochberg FDR；
- Probabilistic Sharpe Ratio；
- Deflated Sharpe Ratio；
- Probability of Backtest Overfitting（结合 CPCV）；
- optional White Reality Check / SPA extension。

所有随机算法必须接受 deterministic seed。

---

## 13. Portfolio Construction

不要把 `backtest.rs` 扩展成庞大的多资产引擎。

新增独立研究组合模块：

```text
factor-analysis/src/portfolio/
  target.rs
  weights.rs
  holdings.rs
  constraints.rs
  rebalance.rs
  optimizer.rs
  turnover.rs
```

### 13.1 Target weights

支持：

- factor proportional；
- rank proportional；
- equal weight；
- top/bottom quantile；
- long only；
- long/short；
- gross/net target；
- group neutral；
- beta neutral；
- volatility scaling。

### 13.2 Constraints

- max asset weight；
- min/max gross；
- net exposure；
- group exposure；
- beta exposure；
- liquidity cap；
- turnover cap；
- position count；
- long/short count。

### 13.3 HoldingPeriodPortfolioEngine

专门实现 Alphalens-style overlapping cohorts：

```text
rebalance t0 ───────── expires t0+h
     rebalance t1 ───────── expires t1+h
          rebalance t2 ───────── expires t2+h
```

使用 ring/deque 管理 active cohorts，而不是每个 timestamp concat 全部历史权重。

### 13.4 Optimizer interface

```rust
pub trait PortfolioOptimizer {
    fn optimize(&self, input: &OptimizationInput) -> Result<TargetWeights>;
}
```

第一阶段只实现无重依赖版本：

- normalize/project constraints；
- inverse volatility；
- risk parity approximation；
- score-risk blend。

Mean-Variance / QP 后端作为 optional feature，不让默认 core 拉入重型 solver。

---

## 14. Transaction Cost / Capacity / Liquidity

```text
factor-analysis/src/cost/
  model.rs
  linear.rs
  impact.rs
  capacity.rs
  liquidity.rs
```

接口：

```rust
pub trait CostModel {
    fn estimate(&self, trade: &TradeIntent, market: &LiquiditySnapshot) -> f64;
}
```

模型：

- commission；
- half-spread；
- fixed bps slippage；
- ADV participation；
- square-root market impact；
- user-defined cost model。

研究输出：

- gross factor return；
- turnover cost；
- net factor return；
- break-even bps；
- capacity curve；
- return vs AUM；
- max practical participation；
- liquidity concentration。

`backtest::BacktestConfig` 的 commission/slippage 可通过 adapter 映射到简单 `CostModel`，避免第三套成本定义。

---

## 15. Attribution / Scenario / Crowding

新增：

```text
factor-analysis/src/risk/
  attribution.rs
  scenario.rs
  crowding.rs
  concentration.rs
```

### Attribution

- factor contribution to return；
- group contribution；
- asset contribution；
- marginal contribution；
- active return attribution；
- active risk attribution。

### Scenario

- historical stress window；
- factor shock；
- sector shock；
- volatility multiplier；
- liquidity haircut；
- correlation shock。

### Crowding / concentration

- HHI；
- effective number of bets；
- top-N concentration；
- factor exposure concentration；
- liquidity concentration；
- common-holdings overlap；
- long/short crowding proxy。

---

## 16. Stability / Decay / Regime / Live Factor Health

复用当前 PSI/CSI/HMM，增加研究级 API：

```text
factor-analysis/src/stability/
  distribution.rs
  ic.rs
  decay.rs
  regime.rs
  health.rs
```

指标：

- PSI / CSI；
- rolling IC；
- rolling ICIR；
- rank autocorrelation；
- IC decay；
- estimated half-life；
- rolling turnover；
- factor coverage drift；
- quantile distribution drift；
- group composition drift；
- regime conditional IC；
- regime conditional spread；
- structural degradation score。

### 16.1 Regime 的 no-lookahead 修复原则

当前 threshold regime 使用全样本 percentile 可用于 **ex-post descriptive analysis**，但不能直接当成 OOS predictive transform。

新增：

```rust
pub trait FitTransform {
    type Model;
    fn fit(train: ...) -> Self::Model;
    fn transform(model: &Self::Model, test: ...) -> ...;
}
```

预测/validation 模式下阈值只能在 train segment 拟合。

---

## 17. Factor Mining / Candidate Research

不创建新的公式语言。

候选生成统一来自：

- Formula；
- CompositeEngine；
- FactorRegistry；
- FeatureSet；
- external numeric factors。

目录：

```text
factor-analysis/src/mining/
  candidate.rs
  fingerprint.rs
  screening.rs
  search.rs
  ensemble.rs
```

功能：

- candidate fingerprint；
- duplicate formula detection；
- equivalent factor detection；
- correlation/redundancy screening；
- minimum coverage gate；
- minimum turnover/capacity gate；
- OOS IC gate；
- multiple-testing correction；
- top-K selection；
- ensemble construction。

后续 Symbolic / Genetic / Bayesian / LLM search 只能作为 **candidate generator adapter**，不得绕过同一 validation pipeline。

---

## 18. Research Orchestration / CSE / Cache

目录：

```text
factor-analysis/src/orchestration/
  plan.rs
  stage.rs
  cache.rs
  fingerprint.rs
  incremental.rs
  provenance.rs
```

### 18.1 不创建新的 DAG 算法

`ResearchPlan` 复用 `ComputePlan`：

```text
Align
 ├─ ForwardReturns
 └─ GroupMap
       ↓
Clean
 ├─ Quantize
 ├─ Rank
 └─ ExposureMatrix
       ↓
Returns / IC / Turnover / Neutralization
       ↓
Report
```

相同 stage config 通过 fingerprint intern，实现 research-level CSE。

例如 Full Report 中：

- quantile assignment 只做一次；
- factor ranks 只做一次；
- forward return 1D/5D/10D matrix 只做一次；
- group date offsets 只做一次；
- demeaned return 只做一次；
- factor weights 只做一次。

### 18.2 Cache key

```rust
pub struct ResearchArtifactKey {
    pub data_revision: u64,
    pub data_fingerprint: Fingerprint,
    pub plan_fingerprint: Fingerprint,
    pub semantics_fingerprint: Fingerprint,
    pub calendar_fingerprint: Fingerprint,
    pub universe_fingerprint: Fingerprint,
}
```

禁止只用“函数名 + 日期”缓存。

### 18.3 Cache levels

- L0：scratch `BufferArena`；
- L1：single-study stage cache；
- L2：revision-aware in-memory cache；
- L3：FeatureStore-backed artifacts；
- L4：optional Arrow/Parquet persistent artifacts。

Research storage 应包装现有 `FeatureStore`，不再造一个完全独立的 save/load/version API。

---

## 19. Incremental / Streaming Research

研究不只支持一次性 batch。

新增：

```rust
pub trait IncrementalStudy {
    fn append_session(&mut self, ...);
    fn update_latest(&mut self, ...);
    fn finalize_matured_horizons(&mut self, ...);
}
```

关键规则：

- 新 bar 只更新受影响 horizon；
- 5D forward return 只在第 5 个未来 session 到来后 mature；
- rolling IC 只更新尾部窗口；
- turnover 只比较最新 relevant snapshot；
- active holding cohorts 使用 ring buffer；
- PSI/CSI 可基于 frozen baseline 增量更新；
- report summary 可增量刷新。

与现有 streaming indicator 的状态机语义保持一致：未成熟输出明确处于 warmup/pending，而不是填假值。

---

## 20. Experiment / Provenance

每一次研究都应该可以重现。

```rust
pub struct StudySpec { ... }
pub struct StudyProvenance {
    pub library_version: String,
    pub data_fingerprint: Fingerprint,
    pub factor_fingerprint: Fingerprint,
    pub universe_fingerprint: Fingerprint,
    pub calendar_fingerprint: Fingerprint,
    pub config_fingerprint: Fingerprint,
    pub random_seed: Option<u64>,
}
```

记录：

- factor/formula definition；
- periods；
- quantile/bin config；
- group map version；
- neutralization exposures；
- validation split；
- cost model；
- portfolio constraints；
- AnalysisMode；
- calendar definition；
- random seed。

不得把 provenance 依赖 Python notebook 隐式状态。

---

## 21. Registry / SSOT 复用

现有 `FunctionRegistry` 已经解决：

- canonical name；
- alias；
- category；
- parameters；
- lookback；
- streaming；
- deterministic metadata。

研究层不要复制一套 alias/name collision 逻辑。

推荐抽取内部 generic registry core：

```rust
pub struct RegistryCore<S> { ... }
```

保持：

```rust
pub struct FunctionRegistry { ... }
```

兼容不变，再新增：

```rust
pub struct ResearchRegistry { ... }
```

两者共享：

- name normalization；
- alias validation；
- deterministic order；
- duplicate detection。

Research spec 增加：

- input shape；
- lookahead class；
- incremental capability；
- compatibility alias；
- report section；
- FFI exposure class。

通过 SSOT 生成：

- docs research API catalog；
- Python symbol registration；
- CLI help；
- capability matrix；
- parity inventory。

---

## 22. Report Model / Visualization

计算与可视化继续严格分层。

```rust
#[derive(Serialize, Deserialize)]
pub struct FactorStudyReport {
    pub provenance: StudyProvenance,
    pub data_quality: DataQualityReport,
    pub returns: ReturnsReport,
    pub information: InformationReport,
    pub turnover: TurnoverReport,
    pub stability: StabilityReport,
    pub multifactor: Option<MultiFactorReport>,
    pub validation: Option<ValidationReport>,
    pub portfolio: Option<PortfolioReport>,
    pub attribution: Option<AttributionReport>,
    pub capacity: Option<CapacityReport>,
    pub events: Option<EventReport>,
}
```

Visualization 只负责：

- summary table；
- quantile bar / violin；
- cumulative returns；
- spread time series；
- IC TS / rolling IC / hist / QQ；
- monthly IC heatmap；
- IC decay；
- turnover；
- rank autocorrelation；
- PCA loading；
- factor correlation heatmap；
- regime IC；
- capacity curve；
- risk contribution；
- event pre/post curve。

HTML/JSON/Python 共用同一 report model。

---

## 23. Python 与其它 FFI

### 23.1 Python

提供两层：

```python
# Native
study = finkit.FactorStudy(...)
report = study.full_report()

# Compatibility
import finkit.alphalens as al
```

Python adapter 只做：

- pandas MultiIndex decode/encode；
- NumPy borrowed array；
- naming/schema compatibility；
- plotting convenience。

重计算留在 Rust。

### 23.2 其它语言

Node/Java/Go/.NET/mobile/WASM 第一阶段不要复制完整 pandas-style API。

优先暴露：

- `StudySpec`；
- compact input arrays；
- `FactorStudyReport` JSON / typed compact result；
- summary metrics。

这能复用已有 FFI/error/schema 体系，控制维护成本。

---

## 24. Observability

扩展现有 `metrics.rs`，而不是引入新 metrics crate：

建议增加：

```text
research_stage_total{stage,result}
research_stage_duration_seconds{stage}
research_cache_total{level,result}
research_rows_total{stage}
research_data_loss_ratio{reason}
research_parallel_tasks_total{stage}
research_forward_horizon_pending{period}
```

不得把 asset/factor 名直接作为高基数 Prometheus label。

---

## 25. 性能策略

### 25.1 只计算一次

Full report 中共享：

- sorted panel index；
- date/group segments；
- validity mask；
- forward-return matrix；
- factor ranks；
- quantiles；
- demeaned/group-adjusted returns；
- portfolio weights；
- exposure matrix。

### 25.2 热路径规则

- String 只出现在 plan/registry 层；
- hot loop 使用 `AssetId / GroupId / column index / segment offset`；
- 不在每个 date 重新 hash group name；
- 不在每个 horizon 重算 factor rank；
- 不在每张 chart 重算 analyzer；
- 不把 output Vec 来回转换为 Python list；
- scratch 复用 `BufferArena`；
- segmented operations 支持 Rayon；
- materialization 只发生在 public ownership boundary。

### 25.3 后续优化

基准证明需要后再做：

- SIMD segmented reductions；
- radix-like AssetId sort；
- bitset validity；
- typed index arena；
- Arrow C Data zero-copy；
- memory mapped Parquet；
- incremental covariance/rank approximations。

禁止为了“理论更快”提前引入复杂基础设施。

---

## 26. No-Lookahead / Data Leakage 门禁

所有研究模块必须接受以下 invariant tests：

### Future mutation test

修改 `t+1 ... end` 的数据，不允许改变 `<=t` 的预测性特征/信号/拟合参数。

### Train/Test transform test

- quantile thresholds；
- scaler；
- regime threshold；
- PCA；
- neutralization model；
- feature selection；

在 validation 模式下必须 fit(train) → transform(test)。

### Point-in-time group test

修改未来 industry membership 不得影响历史 group-adjusted IC。

### Execution lag test

signal at `t` 不允许读取 `t+1` entry price，除非 API 明确将其定义为 forward target 而非可交易收益。

### Forward return maturity test

增量模式未到 horizon 的 forward return 必须为 pending/NaN，不允许临时使用部分未来数据。

---

## 27. Parity / Correctness 测试

### 27.1 Alphalens golden

固定：

```text
alphalens-reloaded==0.4.6
```

覆盖：

- normal factor；
- ties；
- constant；
- NaN/Inf；
- sparse assets；
- missing price；
- changing universe；
- zero-aware；
- bins/quantiles；
- by-group；
- group-neutral；
- timezone mismatch；
- custom calendar；
- intraday；
- cumulative true/false；
- event；
- overlapping positions；
- max_loss。

目标：**numerical parity failure = 0**。

### 27.2 Shared kernel contract

增加 cross-module contract tests：

```text
selection MI == importance MI
factor percentile rank == rank kernel
feature rolling correlation == rolling kernel
backtest Sharpe == risk::sharpe_ratio
labels forward arithmetic == core::returns forward arithmetic
regime log returns == core::returns log returns
```

### 27.3 Property tests

- ranks monotonic；
- quantile bucket complete/disjoint；
- group weights gross=1；
- demeaned weights net≈0；
- group neutral exposure≈0；
- OLS residual orthogonal to exposures；
- turnover in valid range；
- position constraints always satisfied；
- cache hit == uncached numerical result。

---

## 28. Duplicate-Code Gate

新增：

```text
scripts/check_research_ssot.py
```

首批禁止在非 canonical module 新定义：

```text
compute_mi
discretize
pearson_correlation
fractional_ranks
linear_regression_slope
log_returns
forward_return_arithmetic
normal_quantile
```

不是简单按函数名粗暴禁止，而是维护 allowlist，确保兼容 wrapper 可以存在，但算法实现只能有一个 canonical owner。

CI 新 gate：

```text
Research SSOT Check
```

同时增加 review rule：任何新增统计/排序/回归/return helper，PR 描述必须说明为什么不能复用现有 kernel。

---

## 29. Benchmark Gate

### 29.1 External benchmark

对比 pinned Alphalens：

- Clean Factor；
- 5 quantiles；
- 1D/5D/10D forward returns；
- IC；
- quantile returns；
- turnover；
- full report compute-only。

规模：

```text
Small   250 × 500      ≈ 125K rows
Medium  1250 × 2000    ≈ 2.5M rows
Large   2500 × 5000    ≈ 12.5M rows
```

External speedup 第一阶段只做报告，不作为硬失败阈值。

### 29.2 Internal hard gate

对 finkit 自身 baseline：

- wall time；
- peak RSS；
- allocations；
- rows/sec；
- cache-hit speed；
- incremental update latency。

建议同机同 compiler 的 regression threshold：15%–20%，具体值由多轮 baseline 稳定性确定。

---

## 30. 迁移原则

### 30.1 Facade migration

旧 API：

```rust
features::mutual_information(...)
factors::zscore(...)
risk::sharpe_ratio(...)
```

继续存在。

迁移只改变内部实现：

```text
old facade → canonical kernel
```

不为了“去重”立即制造 breaking API。

### 30.2 一次只收敛一个语义族

顺序：

1. return；
2. rank/quantile；
3. correlation；
4. MI；
5. regression；
6. risk metrics；
7. segmented execution；
8. research analyzers。

每个族完成后再进入下一族，避免一次性重写 core 后无法判断 parity regression 来源。

---

## 31. 推荐 PR 执行序列

### Phase A — 去重与共享内核

**PR A1 — Return SSOT**

- 新 `core::returns`；
- factors/labels/backtest/regime 委托；
- compatibility tests。

**PR A2 — Rank / Quantile SSOT**

- tie policy；
- quantile interpolation；
- binning；
- factors/features 委托。

**PR A3 — Statistics / MI / Correlation SSOT**

- selection 与 importance 去重；
- combinations rolling corr 委托。

**PR A4 — Regression SSOT**

- simple/multi OLS；
- WLS；
- residualize；
- rolling_stats 与 neutralize 委托。

**PR A5 — Risk SSOT**

- backtest risk stats 委托 `risk.rs`；
- annualization config 统一。

**PR A6 — Segmented Executor + cache primitive**

- SegmentLayout；
- serial/rayon executor；
- revision cache primitive；
- SSOT duplicate gate。

### Phase B — Research Data Core

**PR B1 — factor-analysis crate + ResearchFrame**

- PanelIndex；
- FeatureMatrix composition；
- borrowed views；
- categorical dictionary。

**PR B2 — Point-in-time universe + quality report**

**PR B3 — ResearchPlan backed by ComputePlan**

**PR B4 — provenance + typed FeatureStore adapter**

### Phase C — Alphalens parity

**PR C1 — ForwardReturnEngine**

**PR C2 — Clean / Align / Quantize / Group**

**PR C3 — Returns analysis**

**PR C4 — IC analysis**

**PR C5 — Turnover / overlapping holdings**

**PR C6 — Event study**

**PR C7 — Alphalens Python compatibility API**

要求同一 final SHA 上 parity failure=0。

### Phase D — Advanced factor research

**PR D1 — Factor diagnostics / monotonicity / decay**

**PR D2 — Multi-factor matrix / redundancy / PCA**

**PR D3 — Multi-exposure neutralization**

**PR D4 — Fama-MacBeth + HAC inference**

**PR D5 — Stability / regime / live health**

### Phase E — Validation

**PR E1 — CV adapter + interval-aware purge**

**PR E2 — Bootstrap / permutation**

**PR E3 — FDR / Deflated Sharpe / PBO**

### Phase F — Portfolio / Risk / Capacity

**PR F1 — TargetWeights / constraints**

**PR F2 — HoldingPeriodPortfolioEngine**

**PR F3 — CostModel / turnover cost**

**PR F4 — Capacity / liquidity**

**PR F5 — Factor risk model / attribution**

**PR F6 — Scenario / concentration / crowding**

### Phase G — Productization

**PR G1 — unified FactorStudyReport**

**PR G2 — visualization / HTML**

**PR G3 — CLI / JSON**

**PR G4 — compact Node/Java/Go/.NET/WASM adapters**

**PR G5 — ResearchRegistry / generated docs / capability matrix**

**PR G6 — performance/incremental final convergence**

---

## 32. Definition of Done

最终发布门禁：

### Architecture

- [ ] Research 不存在独立第四套 DAG planner；
- [ ] Research 不存在独立 buffer pool；
- [ ] Research 不存在独立 calendar implementation；
- [ ] FeatureMatrix 作为 numeric matrix 基础被复用；
- [ ] Formula/Composite/FactorEngine 作为 candidate compute source 被复用。

### Duplicate elimination

- [ ] MI canonical implementation = 1；
- [ ] discretization canonical implementation = 1；
- [ ] fractional rank canonical implementation = 1；
- [ ] Pearson/Spearman correlation canonical family = 1；
- [ ] forward/simple/log return canonical family = 1；
- [ ] regression canonical family = 1；
- [ ] backtest 不再复制 risk metrics；
- [ ] `Research SSOT Check` green。

### Alphalens

- [ ] Forward Returns PASS；
- [ ] Clean Factor PASS；
- [ ] Quantile/Bin PASS；
- [ ] Zero-aware PASS；
- [ ] Group binning PASS；
- [ ] Max-loss PASS；
- [ ] Factor weights PASS；
- [ ] Factor returns PASS；
- [ ] Quantile returns PASS；
- [ ] Spread PASS；
- [ ] Alpha/Beta PASS；
- [ ] IC PASS；
- [ ] Turnover PASS；
- [ ] Event PASS；
- [ ] Overlapping positions PASS；
- [ ] parity failure = 0。

### Advanced Research

- [ ] Multi-factor correlation / redundancy / PCA；
- [ ] multi-exposure neutralization；
- [ ] Fama-MacBeth；
- [ ] HAC/Newey-West；
- [ ] rolling IC / decay / regime；
- [ ] walk-forward / purged / embargo / CPCV；
- [ ] bootstrap / multiple testing；
- [ ] portfolio constraints；
- [ ] transaction cost / capacity；
- [ ] risk attribution；
- [ ] scenario/crowding；
- [ ] provenance/cache/incremental。

### Safety / Correctness

- [ ] future mutation gate；
- [ ] fit/transform leakage gate；
- [ ] point-in-time group gate；
- [ ] calendar/intraday gate；
- [ ] pending horizon maturity gate；
- [ ] NaN/Inf/tie semantics gate；
- [ ] changing universe gate。

### Engineering

- [ ] `cargo fmt --all -- --check`；
- [ ] `cargo clippy --workspace`；
- [ ] workspace tests；
- [ ] doc tests；
- [ ] fuzz targets；
- [ ] generated docs/registry checks；
- [ ] Python parity；
- [ ] cross-language compact-report parity；
- [ ] internal performance regression gate green。

---

## 33. 最优先实施顺序

如果从当前 main 开始真正编码，优先级必须是：

```text
1. Return SSOT
2. Rank / Quantile SSOT
3. MI / Correlation SSOT
4. Regression SSOT
5. Backtest → Risk SSOT
6. Segmented Executor
7. ResearchFrame / PanelIndex
8. ForwardReturnEngine
9. Clean / Quantize / Group
10. Returns / IC / Turnover / Event parity
11. Multi-Factor / Neutralization
12. Validation / HAC / Multiple Testing
13. Portfolio / Cost / Capacity
14. Risk Model / Attribution / Scenario
15. Report / Visualization / FFI
16. Incremental / Cache / Mining convergence
```

原因很明确：如果在 shared kernel 尚未收敛前直接继续增加 Alphalens、多因子、风险、组合模块，会把当前已经存在的 MI、return、correlation、regression、risk 等重复实现进一步放大。先统一 primitive，再扩展上层模块，才能真正减少长期维护成本。

---

## 34. 最终目标形态

完成后，Finkit 的核心定位应形成四层：

```text
Layer 1 — Compute Kernel
Indicators / Math / SIMD / Streaming / Returns / Rank / Regression

Layer 2 — Expression & Feature Runtime
Formula / Composite / FactorPlan / FeatureSet / Labels / Regime

Layer 3 — Factor Research
Panel / Alphalens / IC / Multi-Factor / Validation / Portfolio / Risk / Event

Layer 4 — Product Surface
Report / Visualization / Python / CLI / JSON / FFI / Web
```

其中每个基础算法只有一个 canonical owner；其它所有模块通过 facade、adapter、view 或 plan 复用。这是后续继续扩展 Alpha Research 能力时必须长期坚持的架构约束。
