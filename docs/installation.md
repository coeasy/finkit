# Installation Guide

This document provides detailed installation instructions for each language binding of FTA.

## Prerequisites

### Common Requirements

- **Rust toolchain**: Required for building from source
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Git**: For cloning the repository
  ```bash
  git clone https://github.com/coeasy/finkit.git
cd alpha-ta
  ```

## Python

### Option 1: pip (Recommended)

```bash
pip install alpha-ta
```

### Option 2: Build from Source

```bash
# Install maturin
pip install maturin

# Navigate to Python binding directory
cd ffi/python-binding

# Build and install in development mode
maturin develop --release

# Or build wheel
maturin build --release --out dist
pip install dist/alpha-ta-*.whl
```

### Option 3: Conda

```bash
conda install -c conda-forge alpha_ta
```

### Platform-Specific Notes

**Windows:**
- Requires Visual Studio Build Tools 2019 or later
- Python 3.8+ (64-bit recommended)

**macOS:**
- Requires Xcode Command Line Tools
- Apple Silicon (M1/M2) and Intel supported

**Linux:**
- GCC 7+ or Clang 6+ required
- glibc 2.17+ (Ubuntu 18.04+, CentOS 8+)

### Verification

```python
import finkit as ta
import numpy as np

close = np.arange(1.0, 51.0)
rsi = ta.rsi(close, timeperiod=14)
print(f"Python binding installed successfully! RSI length: {len(rsi)}")
```

## Node.js

### Option 1: npm (Recommended)

```bash
npm install @alphata/node
```

### Option 2: yarn

```bash
yarn add @alphata/node
```

### Option 3: Build from Source

```bash
# Navigate to Node.js binding directory
cd ffi/node-binding

# Install dependencies
npm install

# Build native module
npm run build

# Run tests
npm test
```

### Platform-Specific Notes

**Windows:**
- Requires Visual Studio Build Tools 2019 or later
- Node.js 16+ (LTS recommended)

**macOS:**
- Requires Xcode Command Line Tools
- macOS 10.13+ supported

**Linux:**
- glibc 2.17+ for gnu builds
- musl 1.1+ for musl builds (Alpine Linux)

### TypeScript Support

The package includes TypeScript definitions out of the box:

```typescript
import { sma, rsi, macd, MacdResult } from '@alphata/node';

const close = Array.from({ length: 100 }, (_, i) => i + 1);
const macdResult: MacdResult = macd(close, 12, 26, 9);
```

### Verification

```javascript
const ta = require('@alphata/node');
const close = Array.from({ length: 50 }, (_, i) => i + 1);
const rsi = ta.rsi(close, 14);
console.log(`Node.js binding installed! RSI length: ${rsi.length}`);
```

## Rust

### Option 1: cargo (Recommended)

```bash
cargo add alpha-ta-core
```

### Option 2: Manual (Cargo.toml)

```toml
[dependencies]
alpha-ta-core = "1.0.0"
```

### Option 3: Build from Source

```bash
# Build core library
cargo build --release

# Run tests
cargo test --package alpha-ta-core

# Optional: Build with all features
cargo build --release --all-features
```

### Features

```toml
[dependencies]
alpha-ta-core = { version = "1.0.0", features = ["formula"] }
```

Available features:
- `std` (default): Standard library support with ndarray, thiserror, ahash
- `formula` (default): Enable formula engine for custom indicator expressions (requires std)
- `serde` (default): Enable serialization support with serde and bincode
- `no_std`: Enable no_std support with libm (mutually exclusive with std)
- `rayon`: Enable parallel computation support
- `alpha-ta-polars`: Enable Polars DataFrame integration
- `nightly-avx512`: Enable AVX-512 SIMD optimizations (requires nightly Rust)
- `precision-f32`: Use f32 precision instead of f64

### Verification

```rust
use alpha_ta_core::indicators;

fn main() {
    let close: Vec<f64> = (1..=50).map(|x| x as f64).collect();
    let rsi = indicators::rsi(&close, 14).expect("RSI calculation failed");
    println!("Rust core installed! RSI length: {}", rsi.len());
}
```

## Java

### Option 1: Maven

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>com.alphata</groupId>
    <artifactId>alpha_ta</artifactId>
    <version>1.0.0</version>
</dependency>
```

### Option 2: Gradle

Add to your `build.gradle`:

```gradle
dependencies {
    implementation 'com.alphata:alpha_ta:1.0.0'
}
```

### Option 3: Build from Source

```bash
# Install Java JDK 11+
# Install Apache Maven 3.6+

# Navigate to Java binding directory
cd ffi/java-binding

# Build Rust library
cargo build --release

# Build Java artifacts
mvn clean install -DskipTests

# Run tests
mvn test
```

### Platform-Specific Notes

**Native Library Loading:**

The Java binding requires native libraries. Set the library path:

```bash
# Linux
export LD_LIBRARY_PATH=<repo_root>/ffi/java-binding/target/release:$LD_LIBRARY_PATH

# macOS
export DYLD_LIBRARY_PATH=<repo_root>/ffi/java-binding/target/release:$DYLD_LIBRARY_PATH

# Windows
set PATH=<repo_root>\ffi\java-binding\target\release;%PATH%
```

### Verification

```java
import com.alphata.Indicators;

public class Test {
    public static void main(String[] args) {
        double[] close = new double[50];
        for (int i = 0; i < 50; i++) close[i] = i + 1.0;

        double[] rsi = Indicators.rsi(close, 14);
        System.out.println("Java binding installed! RSI length: " + rsi.length);
    }
}
```

## Go

### Option 1: go get

```bash
go get github.com/coeasy/finkit/go/ta
```

### Option 2: Build from Source

```bash
# Install Go 1.21+
# Ensure CGO is enabled

