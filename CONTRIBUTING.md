# Contributing to AlphaTA

Thank you for your interest in contributing to AlphaTA! This document provides guidelines and information for contributors.

## Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Code Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code
- Run `cargo clippy` and fix all warnings (first time: `rustup component add clippy` — clippy is not bundled with the stable toolchain by default; or use `make lint` which also runs `cargo fmt --check`)
- Write comprehensive documentation comments
- Include unit tests for all new functions

## Adding New Indicators

When adding a new indicator:

1. Implement in `core/src/indicators/` or appropriate module
2. Follow existing patterns and naming conventions
3. Add comprehensive tests with at least:
   - Normal input test
   - Edge cases (empty input, single element, insufficient data)
   - Validation tests (invalid parameters)
4. Add to FFI bindings (Python, Node.js, Java, WASM)
5. Update documentation

## Adding New Candlestick Patterns

1. Implement in `core/src/patterns/candlestick.rs`
2. Return `PatternResult` (Array1<i32>)
3. Values: 100 for bullish, -100 for bearish, 0 for no pattern
4. Include minimum required bars validation
5. Add to Python/Node/Java bindings

## Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p finkit

# Run benchmarks
cargo bench

# Test with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## FFI Bindings

### Python (PyO3)

```bash
cd ffi/python-binding
maturin develop
pytest
```

### Node.js (NAPI-RS)

```bash
cd ffi/node-binding
npm install
npm run build
npm test
```

### Java (JNI)

```bash
cd ffi/java-binding
cargo build --release
# Test with Java test suite
```

### WebAssembly

```bash
cd wasm
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

## Performance Optimization

When optimizing performance:

1. Write benchmarks first
2. Profile to identify bottlenecks
3. Consider SIMD optimization
4. Use rayon for parallel computation
5. Benchmark before and after changes
6. Document performance improvements

## Release Process

1. Update version in `Cargo.toml` files
2. Update CHANGELOG.md
3. Create release commit
4. Create git tag
5. GitHub Actions will automatically publish to:
   - crates.io
   - PyPI
   - npm
   - Maven Central
6. Create GitHub Release with notes

## Code of Conduct

Please be respectful and constructive in all interactions.

## Questions?

- Open an issue for bugs or feature requests
- Join discussions in GitHub Discussions
- Contact maintainers directly for security issues
