# Finkit Go/CGO Binding

This directory contains the Go/CGO binding for Finkit.

## Current status

The binding has a real Go package, a Rust native library, tests, formula support, streaming wrappers, and examples. The nested Go module lives at `ffi/go-binding/go/` and declares the canonical module path:

```text
module github.com/coeasy/finkit/ffi/go-binding/go
```

The public Go package is therefore:

```text
github.com/coeasy/finkit/ffi/go-binding/go/ta
```

The next-release Linux gate validates both repository-source consumption and a standalone archive containing the Go module plus `libfinkit_go.so`. Public remote `go get` remains a separate contract because the nested module still needs a compatible published tag and a long-term cross-platform native-library delivery strategy.

## Requirements

- Go 1.21+;
- CGO enabled;
- Rust 1.85+ when building the native library from source;
- a C compiler/linker compatible with the host Go toolchain.

Check CGO:

```bash
go env CGO_ENABLED
```

## Build and test from source

From the repository root:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
LD_LIBRARY_PATH="../../../target/release:${LD_LIBRARY_PATH:-}" go test ./...
```

On macOS use `DYLD_LIBRARY_PATH` instead of `LD_LIBRARY_PATH` when the dynamic loader cannot find `libfinkit_go.dylib`. On Windows place `finkit_go.dll` on `PATH` or next to the test/application executable.

The CGO package contains `${SRCDIR}`-relative linker search paths. Linux first checks the packaged module-local location `go/native/linux-x86_64/`, then falls back to the repository `target/release` path used by source development.

The convenience Makefile performs the source build/test flow:

```bash
make -C ffi/go-binding test
```

## Basic usage

```go
package main

import (
    "fmt"

    "github.com/coeasy/finkit/ffi/go-binding/go/ta"
)

func main() {
    close := []float64{1, 2, 3, 4, 5}
    sma, err := ta.Sma(close, 3)
    if err != nil {
        panic(err)
    }
    fmt.Println(sma)
}
```

The full repository example is in `ffi/go-binding/examples/example.go`. It intentionally contains enough history for the strictest included example (`MACD(12,26,9)` needs at least 34 bars).

## Formula support

The Go binding exposes formula validation/evaluation in addition to indicator wrappers. Current source includes:

- `FormulaValidate`;
- `FormulaEval`;
- `FormulaEvalMultiJSON`;
- `FormulaEvalDrawJSON`;
- `FormulaEvalDebugJSON`;
- `FormulaEvalZeroCopy`;
- formula template helpers.

`FormulaEvalDebugJSON` is backed by the native `ta_formula_eval_debug` entry point and returns the current formula debugger event payload as JSON. This debugger surface is binding-specific; do not assume every other language binding exposes the same method name or payload.

## Standalone Linux candidate

The multi-language workflow packages a candidate archive named like:

```text
finkit-go-<version>-linux-x86_64.tar.gz
```

Its relevant layout is:

```text
finkit-go-<version>-linux-x86_64/
├── README.md
└── go/
    ├── go.mod
    ├── ta/
    └── native/
        └── linux-x86_64/
            └── libfinkit_go.so
```

CI does not stop at creating the tarball. It extracts the archive into a clean temporary directory, creates a separate Go module with a local `replace` pointing at the extracted `go/` directory, and runs the real example with only the packaged native-library directory on `LD_LIBRARY_PATH`. This verifies that the candidate does not accidentally depend on the repository's `target/release` tree.

## Local external-module integration

Before a nested-module release tag is published and installation-tested, an external Go project can use a checkout explicitly.

Example `go.mod`:

```go
module example.com/my-finkit-app

go 1.21

require github.com/coeasy/finkit/ffi/go-binding/go v0.0.0

replace github.com/coeasy/finkit/ffi/go-binding/go => ../finkit/ffi/go-binding/go
```

Then import:

```go
import "github.com/coeasy/finkit/ffi/go-binding/go/ta"
```

This `replace` workflow is intended for source integration. It is not a claim that a stable remote Go module/native binary has already been published.

## Public Go release requirements

Before documenting a plain `go get` workflow, the project should verify all of the following:

1. publish a nested-module tag matching the module directory convention;
2. decide and document the native-library strategy for every advertised OS/architecture;
3. verify CGO compile and runtime loading on every advertised target;
4. test installation from a clean external module using the published tag;
5. ensure the Go module version and Finkit release version remain aligned;
6. only then advertise the public `go get` command.

## Related documentation

- [Installation guide](../../docs/installation.md)
- [Language bindings](../../docs/language-bindings.md)
- [Complete usage guide](../../docs/usage.md)
- [Troubleshooting](../../docs/troubleshooting.md)
- [FFI memory contract](../../docs/ffi/memory-contract.md)

## License

MIT OR Apache-2.0