# Navigate to Go binding directory
cd ffi/go-binding

# Build native library
make build

# Run tests
go test -v ./...
```

### Project Structure

```
your-project/
├── go.mod
├── go.sum
└── main.go
```

```go
module your-project

go 1.21

require github.com/coeasy/finkit v1.0.0
```

### Platform-Specific Notes

**CGO Requirements:**

Go bindings require CGO, which needs a C compiler:

```bash
# Linux (Ubuntu/Debian)
sudo apt-get install gcc

# macOS
xcode-select --install

# Windows
# Install MinGW or TDM-GCC
```

**Cross-Compilation:**

```bash
# Build for Linux
GOOS=linux GOARCH=amd64 make build

# Build for macOS
GOOS=darwin GOARCH=amd64 make build

# Build for Windows
GOOS=windows GOARCH=amd64 make build
```

### Verification

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit/go/ta"
)

func main() {
    close := make([]float64, 50)
    for i := 0; i < 50; i++ {
        close[i] = float64(i + 1)
    }

    rsi, err := ta.RSI(close, 14)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Go binding installed! RSI length: %d\n", len(rsi))
}
```

## .NET

### Option 1: NuGet CLI

```bash
dotnet add package alpha_ta
```

### Option 2: Package Manager Console

```powershell
Install-Package alpha_ta
```

### Option 3: Build from Source

```bash
# Install .NET SDK 8.0+

# Navigate to .NET binding directory
cd ffi/dotnet-binding

# Build Rust library
cargo build --release

# Build .NET library
cd src/alpha_ta
dotnet build --configuration Release

# Run tests
cd ../alpha_ta.Tests
dotnet test
```

### Platform-Specific Notes

**Native Library Deployment:**

The native libraries are automatically deployed to platform-specific folders:

```
alpha_ta/
├── runtimes/
│   ├── win-x64/native/
│   │   └── alpha_ta_dotnet.dll
│   ├── linux-x64/native/
│   │   └── libalpha_ta_dotnet.so
│   └── osx-x64/native/
│       └── libalpha_ta_dotnet.dylib
└── alpha_ta.dll
```

### Verification

```csharp
using System;
using System.Linq;
using alpha_ta;

class Program
{
    static void Main()
    {
        var close = Enumerable.Range(1, 50).Select(i => (double)i).ToArray();
        var rsi = Indicators.RSI(close, 14);
        Console.WriteLine($".NET binding installed! RSI length: {rsi.Length}");
    }
}
```

## C++

### Option 1: CMake Integration

```cmake
cmake_minimum_required(VERSION 3.15)
project(my_project)

# Add FTA as subdirectory
add_subdirectory(<repo_root>/ffi/c-binding)

# Link against FTA
target_link_libraries(your_target PRIVATE alpha_ta_c)
```

### Option 2: Manual Build

```bash
# Navigate to C binding directory
cd ffi/c-binding

# Build shared library
cargo build --release

# Copy header and library
cp include/alpha_ta.h /usr/local/include/
cp target/release/libalpha_ta_c.so /usr/local/lib/
```

### Usage

```cpp
#include <alpha_ta.h>
#include <iostream>

int main() {
    double close[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0};
    double* sma = nullptr;
    int len = 0;

    int result = alpha_ta_sma(close, 10, 3, &sma, &len);
    if (result == 0) {
        std::cout << "C++ binding installed! SMA length: " << len << std::endl;
        free(sma);
    }

    return 0;
}
```

## WebAssembly

### Option 1: npm

```bash
npm install alpha-ta-wasm
```

### Option 2: Build from Source

```bash
# Install wasm-pack
cargo install wasm-pack

# Navigate to WASM directory
cd wasm

# Build for web
wasm-pack build --target web --out-dir pkg

# Build for Node.js
wasm-pack build --target nodejs --out-dir pkg
```

### Usage

```typescript
import init, { rsi, sma, macd } from 'alpha-ta-wasm';

async function main() {
  await init();

  const close = Array.from({ length: 100 }, (_, i) => i + 1);
  const rsiResult = rsi(close, 14);
  console.log('WASM binding installed!');
}

main();
```

## CLI

### Option 1: cargo install

```bash
cargo install alpha-ta-cli
```

### Option 2: Build from Source

```bash
# Navigate to CLI directory
cd cli

# Build and install
cargo install --path .
```

### Usage

```bash
# Calculate indicators
alpha-ta-cli sma --input data.csv --period 14
alpha-ta-cli ema --input data.csv --period 14
alpha-ta-cli rsi --input data.csv --period 14
alpha-ta-cli macd --input data.csv --fast 12 --slow 26 --signal 9

# Export results
alpha-ta-cli rsi --input data.csv --output rsi.csv
```

## Troubleshooting

### Common Issues

**Python: "ModuleNotFoundError: No module named 'alpha_ta'"**
```bash
# Reinstall with verbose output
pip install --verbose alpha_ta
```

**Node.js: "Cannot find module '@alphata/node'"**
```bash
# Clear npm cache and reinstall
npm cache clean --force
npm install @alphata/node
```

**Java: "UnsatisfiedLinkError"**
```bash
# Ensure native library is in library path
export LD_LIBRARY_PATH=/path/to/native/lib:$LD_LIBRARY_PATH
```

**Go: "could not determine kind of name for C"**
```bash
# Ensure CGO is enabled
export CGO_ENABLED=1
```

**Rust: Build fails with linker errors**
```bash
# Install required dependencies
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config

# macOS
xcode-select --install
```

### Getting Help

- [GitHub Issues](https://github.com/coeasy/finkit/issues)
- [GitHub Discussions](https://github.com/coeasy/finkit/discussions)
