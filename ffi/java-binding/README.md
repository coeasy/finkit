# Finkit Java/JNI Binding

This directory contains the Java binding for Finkit `v0.1.3`, backed by a Rust JNI library.

## Status

The permanent multi-language CI validates the Java packaging path by:

1. building the Rust `finkit-java` JNI library;
2. copying the host native library into the JAR resource layout;
3. running Maven package/Javadoc;
4. asserting that the native resource is present in the JAR;
5. compiling and running a Java smoke program that loads the native library and computes SMA.

The GitHub `v0.1.3` Release does not currently contain a Java JAR/native bundle, and this documentation does **not** assume that `com.finkit:finkit:0.1.3` has been published to Maven Central.

## Requirements

- Rust 1.85+;
- JDK compatible with the project's Java source/target configuration;
- Maven;
- platform native compiler/linker.

## Build from source

From the repository root on Linux x86_64:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

For macOS or Windows, build the corresponding Rust dynamic library and stage it under the matching `natives/<os>-<arch>/` directory with the platform filename before packaging the JAR.

## Native loading

`NativeLoader` uses this order:

1. an explicit native path supplied through the `finkit.native.path` system property;
2. a packaged native resource under `/natives/<os>-<arch>/<mapped-library-name>`;
3. `System.loadLibrary("finkit_java")` as the normal system-library fallback.

Example with an explicit native library:

```bash
java -Dfinkit.native.path=/absolute/path/to/libfinkit_java.so -cp your-app.jar Example
```

A Java JAR is not independently portable unless it contains the native library for the target OS/architecture or the process can load that library externally.

## Basic usage

```java
import com.finkit.Indicators;

public class Example {
    public static void main(String[] args) {
        double[] close = {1, 2, 3, 4, 5};
        double[] sma = Indicators.sma(close, 3);
        System.out.println(sma[sma.length - 1]);
    }
}
```

The Java API also includes wrappers for momentum, volatility, volume, statistical, price-transform, candlestick, chart-pattern, formula, and other Finkit capabilities implemented by the binding. Use the Java sources and generated core registry as the exact current API source of truth rather than relying on a hard-coded indicator count.

## Packaging for another platform

The resource directory must match the platform expected by `NativeLoader`. The current loader recognizes platform families including:

- `windows-x86_64` and `windows-aarch64`;
- `macos-x86_64` and `macos-aarch64`;
- `linux-x86_64` and `linux-aarch64`.

The native library name is platform-mapped, for example:

- Windows: `finkit_java.dll`;
- Linux: `libfinkit_java.so`;
- macOS: `libfinkit_java.dylib`.

A platform being recognized by the loader does not by itself mean that a prebuilt v0.1.3 artifact has been published for it.

## Validate the JAR

After Maven packaging, inspect the artifact:

```bash
jar tf ffi/java-binding/target/*.jar | grep natives
```

Then compile/run a small Java program using `Indicators.sma(...)` on the target platform. A successful Maven build without a successful native-load smoke test is not sufficient release validation.

## Registry publication

The POM contains publication metadata, but registry publication is a separate release operation. Do not document a Maven/Gradle dependency from Maven Central until the exact `com.finkit:finkit:<version>` artifact is visible and install-tested from that registry.

## Related documentation

- [Installation guide](../../docs/installation.md)
- [Complete usage guide](../../docs/usage.md)
- [Indicator catalog](../../docs/generated/indicators.md)
- [FFI memory contract](../../docs/ffi/memory-contract.md)

## License

MIT OR Apache-2.0
