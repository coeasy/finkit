# Development Guide

This guide explains how to contribute to Finkit, build from source, and develop new features.

## Getting Started

### Prerequisites

- **Rust**: 1.85+ (workspace MSRV; install via [rustup](https://rustup.rs/))
- **Git**: For version control
- **Python**: 3.8+ (for Python binding development)
- **Node.js**: 16+ (for Node.js binding development)
- **Java**: JDK 11+ (for Java binding development)
- **Go**: 1.21+ (for Go binding development)
- **.NET**: SDK 8.0+ (for .NET binding development)

### Clone and Setup

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
```

## Project Structure

```
Finkit/
├── core/                       # Core Rust library
│   ├── src/
│   │   ├── lib.rs              # Library entry point
│   │   ├── error.rs            # Error types
│   │   ├── utils.rs            # Utility functions
│   │   ├── indicators/         # Technical indicators
│   │   │   ├── mod.rs
│   │   │   ├── overlap.rs      # SMA, EMA, BBANDS, SAR, etc.
│   │   │   ├── momentum.rs     # RSI, MACD, STOCH, ADX, etc.
│   │   │   ├── volume.rs       # OBV, AD, ADOSC, CMF
│   │   │   ├── volatility.rs   # ATR, NATR, TRANGE
│   │   │   ├── cycle.rs        # HT_DCPERIOD, HT_SINE, etc.
│   │   │   ├── price_transform.rs  # AVGPRICE, TYPPRICE, etc.
│   │   │   └── statistics.rs   # STDDEV, VAR, LINEARREG, ZSCORE
│   │   ├── math/               # Mathematical foundation
│   │   │   ├── mod.rs
│   │   │   ├── moving_avg.rs   # SMA, EMA, WMA, DEMA, TEMA, KAMA
│   │   │   ├── linear.rs       # Linear regression
│   │   │   └── statistics.rs   # Mean, variance, correlation
│   │   └── patterns/           # Pattern recognition
│   │       ├── mod.rs
│   │       ├── candlestick.rs  # 60+ candlestick patterns
│   │       └── chart.rs        # 15+ chart patterns
│   └── Cargo.toml
├── ffi/                        # FFI bindings
│   ├── c-binding/              # C FFI (base layer)
│   ├── python-binding/         # Python (PyO3)
│   ├── node-binding/           # Node.js (NAPI-RS)
│   ├── java-binding/           # Java (JNI)
│   ├── go-binding/             # Go (CGO)
│   └── dotnet-binding/         # .NET (P/Invoke)
├── cli/                        # Command-line interface
├── wasm/                       # WebAssembly module
├── visualization/              # Visualization module
├── .github/workflows/          # CI/CD pipelines
└── docs/                       # Documentation
```

## Building

### Core Library

```bash
# Build
cargo build --release

# Build with all features
cargo build --release --all-features

# Run tests
cargo test

# Run tests for specific crate
cargo test -p finkit

# Run benchmarks
cargo bench
```

### Python Binding

```bash
cd ffi/python-binding

# Development mode (installs in current Python environment)
maturin develop --release

# Build wheel
maturin build --release --out dist

# Run tests
pip install pytest numpy
pytest
```

### Node.js Binding

```bash
cd ffi/node-binding

# Install dependencies
npm install

# Build native module
npm run build

# Build debug version
npm run build:debug

# Run tests
npm test
```

### Java Binding

```bash
cd ffi/java-binding

# Build Rust library
cargo build --release

# Build Java artifacts
mvn clean install -DskipTests

# Run tests
mvn test
```

### Go Binding

```bash
cd ffi/go-binding

# Build native library
make build

# Run tests
go test -v ./...
```

### .NET Binding

```bash
cd ffi/dotnet-binding

# Build Rust library
cargo build --release

# Build .NET library
cd src/Finkit
dotnet build --configuration Release

# Run tests
cd ../Finkit.Tests
dotnet test
```

### WebAssembly

```bash
cd wasm

# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web --out-dir pkg

# Build for Node.js
wasm-pack build --target nodejs --out-dir pkg
```

### CLI

```bash
cd cli

# Build
cargo build --release

# Install locally
cargo install --path .
```

## Code Style

### Rust

Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- Use `cargo fmt` to format code:
  ```bash
  cargo fmt --all
  ```

- Run clippy for linting:
  ```bash
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

- Write documentation comments for all public items:
  ```rust
  /// Calculate the Simple Moving Average.
  ///
  /// # Arguments
  ///
  /// * `data` - Input data array
  /// * `period` - Number of periods for the moving average
  ///
  /// # Returns
  ///
  /// Array of SMA values, same length as input
  ///
  /// # Errors
  ///
  /// Returns `TaError::InvalidPeriod` if period is 0
  /// Returns `TaError::InsufficientData` if data length < period
  pub fn sma(data: &[f64], period: usize) -> Result<Vec<f64>> {
      // Implementation
  }
  ```

### Python

Follow [PEP 8](https://peps.python.org/pep-0008/):

- Use type hints
- Write docstrings in Google style
- Format with black:
  ```bash
  pip install black
  black ffi/python-binding/
  ```

### Node.js

Follow [Standard Style](https://standardjs.com/):

- Use TypeScript
- Format with Prettier
- Run ESLint

## Adding New Indicators

### 1. Implement in Core

Create a new file or add to existing module in `core/src/indicators/`:

```rust
// core/src/indicators/my_indicator.rs

use crate::error::{TaError, Result};
use crate::utils::validate_period;

/// Calculate my custom indicator.
///
/// # Arguments
///
/// * `data` - Input data array
/// * `period` - Lookback period
pub fn my_indicator(data: &[f64], period: usize) -> Result<Vec<f64>> {
    validate_period(period)?;
    validate_data_length(data, period)?;

    let mut result = vec![f64::NAN; data.len()];

    for i in period - 1..data.len() {
        // Calculation logic
        let sum: f64 = data[i - period + 1..=i].iter().sum();
        result[i] = sum / period as f64;
    }

    Ok(result)
}

fn validate_data_length(data: &[f64], period: usize) -> Result<()> {
    if data.len() < period {
        return Err(TaError::InsufficientData {
            needed: period,
            actual: data.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_my_indicator_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = my_indicator(&data, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0);
        assert_relative_eq!(result[3], 3.0);
        assert_relative_eq!(result[4], 4.0);
    }

    #[test]
    fn test_my_indicator_insufficient_data() {
        let data = vec![1.0, 2.0];
        let result = my_indicator(&data, 5);
        assert!(result.is_err());
    }
}
```

### 2. Export from Module

Add to `core/src/indicators/mod.rs`:

```rust
mod my_indicator;
pub use my_indicator::my_indicator;
```

### 3. Add to Python Binding

Add to `ffi/python-binding/src/lib.rs`:

```rust
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn my_indicator(close: &PyArray1<f64>, timeperiod: usize) -> PyResult<Py<PyArray1<f64>>> {
    let data = close.readonly().as_slice().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Failed to read array")
    })?;

    let result = indicators::my_indicator(data, timeperiod).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
    })?;

    Python::with_gil(|py| {
        Ok(PyArray1::from_vec(py, result).to_owned())
    })
}
```

Register the function in the module:

```rust
#[pymodule]
fn finkit(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(my_indicator, m)?)?;
    Ok(())
}
```

### 4. Add to Node.js Binding

Add to `ffi/node-binding/src/lib.rs`:

```rust
#[napi]
pub fn my_indicator(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::my_indicator(&close, timeperiod as usize).map_err(|e| {
        Error::new(Status::InvalidArg, e.to_string())
    })
}
```

### 5. Add Tests

Add integration tests for each binding.

## Adding New Candlestick Patterns

### 1. Implement Pattern

Add to `core/src/patterns/candlestick.rs`:

```rust
/// Detect custom pattern.
///
/// Returns:
/// - 100 for bullish signal
/// - -100 for bearish signal
/// - 0 for no pattern
pub fn custom_pattern(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<Vec<i32>> {
    let len = open.len();
    let mut result = vec![0; len];

    for i in 1..len {
        // Pattern detection logic
        if is_bullish_pattern(open, high, low, close, i) {
            result[i] = 100;
        } else if is_bearish_pattern(open, high, low, close, i) {
            result[i] = -100;
        }
    }

    Ok(result)
}
```

### 2. Add to All Bindings

Follow the same pattern as indicators above.

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test my_indicator

# Run tests for specific crate
cargo test -p finkit
```

### Integration Tests

```bash
# Python
cd ffi/python-binding
pytest

# Node.js
cd ffi/node-binding
npm test

# Java
cd ffi/java-binding
mvn test

# Go
cd ffi/go-binding
go test -v ./...

# .NET
cd ffi/dotnet-binding/src/Finkit.Tests
dotnet test
```

### Code Coverage

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --package finkit --all-features --html

# Generate LCOV format for CI
cargo llvm-cov --package finkit --all-features --lcov --output-path lcov.info
```

### Benchmarks

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench -- my_indicator

# Compare with baseline
cargo bench -- --baseline
```

## Performance Optimization

### 1. Use SIMD

```rust
#[cfg(target_arch = "x86_64")]
pub fn sma_simd(data: &[f64], period: usize) -> Result<Vec<f64>> {
    // Use x86_64 SIMD instructions
}
```

### 2. Use Rayon for Parallel Computation

```rust
use rayon::prelude::*;

pub fn sma_parallel(data: &[f64], period: usize) -> Result<Vec<f64>> {
    let mut result = vec![f64::NAN; data.len()];

    (period - 1..data.len())
        .into_par_iter()
        .for_each(|i| {
            let sum: f64 = data[i - period + 1..=i].iter().sum();
            result[i] = sum / period as f64;
        });

    Ok(result)
}
```

### 3. Pre-allocate Arrays

```rust
// Good: Pre-allocate
let mut result = vec![0.0; data.len()];

// Bad: Push in loop
let mut result = Vec::new();
for i in 0..data.len() {
    result.push(calculate(data[i]));
}
```

### 4. Avoid Unnecessary Allocations

```rust
// Good: Reuse buffer
pub fn calculate_in_place(data: &mut [f64]) {
    for item in data.iter_mut() {
        *item = transform(*item);
    }
}

// Bad: Create new array
pub fn calculate(data: &[f64]) -> Vec<f64> {
    data.iter().map(|&x| transform(x)).collect()
}
```

## CI/CD

### GitHub Actions

The project uses GitHub Actions for CI/CD:

- **CI** (`.github/workflows/ci.yml`):
  - Format check (rustfmt)
  - Lint check (clippy)
  - Core tests (Linux, macOS, Windows)
  - Python binding tests
  - Node.js binding tests
  - Go binding tests
  - .NET binding tests
  - Java binding tests
  - WASM tests
  - CLI tests
  - Cross-platform compilation
  - Security audit
  - Code coverage (Codecov)

- **Release All** (`.github/workflows/release-all.yml`):
  - Version validation
  - Changelog generation
  - Trigger language-specific releases
  - Create GitHub Release

- **Release Python** (`.github/workflows/release-python.yml`):
  - Build wheels for all platforms
  - Test wheels
  - Publish to PyPI

- **Release Node.js** (`.github/workflows/release-node.yml`):
  - Build native modules
  - Publish to npm

### Local CI Testing

```bash
# Run format check
cargo fmt --all -- --check

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run tests
cargo test --workspace

# Run Python tests
cd ffi/python-binding
maturin develop
pytest
```

## Release Process

### 1. Update Versions

Update version in all `Cargo.toml` files:

```bash
# Update workspace version in root Cargo.toml
# Update individual package versions in ffi/*/Cargo.toml
# Update pyproject.toml version
# Update package.json version
```

### 2. Update CHANGELOG.md

Follow [Keep a Changelog](https://keepachangelog.com/) format.

### 3. Create Release Commit

```bash
git add .
git commit -m "chore: release v0.x.x"
git tag -a v0.x.x -m "Release v0.x.x"
git push origin main --tags
```

### 4. GitHub Actions

GitHub Actions will automatically:
- Build and publish to PyPI
- Build and publish to npm
- Build and publish to crates.io
- Create GitHub Release

## Documentation

### Rust Docs

```bash
# Generate documentation
cargo doc --no-deps --open

# Generate for specific crate
cargo doc --no-deps --package finkit --open
```

### API Documentation

Update `docs/api-reference.md` when adding new functions.

### Indicator Documentation

Update `docs/indicators.md` when adding new indicators.

## Troubleshooting

### Common Issues

**Build fails with linker errors:**
```bash
# Install required build tools
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools
```

**Python binding import error:**
```bash
# Rebuild in development mode
cd ffi/python-binding
maturin develop --release
```

**Node.js binding not found:**
```bash
# Rebuild native module
cd ffi/node-binding
npm run build
```

**Go binding CGO error:**
```bash
# Ensure CGO is enabled
export CGO_ENABLED=1

# Verify C compiler is available
gcc --version
```

### Getting Help

- [GitHub Issues](https://github.com/coeasy/finkit/issues)
- [GitHub Discussions](https://github.com/coeasy/finkit/discussions)
- [Contributing Guidelines](../CONTRIBUTING.md)
