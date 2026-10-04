# API Reference

Complete API reference for all language bindings in Finkit.

## Unified operation and formula contracts

All public language surfaces that expose the control plane share the same JSON contracts
(seven named below).
The typed indicator functions remain the preferred hot path; these entry points
are for dynamic operation selection, named outputs, capability discovery and
cross-language parity tests.

```json
{
  "operation": "SMA",
  "semantic_profile": "talib_0_8_0",
  "input_order": ["CLOSE"],
  "inputs": {"CLOSE": [1, 2, 3]},
  "params": [2]
}
```

The result is a versioned envelope with `operation`, `operation_id`,
`semantic_profile`, `primary`, `shape` and named `values`. Non-finite numeric
values are JSON `null`. `core_registry` uses the canonical Formula/Core
semantics; `talib_0_8_0` is explicit and only succeeds for operations listed by
the catalog's `semantic_profiles` field. Unsupported profile/function pairs
return a structured error instead of silently falling back.

When the same operation name has different native and compatibility schemas,
select the entry in `profile_output_contracts` for the requested profile. The
top-level `output_names` and `params` describe the canonical Core registry;
the profile map describes the exact profile parameters and result fields (for
example, the TA-Lib profile of `KDJ` returns `K`, `D`, and `J`).
The dispatcher rejects parameter lists longer than the selected profile schema
and returns a structured error if execution produces unaligned fields or
series, so catalog metadata is enforced at runtime rather than being
descriptive only.

The shared names are:

- Rust: `finkit_ffi_common::execute_operation_json`;
- Python: `operation_execute_json`;
- Go: `ta.OperationExecuteJSON`;
- Java: `Indicators.operationExecuteJson`;
- .NET: `Indicators.OperationExecuteJson`;
- C: `ta_operation_execute_json`;
- Node: `operationExecuteJson`.

iOS and Android do not expose the unified control-plane entry point. They ship
per-indicator FFI surfaces instead: iOS builds a static library of `alpha_ta_*`
symbols, and Android exposes `Java_com_finkit_indicators_Finkit_*Native` JNI
functions. Use those directly for on-device integration; the JSON control plane
above is the recommended path for every other language.

Formula results use the corresponding versioned contract functions:
`formula_eval_contract_json` and `formulaEvalContractJson`, plus their
language-specific casing equivalents.

Timestamped and multi-timeframe Formula evaluation uses the shared
`formula.temporal.v1` request contract. Every official binding exposes the
same `formula_eval_temporal_contract_json` capability with language-specific
casing. The request includes a frame with `symbol`, `timeframe`, monotonic
`timestamps`, aligned OHLCV arrays, and optional named `inputs` and
`fundamentals`:

```json
{
  "schema_version": 1,
  "source": "HIGHER + EARNINGS + CLOSE",
  "dialect": "tdx",
  "frame": {
    "symbol": "AAA", "timeframe": "1m", "timestamps": [10, 20, 30],
    "open": [1, 2, 3], "high": [1, 2, 3], "low": [1, 2, 3],
    "close": [1, 2, 3], "volume": [10, 20, 30]
  },
  "inputs": [{
    "name": "HIGHER", "timestamps": [10, 30], "values": [100, 300],
    "alignment": "as_of_closed"
  }]
}
```

`alignment` is explicitly `exact` or `as_of_closed`; unsupported policies,
non-monotonic timestamps, duplicate names, and built-in-field shadowing are
rejected. Fundamental timestamps are publication/availability times and use
as-of lookup, so future revisions cannot enter an earlier row. The result
includes frame identity, timestamps, named values, an ordered `outputs` array
with output names and optional modifier metadata, the versioned `draw` payload,
and JSON `null` for non-finite values. This is explicit alignment, not an
implicit resampler or a complete Pine `request.security` implementation.

The ordinary Formula response uses the same `outputs` array. A plain output is
represented with a null modifier; styled Pine outputs carry the canonical
modifier object so every language binding and the Lightweight Charts adapter
can select the same series identity and rendering family.

Multi-symbol and multi-timeframe Formula evaluation uses the
`formula.panel.v1` contract and the corresponding
`formula_eval_panel_contract_json` language-specific entry points. Its
`frames` array contains the same explicit timestamped OHLCV frame shape. Each
frame is evaluated independently and the response is sorted by
`symbol@timeframe`; duplicate frame identities are rejected. No recursive
indicator state, cache, or drawing command is shared between frames.

Cross-sectional Factor execution uses `factor.cross_sectional.v1` and the
corresponding `factor_cross_sectional_execute_json` entry points. The request
contains one Factor target, monotonic `timestamps`, an ordered unique
`symbols` axis, and named row-major input panels. Each row is evaluated
independently across symbols; all input panels must have the same dimensions.
The response preserves both axes and returns row-major `values`, with JSON
`null` for missing or non-finite values. The portable built-ins currently
include `cross_zscore`, `cross_rank`, `cross_winsorize_05_95`, and
`cross_neutralize`. This is a cross-sectional Factor contract, not an
automatic multi-timeframe resampler or a Pine `request.security` replacement.

Formula cross-sectional execution uses `formula.cross_sectional.v1` and the
corresponding `formula_eval_cross_sectional_contract_json` entry points. It
accepts row-major named inputs and evaluates explicit `CS_RANK`, `CS_ZSCORE`,
`CS_SCALE`, `CS_INDNEUTRALIZE`, and `CS_SIGNED_POWER` functions for each
timestamp row. The existing time-window `RANK(X, N)` remains unchanged. The
response contains row-major values and a `draw.mode = "per_row"` payload so
drawing commands are never silently discarded; consumers must map each draw
row to its timestamp and symbol columns.

