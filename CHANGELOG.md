# Changelog

## [0.2.0] - 2026-09-23

Trial release. This cycle closes the gap between "the engine can compute it"
and "a user can find out that it can": the formula surface is now described by
a machine-checkable contract, and the factor libraries are defined once and
reused.

### Added

- The 31 TA-Lib 0.7/0.8 indicator functions that were numeric-green but
  unreachable from the formula engine: `AC`, `ACCBANDS`, `ADR`, `AO`, `AROON`,
  `CMOU`, `COPPOCK`, `CVI`, `EFI`, `ER`, `ERI`, `FOSC`, `FRACTAL`, `HA`, `KC`,
  `MAMA`, `MARKETFI`, `MASSI`, `NVI`, `PERCENTRANK`, `PVI`, `PVO`, `PVT`,
  `QSTICK`, `RVI`, `RVOL`, `SMI`, `VHF`, `VORTEX`, `WAD`, `ZLEMA`, together with
  their multi-output registrations.
- `tests/contracts/formula_dialect_coverage_v1.json`, a per-function coverage
  contract for 通达信 / 同花顺 / 大智慧 / TradingView Pine. Each row carries
  three independent facts: `status` (usable out of the box?), `registered` (a
  machine-derived fact about the live function table) and `provenance`
  (substantiated by a named in-repo file, or merely attributed to a vendor
  list). Each terminal also records which rows a checked-in corpus actually
  calls, recomputed by the gate from the corpus files.
- 大智慧 as the sixth `FormulaTerminal` (`dzh` / `dazhihui` / `大智慧`),
  routing the 同花顺 common subset.
- Alpha158 (158 factors) and WorldQuant101 (17 computable, the rest annotated
  with the reason they are not) as declarative factor graphs, plus a one-line
  `finkit.factor_library("alpha158")` entry point in Rust and Python.
- Pine user-defined function blocks, tuple destructuring, and plot visual
  metadata.

### Changed

- `FormulaExecutionMode` is now a real execution switch. The default is still
  the tree interpreter; the compiled plan path is selected explicitly and is
  covered by a differential gate against the interpreter.
- The Pine canonical-name mapping is derived from the engine (the builtin
  table plus the `ast_mapper` special cases plus the `TA_<NAME>` fallback) and
  recorded in the coverage contract, instead of being mirrored by hand in the
  generator.
- The release version gate now covers `docs/getting-started.md`,
  `docs/cli.md`, `docs/language-bindings.md` and `docs/development.md`, and it
  matches any `MAJOR.MINOR.PATCH` series rather than only `0.1.x`. Those four
  documents had drifted to `v0.1.5` while the workspace was at `0.1.15`, and
  the old pattern could not have caught it.
- Removed the unreferenced `formula::pine::runtime` module (`PineRuntime`,
  `PineRuntimeError`, `SeriesValue` and its plot/security/barstate support
  types). It was introduced in the initial commit and never had a caller
  anywhere in the workspace, including the language bindings and the test
  suite. It was a second, parallel Pine evaluator for semantics the mapper plus
  the AlphaTA engine already own, so it could only ever drift from them.

### Fixed

- Pine `na(x)` was silently mis-parsed. `na` is a grammar keyword, so it could
  not reach the call rule, and `na(close)` parsed *without error* as an `na`
  literal followed by a discarded `(close)` expression statement — so
  `is_missing = na(close)` evaluated to `NaN` instead of `ISNA(CLOSE)`. The
  grammar now has an explicit `na_call` rule.
- The Pine coverage figure was a false green. The generator's hand-written
  canonical-name mirror disagreed with the engine on 7 names, left 48 spellings
  unmapped, and credited 43 unusable names as out-of-the-box, reporting 97.8%
  coverage. Deriving the mapping from the engine drops it to the real 81.7%
  (263 rows, 47 `unsupported`).
