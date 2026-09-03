# Finkit Node.js Binding

This directory contains the NAPI-RS binding for Finkit `v0.1.3`.

## Status

The Node binding is **source-build and CI-packaging validated**. The multi-language workflow builds the native module, runs the real `node:test` smoke suite, stages the platform native file, and validates `npm pack` on the currently exercised CI target.

The GitHub `v0.1.3` Release does not currently contain Node packages, and this documentation does not assume that the root `finkit` package or all optional native platform packages have been published to npm.

## Requirements

- Node.js 16+;
- npm;
- Rust 1.85+;
- the native compiler/linker required by the host platform.

## Build from source

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
git checkout v0.1.3
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

## Package layout

`package.json` declares the root package `finkit` version `0.1.3`, ESM/CommonJS entry points, TypeScript definitions, and optional platform-native packages.

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

## Indicators

The exact supported indicator registry can change as the Rust core evolves. Use the generated registry instead of a hard-coded count:

- [Indicator catalog](../../docs/generated/indicators.md)
- [Human-readable indicator guide](../../docs/indicators.md)

## Distribution note

Do not use `npm install finkit` as a guaranteed v0.1.3 installation instruction until the npm registry and all required native dependency packages have been verified. For the current release, source build is the documented Node path.

## License

MIT OR Apache-2.0
