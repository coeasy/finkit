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

This layout now matches the repository directory structure and can be tested from a checkout without inventing an alternative import path. Public remote installation is still a separate release contract because a nested module needs a compatible subdirectory tag and a native-library distribution strategy before `go get` can be advertised as a clean end-user install path.

## Requirements

- Go 1.21+;
- CGO enabled;
- Rust 1.85+;
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

The CGO package also contains repository-relative linker flags based on `${SRCDIR}` so compilation does not depend on the caller's working directory.

The convenience Makefile performs the same source build/test flow:

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

The full repository example is in `ffi/go-binding/examples/example.go`.

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
2. decide whether users receive prebuilt native libraries or need Rust locally;
3. verify CGO compile and runtime loading on every advertised OS/architecture;
4. test installation from a clean external module using the published tag;
5. ensure the Go module version and Finkit release version remain aligned;
6. only then advertise the public `go get` command.

## Related documentation

- [Installation guide](../../docs/installation.md)
- [Language bindings](../../docs/language-bindings.md)
- [Complete usage guide](../../docs/usage.md)
- [FFI memory contract](../../docs/ffi/memory-contract.md)

## License

MIT OR Apache-2.0