- Formula and plan kernel correctness for `IF`, `STOCHF`, `SAR`, `BOLLUP` /
  `BOLLMID` / `BOLLDN`, `DEA`, `WILLR`, `PLUS_DI` / `MINUS_DI` / `ADX`, `TRIX`,
  `STDDEV`, `CROSS` / `FIXNAN` / `VAR`, and the money-flow / `TR` host kernels.
- External variables bound through `HostContext` are now visible on the
  bytecode and JIT paths, which previously resolved only their own private
  tables.
- `MACDFIX` used MACD's `2/(period+1)` EMA constants instead of TA-Lib's fixed
  `0.15` / `0.075`, so `MACDFIX(CLOSE, 9)` on the formula path disagreed with
  `indicators::macdfix` on the very same input. Every differential path agreed
  with every other path — on the wrong constants — so only an absolute
  comparison against the reference implementation could see it. The formula
  function now delegates to the parity implementation.
- The MACD family (`MACDFIX`, `MACDEXT`, `DIFF`, `DEA`) treated a leading
  warm-up run as bad data instead of skipping it, so a composed input such as
  `MACDFIX(MA(CLOSE, 5), 9)` collapsed to an all-NaN series. The plan path's
  `MACD` and `DEA` kernels carried the same gap independently of the tree path,
  which the corpus never exercised.
- `MACDEXT`'s three `matype` arguments were read with the period extractor,
  which rejects `0` outright. The canonical TA-Lib spelling
  `MACDEXT(close, 12, 0, 26, 0, 9, 0)` therefore failed with
  "period must be > 0", and the `0 => SMA` match arm behind it was unreachable:
  only the `EMA` selector ever worked.
- Pine `barstate.*` mapped to a variable name that no execution path resolved
  (`barstate_islast` and friends), so any script reading `barstate` parsed,
  mapped, and then failed with "Unknown variable". The fields are now
  translated in the mapper into `BARPOS` / `BARSCOUNT` comparisons, which keeps
  the decision inside the frontend and out of the five numeric backends.
- Pine's `hl2`, `hlc3`, `ohlc4` and `time` had the same defect: they were
  renamed to `HL2` / `HLC3` / `OHLC4` / `DATE` and handed to the engine as
  variable references, and none of those names resolves. The derived price
  sources are now expanded into OPEN/HIGH/LOW/CLOSE arithmetic (identical to
  `streaming::PriceSource`) and `time` is emitted as a call to the
  zero-argument `DATE()` builtin. The Pine corpus runner had been pre-binding
  `HL2` / `HLC3` / `OHLC4` into the context, which is exactly why a green
  corpus gate never noticed; that workaround is removed.
- SIMD documentation no longer claims acceleration that does not exist. The
  Hilbert section in `indicators::cycle` said the pipeline was AVX2-accelerated
  by the `simd_ht_*` kernels and that `ht_sine`'s terminal stage batched through
  `simd_sin_cos`; in fact the production Hilbert chain is a scalar TA-Lib-faithful
  state machine and `simd_sin_cos` is only reached from a unit test. Likewise
  `simd_ht_dcphase` documented an AVX2 path and radian output that its body does
  not provide (it is a scalar, degree-returning approximation), and
  `simd_aroon` / `simd_kama` / `simd_ema_next` / `simd_mama_hilbert` /
  `simd_sar_step` / `simd_atr` documented AVX2 batching that their scalar bodies
  do not perform. The module docs for `math::simd_ops` and `math::simd_ops_avx512`
  now state the real dispatch policy (AVX-512 falls back to scalar for every
  kernel except `simd512_sma`) and the `avx512` coverage table no longer lists
  non-existent `simd512_macd` / `simd512_bbands` / `simd512_atr` / `simd512_adx`
  names (they are `*_seed`). No numeric behaviour changed — these are
  documentation↔reality corrections.

