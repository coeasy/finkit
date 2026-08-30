# finkit Android Bindings

Android (JNI) bindings for [finkit](https://github.com/coeasy/finkit), the
high-performance financial technical analysis library. The library is written
in Rust and shipped to Android as a `.so` inside an Android Archive (`.aar`).

## Layout

```
ffi/android-binding/
├── Cargo.toml                # the Rust crate that produces libfinkit_android.so
├── src/lib.rs                # JNI shim around finkit-java
└── android/                  # Android library Gradle module
    ├── build.gradle.yaml
    └── src/main/
        ├── AndroidManifest.xml
        └── java/com/finkit/indicators/Finkit.java
```

## Building the AAR

Pre-requisites:

- Android NDK 25 or later
- `ANDROID_NDK_HOME` and `ANDROID_HOME` set
- A recent Rust toolchain (`rustup default stable` + `rustup target add aarch64-linux-android24 ...`)

```bash
# 1. Build the .so for all four ABIs (arm64, armv7, x86_64, x86)
cargo build --release -p finkit-android --target aarch64-linux-android24
cargo build --release -p finkit-android --target armv7-linux-androideabi24
cargo build --release -p finkit-android --target x86_64-linux-android24
cargo build --release -p finkit-android --target i686-linux-android24

# 2. Assemble the .aar
cd ffi/android-binding/android
./gradlew :aar
# → app/build/outputs/aar/finkit-android-release.aar
```

## Using the AAR

```gradle
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        flatDir { dirs("libs") }
    }
}

// app/build.gradle.kts
dependencies {
    implementation(files("libs/finkit-android-release.aar"))
}
```

```kotlin
import com.finkit.indicators.Finkit

val prices = doubleArrayOf(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0)
val sma = Finkit.sma(prices, period = 3)
```

## License

MIT OR Apache-2.0
