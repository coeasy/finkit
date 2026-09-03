# Finkit Go/CGO Binding

This directory contains the Go/CGO binding source for Finkit.

## v0.1.3 status

The binding is available for repository/source development, but it is **not** part of the verified `v0.1.3` GitHub Release asset matrix. The nested Go module currently lives at `ffi/go-binding/go/go.mod` and declares:

```text
module github.com/coeasy/finkit
```

Because that module is nested below `ffi/go-binding/go/` and still depends on a separately built native Rust library, the repository does not document `go get github.com/coeasy/finkit/go/ta` as a stable public v0.1.3 installation contract.

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

## API example

The Go source exposes indicator wrappers in the binding package. A repository-development example has the following shape:

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit/go/ta"
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

Treat the import path above as a source-layout example, not as proof that a remotely versioned Go module can currently be installed with `go get`.

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