Formula compatibility discovery uses the same versioned report in every
official binding: `formula_compatibility_report_json` (Python),
`FormulaCompatibilityReportJSON`, `formulaCompatibilityReportJson`, and their
language-specific casing equivalents. The report includes a
`schema_version` and source-level capability matrix; it does not claim full
terminal compatibility merely because a parser or function name is present.

## Core API (Rust)

### Module Structure

```rust
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::{candlestick, chart};
```

### Cross-market screening

`finkit::indicators` also exposes market-neutral selection primitives. They
operate on aligned oldest-to-newest arrays and return an aligned signal or
excess-return array. The formula runtime exposes the same operations through
`GOLDEN_CROSS`, `DEAD_CROSS`, `BREAKOUT`, `BREAKDOWN`, `VOLUME_SURGE`,
`MA_ALIGN`, `RELATIVE_STRENGTH`, `GAP_SIGNAL`, and `TREND_BREAKOUT`.

```rust
use finkit::indicators::{
    breakout_up, dead_cross, golden_cross, ma_alignment, relative_strength,
    trend_breakout_signal, volume_surge,
};

let cross = golden_cross(&ema_fast, &ema_slow)?;
let breakout = breakout_up(&close, &high, 20)?;
let volume_ok = volume_surge(&volume, 20, 1.5)?;
let relative = relative_strength(&close, &benchmark_close, 20)?;
```

See [screening-formulas.md](screening-formulas.md) for all signatures,
aliases, warm-up behavior, and A-share/HK/US/crypto selection recipes.

### Overlap Studies

```rust
/// Simple Moving Average
pub fn sma(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// Exponential Moving Average
pub fn ema(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// Bollinger Bands
pub fn bbands(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_dn: f64
) -> Result<BbandsResult>;

pub struct BbandsResult {
    pub upper: Array1<f64>,
    pub middle: Array1<f64>,
    pub lower: Array1<f64>,
}

/// Parabolic SAR
///
/// Note the return shape: unlike most overlap indicators this one returns two
/// series, so it yields `SarResult` rather than a bare array.
pub fn sar(
    high: &[f64],
    low: &[f64],
    acceleration: f64,
    maximum: f64
) -> Result<SarResult>;

pub struct SarResult {
    pub sar: Array1<f64>,
    pub af: Array1<f64>,
}
```

> **Return type convention.** Floating-point outputs are `ndarray::Array1<f64>`,
> not `Vec<f64>`. Call `.to_vec()` if you need an owned `Vec`.

### Momentum Indicators

```rust
/// Relative Strength Index
pub fn rsi(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// MACD
pub fn macd(
    data: &[f64],
    fastperiod: usize,
    slowperiod: usize,
    signalperiod: usize
) -> Result<MacdResult>;

pub struct MacdResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

/// Stochastic
pub fn stoch(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    slowk_period: usize,
    slowd_period: usize
) -> Result<StochResult>;

pub struct StochResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}
```

### Volume Indicators

```rust
/// On Balance Volume
pub fn obv(close: &[f64], volume: &[f64]) -> Result<Vec<f64>>;

/// Chaikin A/D Line
pub fn ad(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64]
) -> Result<Vec<f64>>;
```

### Volatility Indicators

```rust
/// Average True Range
pub fn atr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize
) -> Result<Vec<f64>>;
```

### Candlestick Patterns

```rust
/// Doji Pattern
pub fn doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64
) -> Result<Vec<i32>>;

/// Hammer Pattern
pub fn hammer(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64]
) -> Result<Vec<i32>>;

/// Engulfing Pattern
pub fn engulfing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64]
) -> Result<Vec<i32>>;
```

### Chart Patterns

```rust
/// Double Top Detection
pub fn double_top(
    high: &[f64],
    lookback: usize,
    tolerance: f64
) -> Result<Vec<usize>>;

/// Head and Shoulders Detection
// Chart patterns are top/bottom pairs -- there is no single `head_shoulders`.
// Both return ChartPatternResult (an Array1<i32> marker series).
pub fn head_and_shoulders_top(
    high: &[f64],
    min_bars_between_peaks: usize,
    head_height_ratio: f64
) -> Result<ChartPatternResult>;

pub fn head_and_shoulders_bottom(
    low: &[f64],
    min_bars_between_peaks: usize,
    head_depth_ratio: f64
) -> Result<ChartPatternResult>;
```

> **Pattern modules.** Candlestick patterns live in `finkit::patterns::candlestick`
> and return `PatternResult`; chart patterns live in `finkit::patterns::chart` and
> return `ChartPatternResult`. They are *not* re-exported from
> `finkit::indicators`.

### Streaming API (Incremental)

