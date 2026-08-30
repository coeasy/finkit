# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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


