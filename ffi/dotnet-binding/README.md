# Finkit .NET/P-Invoke Binding

This directory contains the .NET binding for Finkit, backed by the Rust `finkit-dotnet` native library and P/Invoke.

## Distribution status

The binding source exists in the published v0.1.3 repository, but a NuGet package is **not** part of the verified v0.1.3 GitHub Release asset matrix.

For the next release, the multi-language workflow adds a Linux validation gate that:

1. builds `libfinkit_dotnet.so` from Rust;
2. runs the .NET 8 test project against that native library;
3. stages the Linux native library into the standard NuGet RID layout;
4. creates a `.nupkg` candidate;
5. inspects the archive for `runtimes/linux-x64/native/libfinkit_dotnet.so`.

A green package-candidate job is still not a public NuGet publication.

## Requirements

- .NET 8 SDK for the current validation/test path;
- .NET 6 support is also declared by the library project;
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

```bash
LD_LIBRARY_PATH="$PWD/target/release:${LD_LIBRARY_PATH:-}" \
  dotnet test ffi/dotnet-binding/src/Finkit.Tests/Finkit.Tests.csproj \
  -c Release --framework net8.0
```

This verifies a real managed-to-native call rather than only compiling the C# project.

## Build the managed library

```bash
dotnet build ffi/dotnet-binding/src/Finkit/Finkit.csproj -c Release
```

Basic usage:

```csharp
using Finkit;

double[] close = { 1, 2, 3, 4, 5 };
double[] sma = Indicators.Sma(close, 3);
Console.WriteLine(sma[^1]);
```

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

For example, the Linux candidate workflow stages the native library and packs with:

```bash
mkdir -p ffi/dotnet-binding/native/linux-x64/native
cp target/release/libfinkit_dotnet.so \
  ffi/dotnet-binding/native/linux-x64/native/

dotnet pack ffi/dotnet-binding/src/Finkit/Finkit.csproj \
  -c Release -o dist/dotnet
```

Then inspect the archive instead of assuming the RID was included:

```bash
unzip -l dist/dotnet/Finkit.0.1.3.nupkg
```

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