- Streaming `OBV` diverged from the batch `obv` / `simd_obv` contract on flat
  bars. It accumulated `diff.signum() * volume`, and `f64::signum(0.0)` is
  `+1.0`, so a bar with an unchanged close wrongly added `volume` instead of
  leaving OBV unchanged (the comment even claimed the opposite). The streaming
  step is now an explicit three-way `> prev` / `< prev` / flat, matching the AVX2
  and scalar batch kernels. A regression gate pins the two implementations
  together on shared bars including a flat one.

- The formula streaming path (`FormulaStatefulStream`) was a differential
  blind spot: `check_all_paths` never instantiates it, so two of its stateful
  indicators silently disagreed with the batch formula table.
  1. `SMA(X, N[, M])` was implemented as a simple moving average (warm-up `NaN`
     prefix), while the batch `fn_sma` is recursive smoothing seeded at the
     first value (`value = (M*cur + (N - M)*prev) / N`). The streaming path now
     has its own `StreamingSmaSmoothed` and routes `SMA` (distinct from `MA`) to
     it in both `compile_state` and `compile_expression_stateful_function`, with
     the optional third weight argument honoured.
  2. `MACD(...)` streaming returned the histogram (`macd - signal`) while the
     batch `MACD` formula returns the DIF line (`fast_ema - slow_ema`); the two
     are different series. The streaming `MACD` now returns `value.macd`.
  A new gate `streaming_stateful_matches_batch` compares MA / SMA / EMA / WMA /
  RSI / HHV / LLV / SUM / REF / STD / VAR / CROSS / CROSSBELOW / ATR / MACD
  (direct and composed) against the batch (AST) path on oscillating data,
  closing the blind spot.

## [0.1.15] - 2026-09-11

### Added

- Forming-bar repaint rollback for streaming MACDEXT.
- Scalar WMA, DEMA, TEMA, KAMA, T3, TRIMA, HMA, ALMA, and VIDYA variants for
  streaming MACDEXT fast, slow, and signal lines.
- Configurable streaming MACDEXT exposed through the Python binding.

### Changed

- Close-stream parsing is now injectable, which makes CLI tests deterministic.

## [0.1.14] - 2026-09-11

- Add registered, parameterized custom formula components with nested
  expansion, built-in shadowing protection and bounded expansion limits.
- Expose the same custom formula registry through the Python binding.
- Add explicit custom-edge PSI calculation with finite-value filtering and
  stable outlier handling.
- Add a shared crosshair data-window snapshot and visible-range-aware cursor
  mapping for native/WASM chart frontends.

## [0.1.13] - 2026-09-11

- Correct the Rust CDLDOJI public wrapper to use TA-Lib's default 0.1
  BodyDoji factor, matching the Python binding and compatibility contract.

## [0.1.12] - 2026-09-11

- Complete the maintained TA-Lib 0.6.x compatibility matrix at 161/161 on
  multiple long-input lengths and seeds, including SAR/SAREXT bootstrap
  semantics, T3 warm-up/coefficient behavior and extended candlestick rules.
- Add shared CandleSettings-based thresholds for candlestick compatibility,
  while preserving native full-length and short-input API behavior.
- Correct streaming T3 coefficients and align cycle/trendline compatibility
  paths with the batch implementation.

## [0.1.11] - 2026-09-11

- Add exact TA-Lib compatibility implementations for STOCHRSI, MACDFIX and
  BETA, including their multi-output, lookback and secondary-input contracts.
- Correct the shared Hilbert recursive warm-up and radian/degree period
  conversion; HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, MAMA and the
  trend-mode path now match TA-Lib on the maintained comparison matrix.
- Add the TA-Lib dominant-cycle phase projection and raw-price trendline
  calculation, plus regression coverage for the extended compatibility paths.
- Align CDLDOJI with TA-Lib's preceding real-body average candle setting.

## [0.1.10] - 2026-09-11

- Correct TA-Lib MACDEXT lookback selection for its `slowperiod` and
  `signalperiod` parameter positions in the compatibility adapter.

## [0.1.9] - 2026-09-11

