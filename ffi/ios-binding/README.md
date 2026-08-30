# AlphaTA-ios

iOS bindings for [AlphaTA](https://github.com/coeasy/finkit), the
high-performance financial technical analysis library.

The Rust crate is compiled as a `staticlib` and packaged into an Apple
`.xcframework` (one slice per supported target). The Swift wrapper
(`Finkit.swift`) sits on top of the C ABI and provides a friendly,
allocation-free API.

## Building the .xcframework

Pre-requisites:

- Xcode 15 or later (provides `xcodebuild`)
- Rust toolchain with the four iOS targets installed:
  ```bash
  rustup target add aarch64-apple-ios \
                    aarch64-apple-ios-sim \
                    x86_64-apple-ios \
                    x86_64-apple-ios-sim
  ```

```bash
./ffi/ios-binding/build-xcframework.sh
# → dist/ios/Finkit.xcframework
```

## Using the .xcframework

1. In Xcode, drag `Finkit.xcframework` into your project.
2. In *Build Settings*, ensure *Validate Workspace* is **Yes** and the
   framework is added to *Frameworks, Libraries, and Embedded Content* as
   *Embed & Sign*.
3. In any Swift file:

```swift
import Finkit

let prices: [Double] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let sma = try AlphaTA.sma(prices, period: 3)
print(sma) // [0, 0, 2, 3, 4, 5, 6, 7, 8, 9]
```

## License

MIT OR Apache-2.0
