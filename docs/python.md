# Python Guide

Finkit Python uses PyO3 + maturin and ships `v0.1.10` as platform-specific CPython stable-ABI wheels (`cp38-abi3`). The same wheel for a platform is validated across GIL-enabled CPython 3.8-3.14 on the supported matrix.

## Release status

`v0.1.10` is published on GitHub Releases. The verified wheel families are:

- Linux x86_64 — manylinux 2.17 / manylinux2014;
- Windows x86_64 — `win_amd64`;
- macOS x86_64;
- macOS arm64.

Not in the v0.1.10 wheel matrix: Linux arm64, musllinux, 32-bit Windows, PyPy, and free-threaded CPython.

The GitHub Release is the documented installation source for v0.1.10. Registry publication is separate; do not assume PyPI contains this exact package/version unless independently verified.

## Install a Release wheel

Download the matching `finkit-0.1.10-cp38-abi3-*.whl` from:

`https://github.com/coeasy/finkit/releases/tag/v0.1.10`

Then install:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.10-<matching-platform>.whl
```

Verify outside the source tree:

```bash
cd /tmp  # use another clean directory on Windows
python - <<'PY'
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)
rsi = ta.rsi(close, timeperiod=14)
assert len(rsi) == 100
assert np.isfinite(rsi[-1])
print("Finkit OK", rsi[-1])
PY
```

## Build from source

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
git checkout v0.1.10

python3 -m venv .venv
source .venv/bin/activate  # PowerShell: .\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest

cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

Source builds need Rust 1.85+ and the platform's native compiler/linker.

## Input contract

For best performance and predictable behavior:

```python
import numpy as np

close = np.ascontiguousarray(close, dtype=np.float64)
```

Use one-dimensional arrays. OHLCV arrays passed to one computation must have the same length. Rolling outputs preserve input length and normally start with warm-up `NaN` values.

## Indicator examples

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)
high = close + 1.0
low = close - 1.0
open_ = close - 0.25
volume = np.full(close.size, 1_000_000.0)

sma20 = ta.sma(close, timeperiod=20)
ema20 = ta.ema(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, 12, 26, 9)
atr14 = ta.atr(high, low, close, timeperiod=14)
obv = ta.obv(close, volume)
```

The package wrapper converts public numeric native list/tuple results to NumPy arrays recursively.

## Chanlun structures and chart overlay

The Chanlun API keeps the structural stages explicit: inclusion-free bars,
fractals, strokes, segments, and centers. It uses zero-based bar indices so
the result can be joined back to a pandas index without imposing a timestamp
type on the native core.

```python
structures = ta.chan_analyze(
    open_, high, low, close, volume,
    min_stroke_bars=6,
    variant="standard",              # conservative / standard / aggressive
    stroke_policy="configurable",    # configurable / fixed5 / fixed6 / fixed7 / threshold
    center_policy="dynamic",         # three_stroke / dynamic / hierarchical
    min_stroke_change_ratio=0.01,
    signal_min_strength=0.2,
)
print(structures["strokes"])
print(structures["centers"])
print(structures["signals"])
```

多周期可以显式指定聚合因子，也可以让核心库根据数据长度自动选择：

```python
mtf = ta.chan_analyze_multi(
    open_, high, low, close, volume,
    factors=None,       # 自动选择；也可传 [1, 5, 20]
    auto_levels=3,
    min_frame_bars=20,
    variant="standard",
)
for frame in mtf["frames"]:
    print(frame["label"], frame["analysis"]["trend"])
```

时间戳数据可以按真实秒数聚合；不传 `durations_seconds` 时会按观测到的
基础周期自动选择层级，`origin_seconds` 可用于对齐交易所开盘边界：

```python
mtf = ta.chan_analyze_multi_timestamps(
    timestamps, open_, high, low, close, volume,
    durations_seconds=[5 * 60, 20 * 60],
    origin_seconds=session_open_unix,
)
print(mtf["resonance_direction"], mtf["resonance_score"])
```

对于真实市场时间轴，可使用内置市场 session 和时区适配器。支持
`a_share`、`china_futures`、`hong_kong`、`us_equity` 和 `crypto`；节假日、临时休市、半日市和具体期货品种 session 可由用户覆盖：

```python
mtf = ta.chan_analyze_multi_timestamps_calendar(
    timestamps, open_, high, low, close, volume,
    market="us_equity",
    durations_seconds=[30 * 60, 4 * 60 * 60],
    # IANA 常用名称；美股自动处理夏令时
    timezone="America/New_York",
    holidays=["2026-07-03"],
    # (open_seconds, close_seconds)，用于期货品种或半日市覆盖
    sessions=[(9 * 3600 + 30 * 60, 16 * 3600)],
    special_sessions=[
        ("2026-11-27", [(9 * 3600 + 30 * 60, 13 * 3600)]),
    ],
)
```