- Add opt-in Python `compute_indicators(..., talib_compat=True)` semantics for
  TA-Lib lookback/NaN conventions, absolute MAX/MININDEX results and compatible
  multi-output ordering without changing native formula behavior.
- Align ADXR, AROONOSC, SAR, PLUS_DM/MINUS_DM and PPO compatibility paths with
  TA-Lib's window, smoothing, output and moving-average-type contracts.
- Correct the full Python comparison harness for TA-Lib 0.6.x multi-output
  finite-ratio accounting and explicit parameter signatures; the maintained
  smoke matrix now calls all 161 public functions and reports 124 exact rows.
- Add Python regression coverage for native-vs-TA-Lib compatibility semantics.

## [0.1.8] - 2026-09-11

- Add a shared formula result metadata contract for dtype, output names,
  NaN/null policy, lookback, warm-up and valid-start semantics across Rust,
  Python, Node and WASM.
- Add a complete 161-function public TA-Lib catalog with explicit runtime
  coverage, category, output-count and compatibility-report fields; unsupported
  and host-required functions are no longer ambiguous.
- Add O(1) append/eval-last formula paths for direct MA, RSI and formula-SMA
  ATR, with exact fallback on mutation or discontinuity.
- Add binding APIs for formula metadata and TA-Lib catalog discovery.
- Preserve separate Wilder/RMA streaming ATR semantics from the formula-layer
  rolling-SMA true-range contract, with differential regression coverage.

## [0.1.7] - 2026-09-11

- Add formula static analysis for dependencies, lookback, future-data risks,
  side effects, stateful nodes and streaming suitability.
- Add terminal semantic profiles and function-level compatibility reports for
  Finkit, TongDaXin, TongHuaShun, EastMoney and Pine.
- Add borrowed range evaluation and Python `eval_range_zero_copy` for chart
  window refreshes without copying complete OHLCV history.
- Add conservative O(1) EMA updates for continuous append/eval-last streams,
  with exact fallback when continuity cannot be proven.
- Expose formula analysis and compatibility APIs through Python, Node and WASM.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-09-09

### Added

- Cross-market screening indicators for crossover, breakout, volume expansion,
  moving-average alignment, relative strength, gaps, and trend confirmation.
- Formula DSL routes and aliases for the new screening indicators.
- Versioned multi-language release workflow documentation and package build
  contracts.

### Performance and validation

- Expanded the TA-Lib release-wheel gate to 155 compatible functions and 310
  observations at 100K/1M values.
- v0.1.5 local validation: 307/310 observations faster, geometric mean 2.03x,
  zero runtime errors; the two existing `HT_TRENDMODE` mask exceptions remain
  explicitly documented.

### Documentation

- Added the cross-market screening API and selection recipes.
- Updated generated indicator/formula/version catalogs and the multi-language
  installation/release guides.
- Replaced stale six-indicator benchmark claims with the current full-matrix
  report and per-observation caveats.

## [0.1.2] - Pending release (source baseline 2026-09-02)

- Added cross-language version checks for .NET and Java binding metadata, with SSOT version-matrix coverage.
- Connected the remaining Android, Go, and .NET generated indicator/chart entrypoints and hardened pre-epoch formula timestamp conversion with regression coverage.

> The `0.1.2` source metadata is ready on the repair branch in PR #13; the GitHub Release and downloadable assets are still pending until CI verification and merge.

### Changed
- 将公式运行时升级为可复用的零拷贝/增量执行模型：公共子表达式合并、`eval_range`/`eval_last`、容量增长式 `append_bar`、持久化 Bytecode/JIT 与缓冲池。
- Python ABI3 wheel 发布矩阵与版本元数据统一为 `0.1.2`，覆盖 CPython 3.8–3.14 的 Linux x86_64、macOS x86_64/arm64 和 Windows x86_64。

### Fixed
- 加固 GitHub Actions 的权限与并发控制，移除旧版 `v0.1.0` 强制回写逻辑，统一使用版本 tag 发布 wheel。

