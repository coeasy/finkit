# Finkit Node.js Binding

This directory contains the NAPI-RS binding for Finkit `v0.1.4`.

## Status

The Node binding is **source-build and CI-packaging validated**. The multi-language workflow builds the native module, runs the real `node:test` smoke suite, stages the platform native file, and validates `npm pack` on the currently exercised CI target.

The GitHub `v0.1.4` Release does not currently contain Node packages, and this documentation does not assume that the root `finkit` package or all optional native platform packages have been published to npm.

## Requirements

- Node.js 16+;
- npm;
- Rust 1.85+;
- the native compiler/linker required by the host platform.

## Build from source

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
git checkout v0.1.4
cd ffi/node-binding

npm install
npm run build
npm test
```

The smoke test loads the real native module and verifies SMA output. It also checks that the exported surface contains `sma`, `ema`, `rsi`, `macd`, `formulaEval`, and `formulaValidate`.

## Local usage

After the native module has been built/staged for the host platform:

### ESM

```javascript
import * as finkit from './index.mjs'

const close = [1, 2, 3, 4, 5]
const sma = finkit.sma(close, 3)
console.log(sma[sma.length - 1]) // 4
```

### CommonJS

```javascript
const finkit = require('./index.js')

const close = [1, 2, 3, 4, 5]
const rsi = finkit.rsi(close, 3)
console.log(rsi)
```

## K-line chart export and interaction

`KlineChartNapi` also exposes `toHtml()`/`saveAsHtml()` for a self-contained
TDX-style interactive chart. The HTML path supports the floating OHLCV and
indicator data window, linked sub-panel crosshair, pointer zoom/pan and
keyboard navigation. `addCustomIndicator()` and `addEventMarker()` use the
same semantic scene as Rust and Python; `toJson()` exports the display data,
source ranges and hit regions for another frontend.

For dense charts, `toCanvasHtml()`/`saveAsCanvasHtml()` emit one Canvas 2D
surface plus a compact command stream, avoiding one DOM node per primitive;
the output retains the OHLCV, indicator and semantic event/Chan hit data
window.

```javascript
const chart = new finkit.KlineChartNapi(data, 'zh', '行情', 1200, 600)
chart.addCustomIndicator('信号线', signal)
chart.addEventMarker(120, '突破候选', close[120], '#f59e0b')
chart.setInteraction(true, true, true, true, true)
chart.addChan(6, true, 'standard', 'configurable', 'dynamic', 0, true)
chart.setChanThresholds(0.001, 0.002, 0.1, 0.001)
chart.setViewport(0, 2000, 1600, 720, 40, true)
chart.setLodPolicy('auto')
const html = chart.toHtml()
```

`addChanMulti([1, 5, 15], 'standard')` enables the same source-index-mapped
multi-timeframe Chan overlay used by the Rust and Python APIs.

For feed-side session routing, `resolveMarketSession()` supports `a_share`,
`china_futures`, `hong_kong`, `us_equity` and `crypto`. It accepts a timezone,
exchange-published holiday dates, regular session overrides and per-date
half-day/closure overrides. The U.S. preset resolves New York DST transitions;
the other presets use their fixed exchange timezone.

```javascript
const session = finkit.resolveMarketSession(
  'us_equity',
  Date.parse('2024-03-11T13:30:00Z'),
  'America/New_York',
  ['2024-07-03'],
)
```

Live feeds can call `upsertKline()` for one quote or `upsertKlines([{ date,
timestamp?, open, high, low, close, volume }])` for a batch. The optional
Unix-second timestamp preserves the exchange time index; a repeated date
revises the current bar and a new date appends one. Batch calls rebuild once
at the end. Existing date-only calls remain compatible.
For replay,
`setReplayWindow()` and `replayNext()` expose the same half-open source range
semantics as the Python binding.

For dense browser charts, `toWebgpuHtml()` / `toWebglHtml()` use the same
document with WebGPU preferred, WebGL2 instanced price/volume geometry as the
next fallback, and Canvas 2D as the final fallback. Indicators, Chan
structures and labels stay on the Canvas overlay; `saveAsWebgpuHtml(path)` and
`saveAsWebglHtml(path)` write the same document. The binding remains a
compute/chart layer and does not own a market-data source or trading session.

## Package layout

`package.json` declares the root package `finkit` version `0.1.4`, ESM/CommonJS entry points, TypeScript definitions, and optional platform-native packages.

The declared platform package set currently includes:

- macOS arm64/x64;
- Linux arm64 GNU/musl;
- Linux x64 GNU/musl;
- Windows x64/arm64 MSVC.

A declaration in `optionalDependencies` is **not** proof that the corresponding package has been built, tested, and published. Before publishing the root package, every platform package you intend to advertise must contain the correct `finkit.node` native file and must be published at the matching version.

## Validate packaging locally

```bash
npm run build
npm test
npm pack
```

For a release workflow, stage the generated host `.node` file into the matching platform package as `finkit.node`, pack that platform package, then pack the root package. The permanent multi-language workflow is the reference implementation for this staging contract.

## Formula entry points

The native binding exposes formula functions such as `formulaEval` and `formulaValidate`. Use the repository formula documentation for language semantics:

- [Formula engine](../../docs/formula.md)
- [Formula grammar](../../docs/formula/grammar.md)
- [Generated formula function catalog](../../docs/generated/formula-functions.md)

## Composite indicators

`computeComposite()` evaluates a dependency-aware graph of custom indicators;
definitions can reference OHLCV series, constants, previous definitions, and
built-ins such as `sma`, `ema`, `rsi`, `atr`, `rolling_std`, `rolling_min`,
`rolling_max`, `threshold`, `between`, `clip`, `cross_up` and `cross_down`:

```js
const result = computeComposite(close, [
  { name: 'trend', function: 'sma', inputs: ['close'], params: [20] },
  { name: 'signal', function: 'cross_up', inputs: ['close', 'trend'], params: [] },
], ['trend', 'signal'])
```

`weighted_average` is also available; pass one weight per input in `params`,
for example `{ name: 'blend', function: 'weighted_average', inputs: ['close',
'volume'], params: [3, 1] }`.

## Indicators

The exact supported indicator registry can change as the Rust core evolves. Use the generated registry instead of a hard-coded count:

- [Indicator catalog](../../docs/generated/indicators.md)
- [Human-readable indicator guide](../../docs/indicators.md)

## Distribution note

Do not use `npm install finkit` as a guaranteed v0.1.4 installation instruction until the npm registry and all required native dependency packages have been verified. For the current release, source build is the documented Node path.

## License

MIT OR Apache-2.0