市场 preset 只提供稳定的常规 session 与时区；交易所每年发布的假日表、临时休市和品种差异应由数据接入层传入，避免把会变化的交易所公告固化在数值核心。

如果只需要查询某个时间戳属于哪个 session，可直接使用统一的市场日历解析器；
它与 Node/WASM 的市场、时区和覆盖参数保持一致；Python 返回字典使用下划线字段，
并额外保留解析后的 `market` 与 `timezone`：

```python
session = ta.resolve_market_session(
    "us_equity",
    timestamp,
    timezone="America/New_York",
    holidays=["2026-07-03"],
    # 期货品种、半日市和临时交易时段可覆盖 preset
    sessions=[(9 * 3600 + 30 * 60, 16 * 3600)],
)
if session is not None:
    print(session["session_day"], session["open_timestamp"], session["close_timestamp"])
```

需要保留交易所文件的 `source`/`revision` 时，使用上面的 JSON 或年度 CSV
解析器；直接 preset 解析器适合日常查询和轻量适配。

组合指标可以用声明式依赖图表达，计算结果按名称返回；同一中间结果只会在一次请求中计算一次：

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

`definitions` 的输入可以是 `close/high/low/volume` 等原始序列、其他定义名，
或 `const:<数字>`。内置函数覆盖常用均线、RSI、ATR、MACD、BOLL、VWMA、收益率、
Z-score、滚动统计、`threshold`/`between`/`clip` 阈值算子和交叉检测；Rust 层同时
提供注册自定义函数的扩展点。`evaluate_borrowed` 路径对原始序列采用零拷贝借用，
只有组合运算结果进入拥有内存，适合实时图表和批量策略计算。

To render the same structures on the main K-line panel:

```python
chart = ta.KlineChart(data, language="zh", title="缠论结构")
chart.add_chan(min_stroke_bars=6, show_labels=True)
# 大数据总览：只绘制最新窗口，并自动使用语义正确的 OHLCV 聚合
chart.set_viewport(end=2000, pixel_width=1600, overscan_bars=40, follow_latest=True)
chart.set_lod_policy("auto")
chart.set_layer_visible("volume", True)
svg = chart.to_svg_string()
```

数据接入层保留 `validate()` 的长度兼容检查，并额外提供
`validate_ohlcv()` 与 `validation_errors()`，可在进入指标、缠论和图表前检查
非有限值、OHLC 越界和负成交量。

HTML 输出支持类似通达信的鼠标浮动数据窗：在主图区域移动鼠标即可查看当前
K 线的 OHLC、涨跌、振幅、成交量、源数据区间以及命中的缠论对象；滚轮缩放、
拖动平移后数据窗会继续跟随当前显示位置。SVG 是静态输出，交互请使用
`to_html_string()` 或 `save_as_html()`。

对于超密集图表，可使用 `to_canvas_html()` 或 `save_as_canvas_html()` 输出
Canvas 2D 命令流 HTML。它只创建一个画布，仍保留 OHLCV、指标值、事件和缠论命中
浮窗，适合降低浏览器 DOM 压力；更复杂的自定义前端可继续消费
`to_json_string()` 的场景和数据载荷自行绘制。

数据窗还会显示当前可见的 MA/EMA、BOLL、MACD、RSI、KDJ、SAR 和自定义指标值。
自定义指标可直接从 Python 注册：

```python
chart.add_custom_indicator("我的信号线", signal_values.tolist())
chart.add_event_marker(120, "突破候选", value=float(highs[120]))
chart.set_chan_thresholds(0.001, 0.002, 0.1, 0.001)
chart.set_interaction(
    show_crosshair=True,
    show_data_window=True,
    enable_pan_zoom=True,
    enable_keyboard=True,
)
```

HTML 交互使用 Pointer Events，鼠标、触控板和触屏共用同一套缩放/平移路径；
左右方向键可在获得焦点的图表上逐根查看 K 线。若嵌入端需要完全静态的 HTML，
可使用 `chart.set_interaction(enabled=False)`。

实时行情可以直接使用 `upsert_kline()`：同一日期会修订当前 K 线，日期变化
则追加新 K 线；`append_kline()` 和修订接口会拒绝非法 OHLCV。研究回放可用
`set_replay_window()` 设置窗口和游标，再用 `replay_next()` 逐步推进，返回值
始终是源数据的半开区间 `[start, end)`。

