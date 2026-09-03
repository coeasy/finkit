# Finkit .NET/P-Invoke Binding

This directory contains the .NET binding for Finkit, backed by the Rust `finkit-dotnet` native library and P/Invoke.

## Distribution status

The binding source exists in the published v0.1.3 repository, but a NuGet package is **not** part of the verified v0.1.3 GitHub Release asset matrix.

For the next release, the multi-language workflow adds a Linux validation gate that:

1. builds `libfinkit_dotnet.so` from Rust;
2. runs the managed/native tests with the binding-local .NET 8 SDK contract;
3. stages the Linux native library into the standard NuGet RID layout;
4. creates a Linux `.nupkg` candidate without relying on runner-global SDK defaults;
5. inspects the archive for `runtimes/linux-x64/native/libfinkit_dotnet.so`.

A green package-candidate job is still not a public NuGet publication.

## SDK contract

`ffi/dotnet-binding/global.json` pins binding validation to the .NET 8 SDK family while allowing the latest installed .NET 8 feature band. This prevents a newer preinstalled GitHub runner SDK from silently changing CLI/MSBuild behavior.

Check it from the binding directory:

```bash
cd ffi/dotnet-binding
dotnet --version
```

The current CI expects the reported SDK version to begin with `8.`.

The library project declares `net6.0` and `net8.0`; the permanent Linux smoke/test gate executes the .NET 8 target.

## Requirements

- .NET 8 SDK for the current validation/test path;
- .NET 6 targeting support for the declared library target;
- Rust 1.85+;
- a native compiler/linker for the target platform.

## Build the native library

From the repository root:

```bash
cargo build -p finkit-dotnet --release --locked
```

Expected native filenames:

| RID family | Native file |
| --- | --- |
| Windows | `finkit_dotnet.dll` |
| Linux | `libfinkit_dotnet.so` |
| macOS | `libfinkit_dotnet.dylib` |

## Test on Linux

From the repository root:

```bash
cargo build -p finkit-dotnet --release --locked
cd ffi/dotnet-binding
LD_LIBRARY_PATH="../../target/release:${LD_LIBRARY_PATH:-}" \
  dotnet test src/Finkit.Tests/Finkit.Tests.csproj \
  -c Release --framework net8.0
```

This verifies real managed-to-native calls rather than only compiling the C# project. The ADX test intentionally provides more than `period * 2` bars because that is the core ADX minimum-history contract.

## Build the managed library

```bash
cd ffi/dotnet-binding
dotnet build src/Finkit/Finkit.csproj -c Release
```

Basic usage:

```csharp
using Finkit;

double[] close = { 1, 2, 3, 4, 5 };
double[] sma = Indicators.Sma(close, 3);
Console.WriteLine(sma[^1]);
```

## Native loading

`NativeBootstrap.cs` registers an assembly-level `DllImportResolver` for `finkit_dotnet`.

It checks, in order:

- explicit `FINKIT_NATIVE_PATH`;
- app/assembly-local native files;
- standard `runtimes/<rid>/native/` locations;
- the operating system's normal native-library search path.

This keeps source tests and packaged NuGet candidates on the same loading model.

## Native package layout

The project can include native assets from these staging paths:

```text
ffi/dotnet-binding/native/win-x64/native/*.dll
ffi/dotnet-binding/native/linux-x64/native/*.so
ffi/dotnet-binding/native/osx-x64/native/*.dylib
ffi/dotnet-binding/native/osx-arm64/native/*.dylib
```

They are packed into standard NuGet paths:

```text
runtimes/win-x64/native/
runtimes/linux-x64/native/
runtimes/osx-x64/native/
runtimes/osx-arm64/native/
```

For the validated Linux candidate:

```bash
cargo build -p finkit-dotnet --release --locked
mkdir -p ffi/dotnet-binding/native/linux-x64/native
cp target/release/libfinkit_dotnet.so \
  ffi/dotnet-binding/native/linux-x64/native/

cd ffi/dotnet-binding
dotnet pack src/Finkit/Finkit.csproj -c Release
unzip -l src/Finkit/bin/Release/Finkit.0.1.3.nupkg
```

CI copies that verified package to a GitHub artifact name like:

```text
finkit-dotnet-<version>-linux-x64.nupkg
```

The platform suffix is intentional: the current permanent package gate proves Linux x64 native content. Windows/macOS RID declarations are not promoted to verified distribution status until their native-runner package tests pass too.

## Formula and indicator scope

`Indicators.cs` contains the managed P/Invoke surface for indicators, patterns, formulas, and related helpers. Treat that source file and its tests as the .NET API source of truth. Do not infer complete parity with Python, Go, or Rust from the shared core alone.

Native strings returned by formula helpers are copied into managed strings and released through the matching Finkit native free function inside the wrapper. Consumers should call the managed methods rather than trying to free internal native pointers themselves.

## Before public NuGet publication

A production NuGet release should verify:

1. package version matches the Finkit release version;
2. every advertised RID actually contains the matching native file;
3. clean external projects can restore and execute a real indicator/formula call;
4. Windows/macOS package paths are tested on their native runners, not inferred from Linux;
5. P/Invoke/native loading passes repeated-call tests;
6. the package is actually visible in the configured NuGet feed.

Only after those checks should documentation advertise `dotnet add package Finkit` from a public feed.

## Related documentation

- [Language bindings](../../docs/language-bindings.md)
- [Installation guide](../../docs/installation.md)
- [Troubleshooting](../../docs/troubleshooting.md)
- [Indicator catalog](../../docs/generated/indicators.md)

## License

MIT OR Apache-2.0
