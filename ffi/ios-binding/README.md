# Finkit iOS Binding

The iOS binding compiles the Rust `finkit-ios` crate as a static library and packages device/simulator slices into `Finkit.xcframework`.

## Current support contract

The source integration contains:

- a Rust C ABI static library;
- generated indicator entry points;
- the `finkit.h` C header;
- a Clang module map named `FinkitC`;
- a Swift-facing `Finkit` wrapper that imports `FinkitC`;
- an XCFramework build script for physical arm64 devices and both Apple Silicon/Intel simulators.

The historical `alpha_ta_*` C symbols are intentionally retained as an internal compatibility ABI. New Swift code should use `Finkit`, not the deprecated `AlphaTA` alias.

## Requirements

- macOS with Xcode 15+;
- Rust 1.85+;
- these Rust targets:

```bash
rustup target add \
  aarch64-apple-ios \
  aarch64-apple-ios-sim \
  x86_64-apple-ios
```

`aarch64-apple-ios` is the physical-device target. The two simulator targets are combined into one universal simulator library before the XCFramework is created.

## Build the XCFramework

From the repository root:

```bash
bash ffi/ios-binding/build-xcframework.sh
```

Output:

```text
dist/ios/Finkit.xcframework
```

The script performs locked Rust builds, combines simulator architectures with `lipo`, packages `finkit.h` + `module.modulemap` + the Swift wrapper source, then calls `xcodebuild -create-xcframework`.

## Swift API

The low-level import module is intentionally named `FinkitC` so it does not collide with the public Swift type:

```swift
import FinkitC
```

The checked-in `Finkit.swift` wrapper then exposes a focused native subset including:

- SMA, EMA, WMA, DEMA, TEMA;
- RSI, ROC, MOM, CMO, TRIX;
- midpoint, z-score, TSF, linear regression, percent rank;
- candlestick detection count.

Example source usage once the XCFramework and wrapper source are integrated into the consuming target:

```swift
let prices: [Double] = [1, 2, 3, 4, 5, 6]
let sma = try Finkit.sma(prices, period: 3)
print(sma)
```

The multi-language CI type-checks `Finkit.swift` against the module map copied into the built XCFramework. This catches missing C imports and Swift wrapper signature errors in addition to native Rust/Xcode packaging failures.

## Packaging boundary

The XCFramework candidate verifies that device/simulator Rust libraries and the C module can be packaged together and that the Swift wrapper type-checks against that module.

It still does not by itself constitute a Swift Package Manager, CocoaPods, or binary registry publication. Before advertising a one-command iOS dependency, provide and clean-test a consumer package/module definition from a separate application project.

## ABI compatibility

The C header continues to declare `alpha_ta_*` names so existing integrations are not silently broken. The Swift wrapper exposes deprecated aliases:

```swift
AlphaTA      // deprecated alias of Finkit
AlphaTAError // deprecated alias of FinkitError
```

New code should use the Finkit names.

## Related documentation

- [Language bindings](../../docs/language-bindings.md)
- [Installation guide](../../docs/installation.md)
- [Troubleshooting](../../docs/troubleshooting.md)
- [Generated indicator catalog](../../docs/generated/indicators.md)

## License

MIT OR Apache-2.0
