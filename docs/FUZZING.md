# Fuzzing Guide (T-2)

This crate ships with a [`cargo-fuzz`][cargo-fuzz] harness targeting the
formula engine and the batch indicator entry points. The harness is
intentionally **not** part of the main `cargo` workspace because it
requires a nightly toolchain and the `cargo-fuzz` CLI.

## Quick start

```bash
# 1. Install cargo-fuzz
cargo install cargo-fuzz

# 2. Run the formula fuzzer for 50K iterations
cargo +nightly fuzz run formula -- -runs=50000

# 3. Run the indicator fuzzer
cargo +nightly fuzz run indicators -- -runs=50000
```

The first run builds the harness; subsequent runs reuse the build cache.

## Targets

| Target      | Entry point | What it stresses                                  |
|-------------|-------------|---------------------------------------------------|
| `formula`   | `fuzz_targets/formula.rs`   | `FormulaEngine::eval` + `eval_partial` on arbitrary strings. Contract: no panic, output length is always `data_len`. |
| `indicators`| `fuzz_targets/indicators.rs`| `sma`/`ema`/`wma`/`dema`/`rsi`/`macd`/`bbands` on arbitrary `f64` slices (incl. NaN/Inf). Contract: no panic. |

## Acceptance criteria

- **50 000 iterations** run with zero panics on CI before each release.
- The fuzzer builds with `--release -C opt-level=3 -C lto=fat`
  (configured in `fuzz/Cargo.toml`).
- A regression in the formula parser, evaluator, or any batch
  indicator surfaces as a crash report in `fuzz/artifacts/<target>/`
  which is then minimized to a reproducer.

## OSS-Fuzz integration

The harness is `cargo-fuzz` compatible, which is the format accepted
by [OSS-Fuzz][oss-fuzz]. To enable OSS-Fuzz:

1. Fork the [google/oss-fuzz](https://github.com/google/oss-fuzz) repo.
2. Add a project entry under `projects/AlphaTA/` with a
   `Dockerfile` and `build.sh` that runs `cargo +nightly fuzz build`.
3. Update the `alpha-ta-core` repository to expose the fuzz workspace as
   the OSS-Fuzz source.

A minimal `build.sh` for OSS-Fuzz:

```bash
#!/usr/bin/env bash
set -ex
cd "$SRC/alpha-ta-core"
cargo +nightly fuzz build --fuzz-dir fuzz
cp fuzz/target/release/fuzz_target_* "$OUT/"
```

## Reproducing a crash

```bash
cargo +nightly fuzz run formula fuzz/artifacts/formula/crash-deadbeef
```

The crash artefact contains the seed bytes that triggered the issue.
Attach it to the issue tracker; the reproducer will run deterministically
with the same binary.

## Sanitizer builds

The harness defaults to using `cargo-fuzz`'s built-in AddressSanitizer +
UndefinedBehaviorSanitizer. To explicitly enable:

```bash
RUSTFLAGS="--cfg fuzzing" cargo +nightly fuzz run formula
```

[cargo-fuzz]: https://github.com/rust-fuzz/cargo-fuzz
[oss-fuzz]: https://google.github.io/oss-fuzz/