The `streaming` module provides O(1) per-bar indicator updates via the
[`StreamingIndicator`](https://docs.rs/finkit/latest/finkit/streaming/trait.StreamingIndicator.html)
trait. Use this path for live data feeds where re-scanning full history is impractical.

#### Core Traits

```rust
use finkit::streaming::{Ohlcv, OhlcvBar, StreamingIndicator, IndicatorMeta};

/// OHLCV bar access
pub trait Ohlcv {
    fn open(&self) -> f64;
    fn high(&self) -> f64;
    fn low(&self) -> f64;
    fn close(&self) -> f64;
    fn volume(&self) -> f64;
}

/// O(1) incremental update.
///
/// `next` returns `None` during warm-up and `Some(output)` once the indicator
/// has converged — it never returns a NaN placeholder.
pub trait StreamingIndicator<Input = f64, Output = f64> {
    fn next(&mut self, input: Input) -> Option<Output>;
    fn next_with_time(&mut self, input: Input, open_time: i64) -> Option<Output>;
    fn reset(&mut self);
    fn is_ready(&self) -> bool;
    fn count(&self) -> usize;
    fn value(&self) -> Option<Output>;
}

/// Machine-readable metadata
pub trait IndicatorMeta {
    fn name() -> &'static str;
    fn category() -> &'static str;
    fn description() -> &'static str;
    fn warm_up_period(&self) -> usize;
}
```

> There is a second, unrelated trait also called `StreamingIndicator`, in
> `finkit::traits`. That one is bar-oriented
> (`update(&mut self, bar: &dyn Ohlcv) -> Option<Output>`, with `Config` /
> `Output` associated types and a `convergence()` method) and is intended for
> adapters. Importing both in one module requires an alias. The trait shown
> above is the one the concrete `Streaming*` structs implement.

#### Available Streaming Indicators

| Struct | Input | Output | Module |
|--------|-------|--------|--------|
| `StreamingSma` | `f64` | `f64` | `streaming::indicators` |
| `StreamingEma` | `f64` | `f64` | `streaming::indicators` |
| `StreamingRsi` | `f64` | `f64` | `streaming::indicators` |
| `StreamingAtr` | `(f64, f64, f64)` — high, low, close | `f64` | `streaming::indicators` |
| `StreamingBoll` | `f64` | `BollOutput` | `streaming::indicators` |
| `StreamingMacd` | `f64` | `MacdOutput` | `streaming::indicators` |

```rust
use finkit::streaming::StreamingIndicator;
use finkit::streaming::indicators::{StreamingSma, StreamingMacd, MacdOutput, StreamingAtr};

// Simple moving average — feed close prices one at a time.
// Warm-up yields `None`, not NaN.
let mut sma = StreamingSma::new(3);
assert_eq!(sma.next(1.0), None);   // warming up
assert_eq!(sma.next(2.0), None);
assert_eq!(sma.next(3.0), Some(2.0)); // ready
assert!(sma.is_ready());
assert_eq!(sma.value(), Some(2.0)); // last value, without advancing

// MACD — returns structured output per bar
let mut macd = StreamingMacd::new(12, 26, 9);
if let Some(out) = macd.next(100.0) {
    // out.macd, out.signal, out.histogram
}

// ATR takes a (high, low, close) tuple, not a bar object.
let mut atr = StreamingAtr::new(14);
let val = atr.next((12.0, 10.0, 11.0)); // Option<f64>
```

To feed whole bars instead of raw prices, use `compute_bar(&OhlcvBar)` (available
on the streaming indicators that carry repaint support) or the bar-oriented
`finkit::traits::StreamingIndicator` adapter shown above.

#### Output Structs

```rust
pub struct BollOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

pub struct MacdOutput {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}
```

### Indicator Registry API

The registry module exposes static metadata for all supported indicators,
enabling discovery, documentation generation, and JSON export.

```rust
use finkit::streaming::{
    all_indicators, by_category, by_id, registry_document, VALID_CATEGORIES,
    IndicatorInfo, ParamInfo, RegistryDocument,
};

/// Returns metadata for every registered indicator
pub fn all_indicators() -> &'static [IndicatorInfo];

/// Looks up one indicator by its canonical name, e.g. "SMA"
pub fn by_id(name: &str) -> Option<&'static IndicatorInfo>;

/// Returns every indicator in one category, in registry order
pub fn by_category(category: &str) -> Vec<&'static IndicatorInfo>;

/// Builds the full registry document for JSON serialization
pub fn registry_document() -> RegistryDocument;
```

#### Data Types

```rust
pub struct IndicatorInfo {
    pub name: &'static str,
    pub category: &'static str,       // a member of VALID_CATEGORIES
    pub description: &'static str,
    pub params: &'static [ParamInfo],
    pub convergence: usize,           // warm-up bars at the default parameters
    pub streaming: bool,              // part of the advertised streaming surface
}

pub struct ParamInfo {
    pub name: &'static str,
    pub param_type: &'static str,     // "usize", "f64", or "str"
    pub default: &'static str,
    pub description: &'static str,
}

pub struct RegistryDocument {
    pub version: &'static str,
    pub generated_at: Option<&'static str>,
    pub indicators: &'static [IndicatorInfo],
}
```

`convergence` describes the **default** configuration declared in `params`;
an indicator instantiated with a longer period warms up for longer. For the
exact figure of a concrete instance, read `IndicatorMeta::warm_up_period()` on
that instance.

`streaming` is a curated claim about the published incremental surface, not a
census of every `Streaming*` type in the crate. The `breadth`, `pattern` and
`fibonacci` categories are deliberately `false`: they need market-wide or
pattern-wide input rather than a single series of bars. That rule is pinned by
`streaming::registry`'s `test_registry_coverage`.

#### Category Slugs

`VALID_CATEGORIES` is the single source of truth for the vocabulary — 15 slugs:

| Category | Contents |
| --- | --- |
| `overlap` | Moving averages and price overlays (`SMA`, `EMA`, `BBANDS`, `SAR`, `SuperTrend`, ...) |
| `momentum` | Oscillators and momentum (`RSI`, `MACD`, `KDJ`, `CCI`, ...) |
| `volume` | Volume-driven indicators (`OBV`, `MFI`, `VWAP`, `CMF`, ...) |
| `volatility` | Range and deviation measures (`ATR`, `NATR`, `Donchian`, `Keltner`, `STDDEV`, `VAR`, ...) |
| `price_transform` | Price derivations (`AVGPRICE`, `MEDPRICE`, `TYPPRICE`, `WCLPRICE`) |
| `cycle` | Hilbert-transform and Ehlers cycle tools |
| `statistics` | Rolling statistics (`BETA`, `CORREL`, `TSF`, `AVGDEV`, `ZSCORE`, ...) |
| `math_transform` | Element-wise transforms (`SIN`, `LOG10`, `SQRT`, ...) |
| `math_operators` | Element-wise operators (`ADD`, `SUB`, `MULT`, `DIV`, `MIN`, `MAX`, `SUM`) |
| `pattern` | Candlestick pattern detectors (`CDL_*`) |
| `smc` | Smart-money-concepts detectors (`FVG`, `OB`) |
| `breadth` | Market-breadth aggregates (`TRIN`, `AR`/`BR`/`CR`, advance/decline) |
| `sentiment` | Sentiment gauges (fear & greed, put/call ratio) |
| `fibonacci` | Fibonacci retracement levels |
| `astock` | A-share market-structure indicators |

Every category returned by an indicator — whether from the registry or from
`IndicatorMeta::category()` on a streaming type — is checked against this list
by `scripts/check_streaming_registry_contract.py`.


#### JSON Export

The canonical JSON snapshot lives at `docs/indicator_registry.json` and is
validated against `registry_document()` in core tests:

```rust
use finkit::streaming::registry_document;
use serde_json;

let json = serde_json::to_string_pretty(&registry_document()).unwrap();
// Write to docs/indicator_registry.json or serve via HTTP
```

To filter by category, use `VALID_CATEGORIES` rather than hard-coding slugs —
it is the declared vocabulary (see [Category Slugs](#category-slugs) above).

## Python API

### Installation

Finkit is distributed as Python ABI3 wheels attached to the published GitHub
`v0.1.15` Release. Download the wheel for your platform and install it locally,
or build the current v0.2.0 workspace from source with `maturin` (see
[installation.md](installation.md)):

```bash
python -m pip install ./finkit-0.1.15-<matching-platform>.whl
```

### Functions

```python
import finkit as ta
import numpy as np

# Overlap Studies
def sma(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def ema(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def bollinger_bands(
    close: np.ndarray,
    timeperiod: int = 20,
    nbdevup: float = 2.0,
    nbdevdn: float = 2.0
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]: ...

# Momentum
def rsi(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def macd(
    close: np.ndarray,
    fastperiod: int = 12,
    slowperiod: int = 26,
    signalperiod: int = 9
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]: ...
def stoch(
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    fastk_period: int = 5,
    slowk_period: int = 3,
    slowd_period: int = 3
) -> Tuple[np.ndarray, np.ndarray]: ...

# Volatility
def atr(
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    timeperiod: int = 14
) -> np.ndarray: ...

# Volume
def obv(close: np.ndarray, volume: np.ndarray) -> np.ndarray: ...

# Pattern Recognition
def cdl_doji(
    open: np.ndarray,
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    doji_pct: float = 0.1
) -> np.ndarray: ...

def detect_double_top(
    high: np.ndarray,
    lookback: int = 20,
    tolerance: float = 0.03
) -> np.ndarray: ...
```

### Complete Example

```python
import finkit as ta
import numpy as np
import pandas as pd

# Generate sample data
np.random.seed(42)
close = np.cumsum(np.random.randn(100)) + 100
high = close + np.random.uniform(0.5, 2.0, 100)
low = close - np.random.uniform(0.5, 2.0, 100)
volume = np.random.uniform(1000, 5000, 100)

# Calculate indicators
sma_20 = ta.sma(close, timeperiod=20)
rsi_14 = ta.rsi(close, timeperiod=14)
macd_line, signal, histogram = ta.macd(close)
upper, middle, lower = ta.bollinger_bands(close)

# Pattern recognition
doji_signals = ta.cdl_doji(open, high, low, close)
double_tops = ta.detect_double_top(high)
```

### Streaming Indicators

Streaming indicators update in O(1) per bar instead of recomputing the whole
series. The Python binding exports **88** `Streaming*` classes; all of them share
the same method set, and **26** additionally support state persistence. The
complete list is in the generated
[streaming indicator catalog](generated/streaming-indicators.md).

Common interface (constructor parameters differ per indicator):

```python
streaming = ta.StreamingSMA(period=20)

streaming.update(value)          # single bar; NaN until the warmup completes
streaming.update_batch(values)   # batch update, returns an equal-length list
streaming.is_ready()             # enough samples accumulated?
streaming.count()                # samples received so far
streaming.reset()                # clear internal state
```

Multi-input indicators (for example `StreamingATR`) take the extra series as
positional arguments to `update`.

State persistence: `save_state()` is an **instance method** returning serialisable
`bytes`, while `restore_state()` is a **static method** that takes those bytes and
returns a **new instance** — it does not mutate the caller:

```python
state = streaming.save_state()
resumed = ta.StreamingSMA.restore_state(state)   # call on the class, not the instance
```

> Writing `streaming.restore_state(state)` does not raise, but the returned object
> is discarded and the caller is left unchanged. Always call it on the class.

Indicators with internal signal state (such as `StreamingMACD`) do not provide
persistence; check the
[streaming indicator catalog](generated/streaming-indicators.md) for the current
set.

```python
# Relative Strength Index
streaming_rsi = ta.StreamingRSI(period=14)
for price in prices:
    rsi_value = streaming_rsi.update(price)

# MACD — update() returns a MACDResult, not a tuple
streaming_macd = ta.StreamingMACD(fast_period=12, slow_period=26, signal_period=9)
result = streaming_macd.update(price)
print(result.macd, result.signal, result.histogram)

# Average True Range — three positional inputs
streaming_atr = ta.StreamingATR(period=14)
for h, l, c in zip(high, low, close):
    atr_value = streaming_atr.update(h, l, c)
```

`MACDResult` exposes `.macd` / `.signal` / `.histogram`; it is **not iterable**, so
`macd, signal, hist = ...` raises `TypeError`. `update_batch()` returns a list of
`MACDResult`. Use
`ta.StreamingMACDEXT(fast_period=12, slow_period=26, signal_period=9, fast_ma="ema", slow_ma="ema", signal_ma="ema")`
when you need configurable moving-average types.

### Chanlun structures

The Chanlun (缠论) entry points return **plain Python dictionaries**, not objects,
so results join back to a pandas index without imposing a timestamp type on the
core. Bar positions are zero-based indices.

> **Inputs must be NumPy arrays.** These functions take
> `PyReadonlyArray1<f64>`; passing a `list` raises
> `TypeError: 'list' object is not an instance of 'ndarray'`. This differs from
> the plain indicators above, which accept lists.

```python
import numpy as np
import finkit as ta

o = np.asarray(open_, dtype=np.float64)   # likewise high / low / close / volume
res = ta.chan_analyze(
    o, high, low, close, volume,
    min_stroke_bars=6,
    variant="standard",              # conservative / standard / aggressive
    fractal_policy="strict",         # strict / loose / right_confirmed
    stroke_policy="configurable",    # configurable / fixed5 / fixed6 / fixed7 / threshold
    center_policy="dynamic",         # three_stroke / dynamic / hierarchical
    min_stroke_change_ratio=0.0,
    min_fractal_range_ratio=0.0,
    signal_min_strength=0.0,
    center_break_ratio=0.0,
)
```

`chan_analyze` returns **one dict** (not a list of dicts) with these keys:

| Key | Type | Meaning |
| --- | --- | --- |
| `bar_count`, `fractal_count`, `stroke_count`, `segment_count`, `center_count` | `int` | stage sizes |
| `trend` | `str` | `unknown` / `bullish` / `bearish` / `range` |
| `developing_fractal` | `dict` or `None` | the unconfirmed fractal, if any |
| `fractals` | `list[dict]` | `index`, `kind` (`top`/`bottom`), `high`, `low`, `value` |
| `strokes` | `list[dict]` | `start_index`, `end_index`, `direction` (`up`/`down`), `high`, `low`, `bars`, `change`, `slope`, `strength` |
| `segments` | `list[dict]` | `start_index`, `end_index`, `start_stroke`, `end_stroke`, `change` |
| `centers` | `list[dict]` | `start_index`, `end_index`, `upper`, `lower`, `middle`, `level`, `start_stroke`, `end_stroke` |
| `signals` | `list[dict]` | `kind` (`buy1`..`sell3`), `index`, `price`, `confirmed`, `strength`, `reason`, `evidence` |
| `divergences` | `list[dict]` | `kind` (`bullish`/`bearish`), `start_stroke`, `end_stroke`, `index`, `price`, `confirmed`, `strength`, `reason` |

Multi-timeframe variants all return `{"frames": [...], "resonance_direction",
"resonance_score", "aligned_frames", "conflicting_frames"}`, where each frame
carries `factor`, `label`, `seconds`, `source_ranges` and a **summary** `analysis`
dict (counts, `trend` and `signals` only — not the full structure lists):

```python
mtf = ta.chan_analyze_multi(o, high, low, close, volume,
                            factors=[1, 5, 20], auto_levels=3, min_frame_bars=20)
for frame in mtf["frames"]:
    print(frame["label"], frame["analysis"]["trend"])
print(mtf["resonance_direction"], mtf["resonance_score"])
```

For real timelines use `chan_analyze_multi_timestamps(timestamps, o, high, low,
close, volume, durations_seconds=None, min_stroke_bars=6, variant="standard",
origin_seconds=0)`, or `chan_analyze_multi_timestamps_calendar(...)` which adds
`market`, `timezone`, `holidays`, `sessions` and `special_sessions` overrides.

### Composite indicators

`compute_composite` evaluates a named dependency graph. Each definition is a
`(name, function, inputs, params)` tuple; an input may be a raw series
(`close`/`high`/`low`/`volume`), another definition's name, or `const:<number>`.
Shared intermediates are computed once per call.

```python
graph = ta.compute_composite(
    close=close,
    definitions=[
        ("fast", "ema", ["close"], [5]),
        ("slow", "ema", ["close"], [20]),
        ("spread", "sub", ["fast", "slow"], []),
        ("signal", "sma", ["spread"], [3]),
        ("cross", "cross_up", ["spread", "const:0"], []),
    ],
    outputs=["spread", "signal", "cross"],
)
```

Built-in functions include `sma`, `ema`, `wma`, `rsi`, `atr`, `macd`,
`boll_mid`, `boll_upper`, `boll_lower`, `vwma`, `return`, `zscore`, `cross_up`
and `cross_down`, plus the threshold operators `threshold` / `between` / `clip`.

### Factor library

```python
alpha158 = ta.factor_library("alpha158")        # 158 factors
wq101    = ta.factor_library("worldquant101")   # 17 factors
```

`factor_library(name)` takes **one required argument**. The available names are
`alpha158` (158 factors) and `worldquant101` (17 factors); any other value
raises `unknown factor library ...; available: ["alpha158", "worldquant101"]`.

It returns a `FactorLibrary`, which supports `len()` and `in` but is **not
iterable and not subscriptable** — enumerate it with `names()`:

```python
lib = ta.factor_library("worldquant101")
len(lib)                      # 17
lib.names()                   # ['Alpha101', 'Alpha12', 'Alpha21', ...]
lib.describe("Alpha101")      # metadata, without evaluating
lib.expression("Alpha101")    # the verbatim compiled expression
lib.direction("Alpha101")     # preferred ranking direction
lib.dependencies("Alpha101")  # external series, in the spelling evaluate() accepts
lib.evaluate("Alpha101", ...) # one factor
lib.evaluate_all(...)         # {name: ndarray}
```

### Formula templates and registry

The built-in template catalog is searchable and versioned as plain dicts:

```python
ta.formula_list_categories()          # list[dict] -> {"category": str, "count": int}
ta.formula_search_templates("ma")     # list[dict] of matching templates
ta.formula_get_template("ma_cross")   # one template dict
```

Each template is a dict with `name`, `category`, `description`, `formula` and
`parameters` (a mapping of parameter name to `{default, min, max}`).

`FormulaRegistry` is the extension point for user-registered formulas, exposing
`compile`, `names`, `register` and `unregister`.

### Compiled formulas

`CompiledFormula(source)` is the reusable plan for repeated evaluation. Construct
it once and call `eval` many times. The stream context retained after `eval` is
also the backing store for `append_bar` / `eval_last`, so repeated streaming
updates do **not** concatenate the whole history.

```python
plan = ta.CompiledFormula("MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); CROSS(MA5,MA10)")
out = plan.eval(open_, high, low, close, volume)     # -> dict of NumPy arrays
last = plan.eval_last()
plan.append_bar(...)                                # amortized O(1)
plan.reset()
```

Available methods: `eval`, `eval_zero_copy`, `eval_owned`, `eval_range`,
`eval_range_zero_copy`, `eval_last`, `append_bar`, `reserve_bars`, `reset`,
`metadata`, `analyze`, `compatibility_report`, `talib_catalog`, and the `source`
property.

> `finkit.compile(...)` / `finkit.stream(...)` do **not** exist as module-level
> functions. Use `CompiledFormula` and the `Streaming*` classes.

### Market calendar

```python
session = ta.resolve_market_session(
    "us_equity", timestamp,
    timezone="America/New_York",
    holidays=["2026-07-03"],
    sessions=[(9 * 3600 + 30 * 60, 16 * 3600)],
)
```

Supported `market` presets: `a_share`, `china_futures`, `hong_kong`,
`us_equity`, `crypto`. Returns `None` when the timestamp is not in a session,
otherwise a dict with `session_day`, `session_index`, `open_timestamp`,
`close_timestamp`, `market`, `timezone`, `source` and `revision`.

Market presets only carry stable regular sessions and time zones: exchange
holiday calendars, ad-hoc closures and per-product sessions should be passed in
through `holidays`, `sessions` and `special_sessions` rather than baked into the
numeric core. Use `resolve_market_session_config(config_json, timestamp)` to
resolve from a versioned JSON exchange-calendar definition and preserve the
file's own `source` / `revision`.

### Charting

`KlineChart(data, language="zh", title="", width=1200, height=600)` renders K-line
charts with indicator overlays:

```python
chart = ta.KlineChart(data, language="zh", title="缠论结构")
chart.add_chan(min_stroke_bars=6, show_labels=True)
chart.set_viewport(end=2000, pixel_width=1600, overscan_bars=40, follow_latest=True)
chart.set_lod_policy("auto")
chart.set_layer_visible("volume", True)
svg = chart.to_svg_string()
```

Overlays: `add_ma`, `add_ema`, `add_boll`, `add_macd`, `add_rsi`, `add_kdj`,
`add_sar`, `add_chan`, `add_chan_multi`, `add_custom_indicator`,
`add_event_marker`. Output: `to_svg_string`, `to_canvas_html`,
`to_webgl_html`, `to_webgpu_html`, `save_as_svg`, `save_as_html`,
`save_as_canvas_html`, `save_as_webgl_html`, `save_as_webgpu_html`. Live data:
`upsert_kline`, `upsert_kline_timestamped`, `upsert_klines`, `append_kline`,
`update_last_kline`, `replay_next`, `set_replay_window`, `set_interaction`.

SVG is static output; use the HTML emitters for the interactive mouse-tracking
data window (OHLC, change, amplitude, volume, source range and hit Chanlun
objects, following zoom/pan).

## Node.js API

### Installation

The Node binding is built from source under `ffi/node-binding` with NAPI-RS.
There is no npm registry release yet; see [installation.md](installation.md)
for the build and `npm pack` path.

```bash
cd ffi/node-binding && npm ci && npm run build && npm test
```

### TypeScript Definitions

```typescript
export interface MacdResult {
  macd: number[];
  signal: number[];
  hist: number[];
}

export interface StochResult {
  k: number[];
  d: number[];
}

export interface BbandsResult {
  upper: number[];
  middle: number[];
  lower: number[];
}

// Overlap Studies
export function sma(close: number[], timeperiod: number): number[];
export function ema(close: number[], timeperiod: number): number[];
export function bollingerBands(
  close: number[],
  timeperiod: number,
  nbdevup: number,
  nbdevdn: number
): BbandsResult;

// Momentum
export function rsi(close: number[], timeperiod: number): number[];
export function macd(
  close: number[],
  fastperiod: number,
  slowperiod: number,
  signalperiod: number
): MacdResult;
export function stoch(
  high: number[],
  low: number[],
  close: number[],
  fastkPeriod: number,
  slowkPeriod: number,
  slowdPeriod: number
): StochResult;

// Volatility
export function atr(
  high: number[],
  low: number[],
  close: number[],
  timeperiod: number
): number[];

// Volume
export function obv(close: number[], volume: number[]): number[];

// Pattern Recognition
export function cdlDoji(
  open: number[],
  high: number[],
  low: number[],
  close: number[],
  dojiPct: number
): number[];

export function detectDoubleTop(
  high: number[],
  lookback: number,
  tolerance: number
): number[];
```

### Complete Example

```typescript
import {
  sma, ema, rsi, macd, bollingerBands,
  stoch, atr, obv,
  cdlDoji, cdlHammer, detectDoubleTop
} from 'finkit';

const close = Array.from({ length: 100 }, (_, i) => 100 + i + Math.random());
const high = close.map(x => x + Math.random() * 2);
const low = close.map(x => x - Math.random() * 2);
const volume = Array.from({ length: 100 }, () => Math.random() * 4000 + 1000);

const smaResult = sma(close, 20);
const rsiResult = rsi(close, 14);
const macdResult = macd(close, 12, 26, 9);
const bbandsResult = bollingerBands(close, 20, 2, 2);
const atrResult = atr(high, low, close, 14);

const doji = cdlDoji(close, high, low, close, 0.1);
const doubleTops = detectDoubleTop(high, 20, 0.03);
```

## Java API

### Maven Dependency

The Java/JNI binding is packaged from source under `ffi/java-binding`. Maven
Central publication is not enabled yet; build the native library and package
the JAR locally (see [installation.md](installation.md)).

```bash
cargo build -p finkit-java --release --locked
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

### Classes

```java
package com.finkit;

public class Indicators {
    // Overlap Studies (return arrays directly)
    public static native double[] sma(double[] input, int period);
    public static native double[] ema(double[] input, int period);
    // bbands writes into a pre-allocated result object
    public static native void bbands(double[] input, int timePeriod, double nbDevUp, double nbDevDn, BbandsResult result);

    // Momentum
    public static native double[] rsi(double[] input, int period);
    // macd writes into a pre-allocated result object
    public static native void macd(double[] input, int fastPeriod, int slowPeriod, int signalPeriod, MacdResult result);
    // stoch writes into a pre-allocated result object
    public static native void stoch(double[] high, double[] low, double[] close, int fastK, int slowK, int slowD, StochResult result);

    // Volatility
    public static native double[] atr(double[] high, double[] low, double[] close, int period);

    // Volume
    public static native double[] obv(double[] close, double[] volume);
}

// Pattern recognition lives in dedicated classes, not Indicators.
public final class Patterns {
    public static native int[] cdlDoji(double[] open, double[] high, double[] low, double[] close);
    public static native int[] cdlDojiWithThreshold(double[] open, double[] high, double[] low, double[] close, double dojiPct);
    public static native int[] cdlHammer(double[] open, double[] high, double[] low, double[] close);
}

public final class ChartPatterns {
    public static native int[] detectDoubleTop(double[] high, int lookback, double tolerance);
    public static native int[] detectDoubleBottom(double[] low, int lookback, double tolerance);
}

public class MacdResult {
    public double[] macd;
    public double[] signal;
    public double[] hist;
}

public class BbandsResult {
    public double[] upper;
    public double[] middle;
    public double[] lower;
}

public class StochResult {
    public double[] k;
    public double[] d;
}
```

### Complete Example

```java
import com.finkit.Indicators;
import com.finkit.MacdResult;
import com.finkit.BbandsResult;
import com.finkit.Patterns;
import com.finkit.ChartPatterns;

public class Example {
    public static void main(String[] args) {
        double[] close = new double[100];
        for (int i = 0; i < 100; i++) {
            close[i] = 100 + i + Math.random();
        }

        double[] sma20 = Indicators.sma(close, 20);
        double[] rsi14 = Indicators.rsi(close, 14);

        MacdResult macd = new MacdResult();
        Indicators.macd(close, 12, 26, 9, macd);

        BbandsResult bbands = new BbandsResult();
        Indicators.bbands(close, 20, 2.0, 2.0, bbands);

        int[] doji = Patterns.cdlDoji(close, close, close, close);
        int[] doubleTops = ChartPatterns.detectDoubleTop(close, 20, 0.03);

        System.out.println("SMA length: " + sma20.length);
        System.out.println("RSI length: " + rsi14.length);
        System.out.println("MACD length: " + macd.macd.length);
    }
}
```

## Go API

### Installation

The Go binding is a nested source module under `ffi/go-binding/go`. There is no
public `go get` release yet; build the native library and test the module from
a checkout (see [installation.md](installation.md)).

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
go test ./...
```

### Functions

```go
package ta

// Overlap Studies
func Sma(input []float64, period int) ([]float64, error)
func Ema(input []float64, period int) ([]float64, error)
func Bbands(input []float64, period int, nbDevUp, nbDevDn float64) (*BbandsResult, error)

// Momentum
func Rsi(input []float64, period int) ([]float64, error)
func Macd(input []float64, fastPeriod, slowPeriod, signalPeriod int) (*MacdResult, error)
func Stoch(high, low, close []float64, kPeriod, kSlow, dPeriod int) (*StochResult, error)

// Volatility
func Atr(high, low, close []float64, period int) ([]float64, error)

// Volume
func Obv(close, volume []float64) ([]float64, error)
```

Go follows Go naming: every function is exported `CamelCase` (`Sma`, not `SMA`), and a
multi-output indicator returns a pointer to a result struct rather than several
slices. The result payloads are:

```go
type MacdResult struct {
    Macd   []float64
    Signal []float64
    Hist   []float64
}

type BbandsResult struct {
    Upper  []float64
    Middle []float64
    Lower  []float64
}

type StochResult struct {
    K []float64
   D  []float64
}
```

> **No pattern-recognition surface.** Unlike Python and Node, the Go binding does
> **not** expose `CDLDoji` or `DetectDoubleTop`. Candlestick and chart patterns
> are reachable only from those other bindings; the omission is in the binding,
> not in this document.

### Complete Example

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit/ffi/go-binding/go/ta"
)

func main() {
    close := make([]float64, 100)
    for i := 0; i < 100; i++ {
        close[i] = float64(i + 1)
    }

    sma, err := ta.Sma(close, 20)
    if err != nil {
        panic(err)
    }

    rsi, err := ta.Rsi(close, 14)
    if err != nil {
        panic(err)
    }

    macd, err := ta.Macd(close, 12, 26, 9)
    if err != nil {
        panic(err)
    }

    fmt.Printf("SMA length: %d\n", len(sma))
    fmt.Printf("RSI length: %d\n", len(rsi))
    fmt.Printf("MACD length: %d\n", len(macd.Macd))
}
```

## .NET API

### NuGet Package

The .NET binding is built from source under `ffi/dotnet-binding`. There is no
NuGet feed release yet; see [installation.md](installation.md) for the build
and `dotnet pack` path.

```bash
cargo build -p finkit-dotnet --release --locked
dotnet pack ffi/dotnet-binding/src/Finkit/Finkit.csproj -c Release -o dist/dotnet
```

### Classes

```csharp
namespace Finkit;

public static class Indicators
{
    // Overlap Studies
    public static double[] Sma(double[] input, int period);
    public static double[] Ema(double[] input, int period);
    public static BbandsResult Bbands(double[] input, int period, double nbDevUp = 2.0, double nbDevDn = 2.0);

    // Momentum
    public static double[] Rsi(double[] input, int period = 14);
    public static MacdResult Macd(double[] input, int fastPeriod = 12, int slowPeriod = 26, int signalPeriod = 9);

    // Volatility
    public static double[] Atr(double[] high, double[] low, double[] close, int period = 14);

    // Volume
    public static double[] Obv(double[] close, double[] volume);
}

> **Two things that surprise people coming from TA-Lib.** Methods follow the .NET
> framework naming convention (`Sma`, not `SMA`), and `period` has **no default**
> on `Sma`, `Ema` and `Bbands` -- it must be passed explicitly. There is also no
> candlestick or chart-pattern surface; unlike Python and Node, the .NET binding
> does not expose `CDLDoji` or `DetectDoubleTop`.

public class MacdResult
{
    public double[] Macd { get; set; }
    public double[] Signal { get; set; }
    public double[] Hist { get; set; }
}

public class BbandsResult
{
    public double[] Upper { get; set; }
    public double[] Middle { get; set; }
    public double[] Lower { get; set; }
}
```

### Complete Example

```csharp
using System;
using System.Linq;
using Finkit;

class Program
{
    static void Main()
    {
        var close = Enumerable.Range(0, 100)
            .Select(i => 100.0 + i + new Random().NextDouble())
            .ToArray();

        var sma20 = Indicators.Sma(close, 20);
        var rsi14 = Indicators.Rsi(close, 14);
        var macd = Indicators.Macd(close);

        Console.WriteLine($"SMA length: {sma20.Length}");
        Console.WriteLine($"RSI length: {rsi14.Length}");
        Console.WriteLine($"MACD length: {macd.Macd.Length}");
    }
}
```

## WebAssembly API

### Installation

The WASM module is built from the `finkit-wasm` crate for the
`wasm32-unknown-unknown` target. There is no npm registry release yet; see
[installation.md](installation.md) for the build path.

```bash
rustup target add wasm32-unknown-unknown
cargo build -p finkit-wasm --target wasm32-unknown-unknown --release --locked
```

### Functions

```typescript
// Initialize WASM module
export default function init(): Promise<void>;

// Overlap Studies
export function sma(close: number[], timeperiod?: number): number[];
export function ema(close: number[], timeperiod?: number): number[];

// Momentum
export function rsi(close: number[], timeperiod?: number): number[];
export function macd(
  close: number[],
  fastperiod?: number,
  slowperiod?: number,
  signalperiod?: number
): { macd: number[]; signal: number[]; hist: number[] };

// Volatility
export function atr(
  high: number[],
  low: number[],
  close: number[],
  timeperiod?: number
): number[];

// Volume
export function obv(close: number[], volume: number[]): number[];
```

### Complete Example

```typescript
import init, { sma, rsi, macd } from 'finkit-wasm';

async function main() {
  await init();

  const close = Array.from({ length: 100 }, (_, i) => i + 1);
  const smaResult = sma(close, 20);
  const rsiResult = rsi(close, 14);
  const macdResult = macd(close, 12, 26, 9);

  console.log(`SMA length: ${smaResult.length}`);
  console.log(`RSI length: ${rsiResult.length}`);
  console.log(`MACD length: ${macdResult.macd.length}`);
}

main();
```

## CLI API

### Installation

```bash
cargo install finkit
```

### Commands

```bash
# Calculate indicators
finkit sma --input data.csv --period 14 --output sma.csv
finkit ema --input data.csv --period 14
finkit rsi --input data.csv --period 14
finkit macd --input data.csv --fast 12 --slow 26 --signal 9
finkit bbands --input data.csv --period 20 --nbdevup 2.0 --nbdevdn 2.0
finkit atr --input data.csv --period 14

# Detect patterns
finkit patterns --input data.csv --format json
finkit candlestick --input data.csv
finkit chart-patterns --input data.csv

# Export options
finkit rsi --input data.csv --output rsi.csv --format csv
finkit rsi --input data.csv --output rsi.json --format json
```

### CSV Input Format

```csv
timestamp,open,high,low,close,volume
2024-01-01,100.0,101.0,99.0,100.5,1000
2024-01-02,100.5,102.0,100.0,101.0,1200
...
```

### Output Format

```json
{
  "indicator": "RSI",
  "parameters": { "timeperiod": 14 },
  "data": [
    { "timestamp": "2024-01-01", "value": null },
    { "timestamp": "2024-01-15", "value": 52.34 },
    ...
  ]
}
```

## Error Types

### Rust

```rust
#[derive(Debug, Error)]
pub enum TaError {
    #[error("Invalid period: {0}")]
    InvalidPeriod(usize),

    #[error("Insufficient data: need {needed}, got {actual}")]
    InsufficientData { needed: usize, actual: usize },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Computation error: {0}")]
    ComputationError(String),
}
```

### Python

```python
class FinkitError(Exception):
    """Base exception for every Finkit error."""
    pass

class InsufficientDataError(FinkitError, ValueError):
    """Raised when the input is shorter than the requested period."""
    pass

class InvalidParameterError(FinkitError, ValueError):
    """Raised when a parameter is outside its valid range."""
    pass

class IndicatorNotFoundError(FinkitError, KeyError):
    """Raised when an indicator id does not exist in the registry."""
    pass
```

`InsufficientDataError` and `InvalidParameterError` also inherit from
`ValueError`, and `IndicatorNotFoundError` also inherits from `KeyError`, so code
written against the built-in exceptions keeps working. Catch `FinkitError` to
handle anything this library raises.

### Node.js

```typescript
export class TaLibError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: Record<string, any>
  ) {
    super(message);
    this.name = 'TaLibError';
  }
}
```