### Added
- Comprehensive CI/CD pipeline with fmt, clippy, and security audits
- Multi-language binding tests (Python, Node.js, Go, .NET, Java, WASM, CLI)
- Complete documentation suite (indicators, installation, API reference, development)
- Cross-platform compilation support (Linux, macOS, Windows)
- Dependency review for pull requests
- **`core/tests/golden_regression.rs`**：30 个黄金 CSV 回归断言（容差 1e-9），覆盖 SMA/EMA/RSI/MACD/ATR/STOCH/ADX/BBANDS/MOM/ROC 等核心指标
- **`core/tests/property_smoke.rs`**：3 个 proptest 属性测试 — SMA 线性、Bollinger 上下界包络、RSI 范围 [0,100]
- **`core/benches/zero_alloc_bench.rs`**：SIMD 路径 `n=10_000` 零分配验证（`fma_avx2` vs `scalar_fma`）
- **`streaming::registry` 额外缓存**：`by_category()` / `by_id()` 改 `OnceLock<HashMap<...>>` 缓存，与 `all_indicators` 保持一致
- **`core/benches/watchlist_self_bench.rs`**：9 个 watchlist 指标（AROON/WILLR/WMA/KAMA/MFI/STOCHF/AD/ADOSC/OBV）的自基准测试，3 档数据规模（1K/10K/100K）
- **`watchlist_self_bench` 注册到 `Cargo.toml`**：纳入 cargo bench 体系

### Changed
- **Crate-wide `#[allow(missing_docs)]`**：在 `core` / `visualization` / `cli` / 5 个 FFI binding / `wasm` 的 `lib.rs` 顶部集中抑制 internal helper 的 missing_docs warning（净减 1700+ pedantic warnings，cargo check 0 warnings）
- **`StreamingPpo` / `StreamingTsi` / `StreamingApo` / `StreamingCoppock` / `StreamingNatr`**：移出 `wasm_streaming_f64!` 宏，补独立 wrapper（PPO/TSI/APO 2 参构造、Coppock 3 参构造、NATR `&dyn Ohlcv` 入参）
- **`StreamingBoll` / `StreamingStoch` 构造签名**：wasm 入口补第 3 参数（`nb_dev_dn` / `k_slow`），与 core 对齐
- **`aroon` 阈值分流**：`period ≤ 16` 走 `aroon_scan`（线性扫描，cache 友好），`period > 16` 走 `aroon_with_deques`（单调双端队列，O(1) amortized）。消除原 O(period) rescan 慢路径
- **`willr` 单调双端队列**：完全重写为 max-deque + min-deque，热路径 O(1) amortized，删除 rescan 内层循环
- **`stochf` 单调双端队列**：max-deque 替代 rescan；保留 ring-buffered %D 累加器，输出流水线不变
- **`docs/BENCHMARK_REPORT.md` watchlist 章节**：补充 9 指标吞吐量表 + 算法形态分类 + 下一步 SIMD 优化建议

