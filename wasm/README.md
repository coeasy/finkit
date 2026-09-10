# Finkit WebAssembly Binding

`finkit-wasm` exposes a WebAssembly surface over the Rust Finkit core for browser/JavaScript runtimes.

## Current support contract

The crate contains `wasm-bindgen` exports for a broad set of indicators plus formula, streaming, transform, pattern, and related helpers implemented in `wasm/src/`.

The multi-language release workflow compiles the crate for `wasm32-unknown-unknown` and stores the resulting `.wasm` module as a CI artifact. That compile artifact is not the same thing as a published npm package or a complete browser bundle: JavaScript/TypeScript glue still needs to be generated for the chosen deployment target.

## Requirements

- Rust 1.85+;
- the `wasm32-unknown-unknown` target.

```bash
rustup target add wasm32-unknown-unknown
```

## Build

From the repository root:

```bash
cargo build \
  -p finkit-wasm \
  --target wasm32-unknown-unknown \
  --release \
  --locked
```

The raw module is produced at:

```text
target/wasm32-unknown-unknown/release/finkit_wasm.wasm
```

## JavaScript glue

For an application-facing package, use a `wasm-bindgen`/`wasm-pack` toolchain compatible with the crate's locked `wasm-bindgen` dependency and select the correct target (`web`, `bundler`, or Node) for the consuming application.

Do not publish or document a generic `npm install finkit-wasm` command until an actual package containing the generated JS/TypeScript glue and `.wasm` payload has been built, published, and smoke-tested.

## API expectations

WebAssembly is a language/runtime binding, not a separate numerical implementation. It delegates calculations to the same Rust core, but the exported function set, memory transfer cost, JavaScript types, and runtime constraints are binding-specific.

For exact source exports, inspect `wasm/src/lib.rs`, `wasm/src/streaming.rs`, and `wasm/src/transforms.rs` in the same release tag.

### Browser K-line chart facade

`WasmKlineChart` exposes the shared chart scene used by the Rust, Python and Node bindings. It supports:

- self-contained interactive SVG HTML with TDX-style floating OHLCV data window, crosshair, pan/zoom and keyboard interaction;
- Canvas 2D HTML output with the same data-window interaction and a lower DOM overlay cost;
- `toWebglHtml()` defaults to WebGL2 instanced rendering; `toWebgpuHtml()` explicitly opts into WebGPU/WGSL, with fallback
  geometry, and finally Canvas 2D while sharing the indicator, Chan and tooltip scene;
- JSON scene/data export with source-index mapping, viewport/LOD configuration and render statistics;
- MA, MACD, RSI, custom indicator series and event markers;
- real-time `upsertKline`/`upsertKlineTimestamped`/`upsertKlines`, replay windows, configurable Chan
  thresholds, single-period Chan analysis and multi-period Chan analysis.

Example after generating the JavaScript glue:

```js
const chart = new WasmKlineChart(
  dates, opens, highs, lows, closes, volumes,
  "zh-CN", "行情", 1280, 720
);

chart.addMa([5, 20]);
chart.addMacd(12, 26, 9);
chart.addChan(3, "standard", true, true);
chart.setChanThresholds(0.001, 0.002, 0.1, 0.001);
chart.setInteraction(true, true, true, true, true);

document.querySelector("#chart").innerHTML = chart.toHtml();
// Or: chart.toCanvasHtml() for a Canvas 2D renderer.
// For dense data: chart.toWebglHtml() (WebGL2 -> Canvas 2D).
// Optional: chart.toWebgpuHtml() (WebGPU -> WebGL2 -> Canvas 2D).
chart.upsertKline("2024-01-02", 10, 11, 9.8, 10.6, 120000);
chart.upsertKlines(
  ["2024-01-02", "2024-01-03"],
  [10, 10.6], [11, 11.2], [9.8, 10.2], [10.6, 11], [120000, 130000]
);
```

For a timestamp-aware live feed, call `upsertKlineTimestamped(timestamp, date,
open, high, low, close, volume)`. The source `KlineData` timestamp index must
be complete and strictly increasing; date-only calls remain compatible.

Use `addCustomIndicator(name, values)` for a browser-computed series. The values must align with the source bars; appended real-time bars receive a `NaN` placeholder until the caller supplies a recalculated series. Call `setCustomIndicator(name, values)` to replace an existing series after a real-time recalculation without registering a duplicate indicator.

For dependency-aware custom indicator graphs, use `computeComposite`. Definitions can
reference OHLCV fields, earlier definitions, constants (`const:2`), arithmetic
operations (`add`, `sub`, `mul`, `div`, `min`, `max`, `weighted_average`) and the
same built-in indicator calls exposed by the Rust core:

```js
const values = computeComposite(
  [1, 2, 3, 4, 5, 6],
  [
    { name: "trend", function: "sma", inputs: ["close"], params: [3] },
    { name: "signal", function: "cross_up", inputs: ["close", "trend"], params: [] },
  ],
  ["trend", "signal"],
  undefined, undefined, undefined, undefined,
);
```

The returned object is keyed by the requested output names. This API keeps the
calculation engine identical across Rust, Python, Node and WebAssembly; the binding
only handles input/output conversion.

The module also exports `resolveMarketSession(market, timestamp, timezone?,
holidays?, sessions?, specialSessions?)` and
`resolveMarketSessionConfig(configJson, timestamp)`. Market presets are
`a_share`, `china_futures`, `hong_kong`, `us_equity` and `crypto`; the U.S.
preset resolves New York DST transitions, while exchange-published holiday
dates, product-specific sessions and half-day overrides can be supplied by the
application. `sessions` accepts `{openSeconds, closeSeconds}` objects and
`specialSessions` accepts `{date, sessions}` objects. The JSON form preserves
the calendar `source` and `revision` for reproducible backtests and replays.

## Validation

The release workflow verifies that the crate compiles for the real wasm32 target and that the generated `.wasm` file is non-empty. Host-only workspace compilation is not treated as proof of WebAssembly support.

## Related documentation

- [Language bindings](../docs/language-bindings.md)
- [Complete usage guide](../docs/usage.md)
- [Formula engine](../docs/formula.md)

## License

MIT OR Apache-2.0
