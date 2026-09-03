# Finkit Go/CGO Binding

This directory contains the Go/CGO binding source for Finkit.

## v0.1.3 status

The binding is available for repository/source development, but it is **not** part of the verified `v0.1.3` GitHub Release asset matrix. The nested Go module currently lives at `ffi/go-binding/go/go.mod` and declares:

```text
module github.com/coeasy/finkit
```

Its Go package is under `ffi/go-binding/go/ta`, so **inside that nested module** the package import path is `github.com/coeasy/finkit/ta`. Because the module itself is nested below `ffi/go-binding/go/` while its declared module path points at the repository root, this is not yet a clean remotely versioned public-module layout. The repository therefore does not document a `go get` command as a stable public v0.1.3 installation contract.

## Requirements

- Go 1.21+;
- CGO enabled;
- Rust 1.85+;
- a C compiler/linker compatible with the host Go toolchain.

Check CGO:

```bash
go env CGO_ENABLED
```

## Build the native Rust library

From the repository root:

```bash
cargo build -p finkit-go --release --locked
```

The Rust package name is `finkit-go` and produces the native library used by the CGO layer.

## Work with the nested Go source

```bash
cd ffi/go-binding/go
go test ./...
```

When integrating from a repository checkout, ensure the linker can locate the native library produced by the Rust build. Exact linker configuration is platform-specific; inspect the CGO directives and build scripts in this directory before packaging an application.

## Local external-module example

For development before the public module layout is finalized, an external Go project can point the declared module path at the nested checkout explicitly.

Example `go.mod`:

```go
module example.com/my-finkit-app

go 1.21

require github.com/coeasy/finkit v0.0.0

replace github.com/coeasy/finkit => ../finkit/ffi/go-binding/go
```

Then import the actual package inside that local module:

```go
package main

import (
    "fmt"

    "github.com/coeasy/finkit/ta"
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

This local `replace` workflow is a development technique, not proof that `github.com/coeasy/finkit` is currently a remotely installable Go module at v0.1.3.

## Before making Go a public release contract

A production Go distribution should first resolve all of the following:

1. choose a public module root/path that matches the repository directory layout;
2. define the version/tag convention for the Go module;
3. decide whether users receive prebuilt native libraries or must have Rust installed;
4. verify Linux/macOS/Windows CGO linking on the advertised architectures;
5. run an install test from a clean external Go module using the published tag/artifact;
6. only then document a `go get` command.

## Related documentation

- [Installation guide](../../docs/installation.md)
- [Complete usage guide](../../docs/usage.md)
- [FFI memory contract](../../docs/ffi/memory-contract.md)

## License

MIT OR Apache-2.0