### Fixed
- **dotnet-binding & java-binding 内存管理 README 文档契约** 已在 `ffi/dotnet-binding/README.md` 与 `ffi/java-binding/README.md` 完善
- **`simd_kernels.rs` 两处死赋值**：`highest = *high_ptr.add(ws);` 与 `lowest = *low_ptr.add(ws);` 立即被覆盖，删除
- **`pca.rs::cov_idx` 死方法**：未引用，删除
- **dotnet-binding & java-binding CString UB**：为两者增加 `ta_free_cstring` / `freeJString` 配对释放函数；引入 `serde_json` 替代 36+ 处手写 JSON 序列化（DrawCommand / DebugEvent / FormulaTemplate 等核心类型现在 derive `serde::Serialize`）
- **FormulaCache 真 LRU**：从 O(n) 最小 counter 扫描改为 `lru` crate 的 O(1) LruCache，公开 API 完全兼容
- **JIT 路径 `load_variable` 零分配**：从 `name.to_uppercase().as_str()` 链式匹配改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **SIMD feature detection 缓存**：用 `std::sync::OnceLock<SimdLevel>` 缓存 CPUID 结果，18 个 public SIMD 入口改为 `match simd_level()`，100 万次 add 仅触发 1 次 CPUID
- **BufferPool 双池清理**：删除未使用的 legacy `VecDeque<Vec<f64>>` 字段与 8 个 API（`acquire/release/shrink_to/...`），构造 BufferPool 不再预分配 64KB `Vec<f64>`
- **`resolve_variable_zero_copy` 6 段重复代码抽取**：Close/High/Low/Open/Volume/Amount 6 段近似重复逻辑抽取为单一 `copy_view_to_pool` helper
- **Parser 别名表零分配**：`parse_variable` 中 16 个 C1/O1/CLOSE1 等别名从 `to_uppercase().as_str()` 改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **streaming `all_indicators` OnceLock 缓存**：避免重复调用时重复构造
- **FormulaCache 单次查找**：`get_cloned` 与 `insert` 中删除 `contains_key + get/get_mut` 双查找
- **workspace 构建配置精细化**：`[profile.release]` 改为 `lto="thin", strip="debuginfo"`；`[profile.release.package."alpha-ta-..."] codegen-units = 16`；`[profile.dev] opt-level = 1, debug = 1`；新增 workspace lints（`unsafe_op_in_unsafe_fn = "warn"`, `missing_debug_implementations = "warn"`, `clippy::pedantic`）
- **Go 绑定版本硬编码 bug**：`ta_version()` 从硬编码 `0.1.0` 改为 `env!("CARGO_PKG_VERSION")`
- **Java 绑定 panic 保护**：为公式评估函数添加 `catch_unwind` 包裹和 `RuntimeException` 错误传播

## [1.0.0] - 2026-06-24

### Added
- Hilbert Transform cycle indicators (HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDMODE)
- Statistics indicators (STDDEV, VAR, LINEARREG, ZSCORE, CORREL)
- Price transform functions (AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE)
- Volume indicators (AD, ADOSC, CMF)
- Pattern recognition for 60+ candlestick patterns
- Chart pattern detection (Double Top/Bottom, Head & Shoulders)
- Go binding with CGO support
- .NET binding with P/Invoke
- Java binding with JNI support
- CLI tool for command-line analysis
- WebAssembly module for browser usage
- Visualization module
- Formula engine with AST parsing, bytecode compilation, JIT optimization, and SIMD acceleration
- Streaming (incremental) indicator framework with 150+ indicators
- Feature engineering module for ML label generation

### Changed
- Improved performance of SMA and EMA calculations
- Enhanced error handling with detailed error messages
- Updated Python binding to use latest PyO3 features
- Optimized memory allocation in core library

### Fixed
- Fixed RSI calculation for edge cases with constant values
- Fixed MACD signal line initialization
- Fixed pattern detection boundary conditions

## [0.3.0] - 2024-XX-XX

### Added
- Python binding with PyO3
- Node.js binding with NAPI-RS
- Support for all major overlap indicators (SMA, EMA, WMA, DEMA, TEMA, KAMA, BBANDS, SAR)
- Momentum indicators (RSI, MACD, STOCH, ADX, AROON, CCI, WILLR)
- Volatility indicators (ATR, NATR, TRANGE)
- Basic candlestick pattern recognition

### Changed
- Refactored moving average calculations for better performance
- Improved test coverage for all indicators

### Fixed
- Fixed EMA calculation precision issues
- Fixed Bollinger Bands upper/lower band calculation

## [0.2.0] - 2024-XX-XX

### Added
- Initial release of FTA core library
- Basic technical analysis framework
- Moving average implementations (SMA, EMA)
- Core mathematical functions
- Rust API design
- Multi-platform support (Linux, macOS, Windows)
- CI/CD pipeline setup
- Comprehensive test suite

