# Build Guide · 构建指南

本文件给出各目标按平台的编译步骤。核心统一由 Cargo 驱动；若只想要"开箱即用"的多语言产物，优先看 [ONE_CLICK_BUILD.md](./ONE_CLICK_BUILD.md)。

## 0. 必备工具链

- Rust：MSRV 1.75，推荐 stable 1.82+（`rustup`）。
- 通用构建：`make`（可选，纯 `make` target）；Windows 用 PowerShell 7（`pwsh`）。
- 各语言绑定额外依赖见下表。

## 1. 核心库（crate）

```bash
cargo build --release
cargo test                                   # 全测试
cargo build --release --no-default-features  # 裁剪默认特性（见 core/Cargo.toml）
```

常用特性：`std`、`formula`、`indicators-all`（默认全开）、`rayon`（并行批量）、`feature` 相关、`unchecked-indexing`（越界热路径）、`talib-c`（TA-Lib C）
、`alpha-ta-polars`（Polars 集成）。

## 2. CLI

```bash
cargo build --release -p finkit-cli
# 产物：target/release/finkit-cli(.exe)
cargo install --path cli
```

## 3. 各语言绑定

| 语言 | 构建命令 | 产物 |
|------|---------|------|
| C/C++ | `cargo build --release -p finkit-ffi` | `alpha_ta.{a,so,dll}` + `include/alpha_ta.h`（cbindgen 生成） |
| Python | `cd ffi/python-binding && maturin build --release`（手动）；`maturin develop`（本地）。另见 `ffi/python-binding/pyproject.toml` 的 `cibuildwheel` 配置 | `.whl` / sdist |
| Node | `cd ffi/node-binding && npm install && npm run build` | `*.node` + `index.d.ts` |
| Go | `cd ffi/go-binding && make build` | C shim + `go/ta` 包 |
| Java | `cargo build --release`（原生库）；JAR 由 Maven（`ffi/java-binding/pom.xml`）打包 | `.so/.dll` + `.jar` |
| .NET | `cd ffi/dotnet-binding && cargo build --release`；NuGet 打包 | `.nupkg` |
| iOS | `./ffi/ios-binding/build-xcframework.sh`（需 Xcode + 4 个 iOS target） | `AlphaTA.xcframework` |
| Android | 见 `ffi/android-binding/README.md`（NDK 25+，四个 ABI） | `AlphaTA-android-release.aar` |
| WASM | `cd wasm && wasm-pack build --target web` | wasm + JS glue |

> 注意：iOS/Android/Go 等需要对应平台工具链；本机缺失时对应 crate 可不编译，不影响其他成员。

## 4. 校验与规范（与 CI 对齐）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo doc --no-deps
cargo audit                    # 需 cargo-audit，若有
cargo deny check               # 需 cargo-deny，基于 deny.toml
cargo fuzz run formula         # 模糊测试，需 cargo-fuzz
```

## 5. 常见问题

- **Windows 构建脚本执行受限**：改用 `pwsh` 或直接调 `cargo` 等价命令（代码跨平台已验证，Linux/macOS 无碍）。
- **crate 编译很慢**：绑定 crate 各自 `codegen-units=16`；核心用 `release` 的 `lto="thin"`。调试其一单独 `cargo build -p <crate>`。
- **找不到某绑定产物**：检查对应平台工具链是否就绪，产物默认输出 `target/release/` 或 `dist/`。