高频接入可使用 `upsert_klines([(date, open, high, low, close, volume), ...])`，
一次校验并批量更新，导出图表时只重建一次。图表也支持
`add_chan_multi(factors=None, variant="standard")`，不传周期因子时由核心库
按数据长度自动选择多周期层级；`set_chan_thresholds` 可独立调整结构和候选点
阈值。

如果行情源提供交易所 Unix 秒级时间戳，`KlineData(..., timestamps=[...])` 可将
真实时间轴传入图表；实时更新使用 `chart.upsert_kline_timestamped(timestamp, date,
open, high, low, close, volume)`，可保持交易日历、缺口和浮窗时间。旧版
`upsert_kline()`/`upsert_klines()` 仍可用于 date-only 数据。

大数据量浏览器输出默认使用 `chart.to_webgl_html()`：WebGL2 实例化绘制失败时降级到 Canvas 2D。
如需显式启用 WebGPU，可使用 `chart.to_webgpu_html()`，其链路为 WebGPU/WGSL → WebGL2 → Canvas 2D；指标、缠论和文字
保留 Canvas overlay。两个入口生成同一份可复现图表协议。
Finkit 不负责行情数据源或交易连接，宿主应用只需注入标准化的 OHLCV、时间戳和
事件数据。

交易所年度日历可直接通过 `resolve_market_session_csv(csv, market, timestamp,
timezone=None)` 解析。CSV 使用 `date,status,sessions` 三列；例如：

```text
date,status,sessions
2026-01-01,closed,
2026-01-02,open,09:30-11:30;13:00-15:00
```

该入口与 Rust、Node、WASM 共用日期、时区、跨午夜 session 和严格校验逻辑；需要
`source/revision` 审计元数据时使用 JSON 日历配置。

多周期结构可由核心库预先计算后注入 Rust 图表；如需在图表中显示高周期
的分型、信号和背驰标注，可在 `add_chan` 中设置
`show_multi_timeframe_annotations=True`。自定义指标序列通过 Rust
`KlineChart.set_custom_indicator_series(name, values)` 注册，序列长度必须
与源数据一致，概览聚合时会按 `source_ranges` 取每个桶的最新值。
JSON 导出现在包含与当前图元一致的 `data`、`revision`、`source_offset` 和
`source_ranges`，便于 WASM/原生前端直接复用数据窗和聚合索引。

The overlay is disabled by default. The initial implementation uses strict
three-bar fractals and a conservative three-stroke segment definition; the
rules and next-stage extension points are recorded in
`docs/chan-visualization-roadmap-zh.md`.

## Handling warm-up values

```python
ready = np.isfinite(sma20) & np.isfinite(rsi14)
strategy_signal = np.zeros(close.size, dtype=bool)
strategy_signal[ready] = (close[ready] > sma20[ready]) & (rsi14[ready] > 50)
```

Do not treat leading `NaN` values as a calculation failure; they represent insufficient lookback history for many rolling indicators.

## `CompiledFormula`

Use `CompiledFormula` when the same formula is executed repeatedly. The object keeps the parsed/optimized plan and formula-engine caches alive across calls.

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

### `eval()`

`eval()` copies the input arrays into an owned formula context. This is the correct starting point when you plan to call `append_bar()` later.

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10000)
plan.append_bar(101.0, 103.0, 100.0, 102.5, 1_200_000.0)
latest = plan.eval_last()
```

### `eval_zero_copy()`

`eval_zero_copy()` borrows contiguous `float64` NumPy OHLCV buffers for the synchronous evaluation:

```python
out = plan.eval_zero_copy(open_, high, low, close, volume)
value = out["__result__"]
```

Rules:

- every required input must be a contiguous, one-dimensional `float64` NumPy array;
- all arrays must have equal length and be non-empty;
- keep the arrays alive and do not concurrently resize/mutate them while the call is executing;
- direct fast-path formulas can avoid input materialization, while complex formulas may allocate intermediate arrays;
- `eval_zero_copy()` does not establish the retained streaming context used by `append_bar()`.

### `eval_range()`

Evaluate `[start, end)`:

```python
out = plan.eval_range(
    open_, high, low, close, volume,
    900, 1000,
)
```

The runtime uses dependency/lookback information to include the required prefix conservatively.

For high-frequency chart-window refreshes, use `eval_range_zero_copy()` with
contiguous float64 arrays. It borrows the full input history, computes the
required dependency window, and does not create a retained owned context:

```python
out = plan.eval_range_zero_copy(
    open_, high, low, close, volume,
    900, 1000,
)["__result__"]
```

### `eval_last()`

With arrays:

```python
latest = plan.eval_last(open_, high, low, close, volume)
```

Or reuse a context created by `eval()` / `eval_range()`:

```python
plan.eval(open_, high, low, close, volume)
latest = plan.eval_last()
```

### `append_bar()`, `reserve_bars()`, `reset()`

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(5000)
plan.append_bar(102.0, 104.0, 101.0, 103.5, 900_000.0)
latest = plan.eval_last()
plan.reset()
```

