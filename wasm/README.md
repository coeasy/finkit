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

## Validation

The release workflow verifies that the crate compiles for the real wasm32 target and that the generated `.wasm` file is non-empty. Host-only workspace compilation is not treated as proof of WebAssembly support.

## Related documentation

- [Language bindings](../docs/language-bindings.md)
- [Complete usage guide](../docs/usage.md)
- [Formula engine](../docs/formula.md)

## License

MIT OR Apache-2.0
