# Installation Guide

This document provides detailed installation instructions for each language binding of Finkit.

> **Release status:** the source baseline is `0.1.2`, but the GitHub `v0.1.2` Release is still pending. Until it is published, use a successful [Python wheels workflow](https://github.com/coeasy/finkit/actions/workflows/python-wheels.yml) artifact or build from source.

## Prerequisites

### Common Requirements

- **Rust toolchain**: Required for building from source
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Git**: For cloning the repository
  ```bash
  git clone https://github.com/coeasy/finkit.git
cd finkit
  ```

## Python

Finkit Python wheel 由 GitHub Actions 使用 maturin 构建。完整的版本、平台、wheel 选择、源码构建、测试和故障排查说明见 [Python 安装与发布指南](python.md)。

### 已构建 wheel

从 [GitHub Releases](https://github.com/coeasy/finkit/releases) 下载匹配本机的 `finkit-0.1.2-*.whl`，然后执行：

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.2-<matching-wheel>.whl
```

支持 CPython 3.8–3.14，以及 Linux x86_64、macOS x86_64/arm64 和 Windows x86_64。pip 会自动安装 NumPy 运行时依赖。

### 从源码构建

```bash
python -m venv .venv
source .venv/bin/activate       # Windows PowerShell: .\\.venv\\Scripts\\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest

cd ffi/python-binding
maturin develop --release

cd ../..
python -m pytest ffi/python-binding/tests -q
```

源码构建需要 Rust 1.85+ 和平台 C/C++ 编译工具链。

## Node.js

### Option 1: npm (Recommended)

```bash
npm install finkit
```

### Option 2: yarn

```bash
yarn add finkit
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
import { sma, rsi, macd, MacdResult } from 'finkit';

const close = Array.from({ length: 100 }, (_, i) => i + 1);
const macdResult: MacdResult = macd(close, 12, 26, 9);
```

### Verification

```javascript
const ta = require('finkit');
const close = Array.from({ length: 50 }, (_, i) => i + 1);
const rsi = ta.rsi(close, 14);
console.log(`Node.js binding installed! RSI length: ${rsi.length}`);
```

## Rust

### Option 1: cargo (Recommended)

```bash
cargo add finkit
```

### Option 2: Manual (Cargo.toml)

```toml
[dependencies]
finkit = "0.1.2"
```

### Option 3: Build from Source

```bash
# Build core library
cargo build --release

# Run tests
cargo test --package finkit

# Optional: Build with all features
cargo build --release --all-features
```

### Features

```toml
[dependencies]
finkit = { version = "0.1.2", features = ["formula"] }
```

Available features:
- `std` (default): Standard library support with ndarray, thiserror, ahash
- `formula` (default): Enable formula engine for custom indicator expressions (requires std)
- `serde` (default): Enable serialization support with serde and bincode
- `no_std`: Enable no_std support with libm (mutually exclusive with std)
- `rayon`: Enable parallel computation support
- `finkit-polars`: Enable Polars DataFrame integration
- `nightly-avx512`: Enable AVX-512 SIMD optimizations (requires nightly Rust)
- `precision-f32`: Use f32 precision instead of f64

### Verification

```rust
use finkit::indicators;

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
    <groupId>com.finkit</groupId>
    <artifactId>finkit</artifactId>
    <version>0.1.2</version>
</dependency>
```

### Option 2: Gradle

Add to your `build.gradle`:

```gradle
dependencies {
    implementation 'com.finkit:finkit:0.1.2'
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
import com.finkit.Indicators;

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
dotnet add package finkit
```

### Option 2: Package Manager Console

```powershell
Install-Package finkit
```

### Option 3: Build from Source

```bash
# Install .NET SDK 8.0+

# Navigate to .NET binding directory
cd ffi/dotnet-binding

# Build Rust library
cargo build --release

# Build .NET library
cd src/finkit
dotnet build --configuration Release

# Run tests
cd ../finkit.Tests
dotnet test
```

### Platform-Specific Notes

**Native Library Deployment:**

The native libraries are automatically deployed to platform-specific folders:

```
finkit/
├── runtimes/
│   ├── win-x64/native/
│   │   └── finkit_dotnet.dll
│   ├── linux-x64/native/
│   │   └── libfinkit_dotnet.so
│   └── osx-x64/native/
│       └── libfinkit_dotnet.dylib
└── finkit.dll
```

### Verification

```csharp
using System;
using System.Linq;
using Finkit;

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
target_link_libraries(your_target PRIVATE finkit)
```

### Option 2: Manual Build

```bash
# Navigate to C binding directory
cd ffi/c-binding

# Build shared library
cargo build --release

# Copy header and library
cp include/finkit.h /usr/local/include/
cp target/release/libfinkit_ffi.so /usr/local/lib/
```

### Usage

```cpp
#include <finkit.h>
#include <iostream>

int main() {
    double close[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0};
    double* sma = nullptr;
    int len = 0;

    int result = ta_sma(close, 10, 3, &sma, &len);
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
npm install finkit-wasm
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
import init, { rsi, sma, macd } from 'finkit-wasm';

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
cargo install finkit
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
finkit sma --input data.csv --period 14
finkit ema --input data.csv --period 14
finkit rsi --input data.csv --period 14
finkit macd --input data.csv --fast 12 --slow 26 --signal 9

# Export results
finkit rsi --input data.csv --output rsi.csv
```

## Troubleshooting

### Common Issues

**Python: "ModuleNotFoundError: No module named 'finkit'"**

确认 `python` 和 `python -m pip` 使用同一个虚拟环境，并且不要在源码目录中直接验证已安装 wheel；请参阅 [Python 故障排查](python.md#常见问题)。

**Node.js: "Cannot find module 'finkit'"**
```bash
# Clear npm cache and reinstall
npm cache clean --force
npm install finkit
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