`reset()` removes the retained market context but does not discard the compiled formula itself.

### `analyze()` and `compatibility_report()`

Inspect a formula before running it:

```python
analysis = plan.analyze()
print(analysis["required_lookback"], analysis["supports_streaming"])
report = plan.compatibility_report("tdx")
print(report["sma_policy"], report["functions"])
```

The analysis identifies input dependencies, stateful and future-data functions,
side effects, unknown functions and conservative streaming suitability. The
compatibility report identifies terminal semantic policies and marks each
function as exact, near, approximate, host-required or unsupported.

## Formula result dictionaries

`eval()` can return named formula variables plus `__result__`. Internal common-subexpression variables are filtered from the Python-facing dictionary.

```python
out = plan.eval(open_, high, low, close, volume)
print(out.keys())
print(out["__result__"][-1])
```

See [formula-runtime.md](formula-runtime.md) and [formula-runtime-contract.md](formula-runtime-contract.md) for the detailed contract.

## Pandas

Pandas is optional. Explicit NumPy conversion is the simplest integration:

```python
import pandas as pd
import numpy as np
import finkit as ta

frame = pd.DataFrame({"close": np.arange(1.0, 101.0)})
close = frame["close"].to_numpy(dtype=np.float64, copy=False)
frame["rsi14"] = ta.rsi(close, timeperiod=14)
```

The Python package also contains an optional `TaAccessor`. Install pandas when using/testing it:

```bash
python -m pip install pandas
python -m pytest ffi/python-binding/tests/test_accessor.py -q
```

## Stable exceptions

The Python wrapper exposes:

- `FinkitError`;
- `InsufficientDataError`;
- `InvalidParameterError`;
- `IndicatorNotFoundError`.

Common native validation failures are translated at the package boundary. Invalid MACD periods, invalid period arguments, insufficient data, and empty inputs should be handled explicitly in application code.

## Patterns

```python
doji = ta.cdl_doji(open_, high, low, close)
hammer = ta.cdl_hammer(open_, high, low, close)
engulfing = ta.cdl_engulfing(open_, high, low, close)

heads = ta.detect_head_shoulders(high)
double_tops = ta.detect_double_top(high)
```

Pattern algorithms have lookbacks; an initial region with no pattern signal is expected.

## Build a wheel locally

```bash
cd ffi/python-binding
maturin build --release --locked --out dist --compatibility pypi --interpreter python
```

Install the generated wheel from a clean directory and run tests against the installed package. Avoid verifying from a working directory that can shadow the installed `finkit` package.

## CI release behavior

The Python Wheels workflow:

1. builds the four v0.1.10 platform wheels;
2. installs/tests each platform wheel outside the source tree;
3. reuses the Linux ABI3 wheel across CPython 3.8-3.14 compatibility jobs;
4. validates package version, wheel metadata, and platform coverage;
5. on the explicit release path, builds the `.crate`, Linux CLI, checksum file, and creates/updates the GitHub Release.

A normal pull request does not publish Release assets.

## Troubleshooting

### `is not a supported wheel on this platform`

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
```

Match OS and CPU architecture. ABI3 spans supported CPython minor versions; it does not span OS/architecture boundaries.

### `ModuleNotFoundError: No module named 'finkit'`

```bash
python -m pip show finkit
python -c "import sys; print(sys.executable)"
```

Confirm that pip and Python use the same environment, and test outside the repository/source package directory.

### NumPy import/ABI failure

```bash
python -m pip install --upgrade pip numpy
python -m pip install --force-reinstall ./finkit-0.1.10-<matching-platform>.whl
```

### `eval_zero_copy()` rejects an array

Normalize it:

```python
arr = np.ascontiguousarray(arr, dtype=np.float64)
```

Also ensure every OHLCV array is one-dimensional and the lengths match.

## Related documentation

- [Complete usage guide](usage.md)
- [Installation](installation.md)
- [Formula engine](formula.md)
- [Formula runtime](formula-runtime.md)
- [Indicators](indicators.md)
- [Python binding source README](../ffi/python-binding/README.md)
