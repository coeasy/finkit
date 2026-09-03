# Finkit .NET/P-Invoke Binding

This directory contains the .NET binding for Finkit, backed by the Rust `finkit-dotnet` native library and P/Invoke.

## v0.1.3 status

The source binding exists in the repository, but it is **not** part of the verified `v0.1.3` GitHub Release asset matrix and this documentation does not assume a NuGet package has been published.

Use the binding from source for development and integration validation. Add a `dotnet add package ...` instruction only after a real NuGet package is published and install-tested.

## Requirements

- .NET SDK compatible with the project files under `src/`;
- Rust 1.85+;
- the native compiler/linker for the target platform.

## Build the native library

From the repository root:

```bash
cargo build -p finkit-dotnet --release --locked
```

The native output is platform-specific (`.dll`, `.so`, or `.dylib`). It must be copied/staged where the managed project can load it.

## Build the managed project

```bash
cd ffi/dotnet-binding/src/Finkit
dotnet build -c Release
```

Before running an application, ensure the matching native library is available in the application's native search path or expected runtime-native directory.

## Basic usage

```csharp
using Finkit;

double[] close = { 1, 2, 3, 4, 5 };
double[] sma = Indicators.Sma(close, 3);
Console.WriteLine(sma[^1]);
```

The binding source contains wrappers for multiple indicator families. Use the current managed source and generated core registry as the exact API source of truth rather than relying on a hard-coded indicator count in documentation.

## Native memory ownership

Some native APIs return Rust-allocated C strings. The managed side must copy the value and release the original pointer through the binding's matching native free function, such as `ta_free_cstring`, rather than using an unrelated allocator.

Conceptual pattern:

```csharp
IntPtr raw = NativeBindings.ta_get_version();
try
{
    string? version = Marshal.PtrToStringAnsi(raw);
    Console.WriteLine(version);
}
finally
{
    NativeBindings.ta_free_cstring(raw);
}
```

Never free the same pointer twice, and do not pair a Rust-owned pointer with `Marshal.FreeHGlobal` unless the allocating API explicitly documents that allocator contract.

See [FFI memory contract](../../docs/ffi/memory-contract.md) and [FFI error codes](../../docs/ffi/error-codes.md).

## Before publishing a NuGet package

A production NuGet release should verify:

1. package version matches the repository release version;
2. native assets are laid out by correct RID;
3. clean external projects can restore and run on every advertised OS/architecture;
4. P/Invoke names and native filenames match each platform;
5. memory/free functions pass repeated-call leak tests;
6. the package is actually visible in the target NuGet feed.

## Related documentation

- [Installation guide](../../docs/installation.md)
- [Complete usage guide](../../docs/usage.md)
- [Indicator catalog](../../docs/generated/indicators.md)

## License

MIT OR Apache-2.0
