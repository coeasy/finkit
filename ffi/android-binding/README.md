# Finkit Android Binding

Finkit's Android binding packages the Rust JNI library as an Android Archive (`.aar`) with the Java API under `com.finkit.indicators.Finkit`.

## Current support contract

The Android source contains:

- the Rust `finkit-android` JNI crate;
- generated JNI indicator shims;
- the Java `Finkit` wrapper;
- a standard Gradle Android Library project;
- a `jniLibs` staging path for Rust `.so` files.

The multi-language CI builds the native Android libraries and assembles the AAR before Android can be described as a validated release target. A GitHub Actions artifact is not the same as publication to Maven Central or another Android package registry.

## Requirements

- Rust 1.85+;
- Android SDK with platform 34;
- Android NDK 25+;
- Java 17 for the Android Gradle Plugin;
- Gradle 8.7+;
- `cargo-ndk`.

Install Rust/NDK helpers:

```bash
cargo install cargo-ndk --locked
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android
```

## Build native libraries

From the repository root:

```bash
cargo ndk \
  --platform 24 \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  -o ffi/android-binding/android/src/main/jniLibs \
  build --release -p finkit-android --locked
```

The output directory should contain ABI-specific copies of `libfinkit_android.so`.

## Assemble the AAR

```bash
cd ffi/android-binding/android
gradle assembleRelease
```

The release AAR is written below:

```text
build/outputs/aar/
```

Before distributing it, inspect the archive and confirm that the advertised ABIs contain `libfinkit_android.so`.

## Use the AAR

For a local application, copy the AAR into the app's `libs/` directory and add it as a file dependency. For example:

```kotlin
dependencies {
    implementation(files("libs/finkit-android-release.aar"))
}
```

Java/Kotlin usage:

```kotlin
import com.finkit.indicators.Finkit

val prices = doubleArrayOf(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
val sma = Finkit.sma(prices, 3)
println(Finkit.version())
```

`Finkit` loads `finkit_android` automatically when the class is first referenced. There is no separate `init()` call.

## API scope

The current Android wrapper includes a focused generated subset covering moving averages, momentum, and statistics, including SMA, EMA, WMA, DEMA, TEMA, midpoint, RSI, ROC, MOM, CMO, TRIX, z-score, TSF, linear regression, and percent rank.

Do not infer complete parity with Python/Rust solely because both bindings use the same core. Binding API coverage and package validation are separate contracts.

## Distribution status

Until a new release actually attaches and smoke-tests the AAR, treat Android as a CI-validated/source-build target rather than a public registry package. Do not advertise Maven coordinates that have not been published.

## Related documentation

- [Language bindings](../../docs/language-bindings.md)
- [Installation guide](../../docs/installation.md)
- [Generated indicator catalog](../../docs/generated/indicators.md)

## License

MIT OR Apache-2.0
