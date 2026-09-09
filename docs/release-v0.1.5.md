# v0.1.5 release checklist

Version `0.1.5` is the canonical workspace version across Rust, Python, Node,
Java, .NET, CMake, Cargo.lock, generated catalogs, and release-facing docs.

## Included changes

- Cross-market screening indicators and formula aliases;
- generated indicator catalog updated with the `screening` module;
- generated formula catalog updated with the routed functions;
- TA-Lib benchmark report updated to the 155-case/310-observation wheel gate;
- versioned multi-language packaging scripts and installation guidance;
- release workflow covers Node, Java, C/C++, Go, .NET, WASM, Android, iOS,
  Rust, CLI, Python, and checksums.

## Validation matrix

| Target | Local validation on Windows | Release workflow |
| --- | --- | --- |
| Rust core | release dependencies compiled; crate packaged with `--offline` | Linux crate/package gate |
| Python | `finkit-0.1.5` ABI3 wheel built and TA-Lib gate run | Linux/macOS/Windows wheel matrix |
| Node.js | native build, 2 tests, root and Windows x64 `.tgz` packages | Linux/Windows/macOS package matrix |
| Go/CGO | Rust DLL built; consumer test blocked by missing `gcc` | Linux CGO + external-module smoke |
| Java/JNI | Rust JNI DLL built; Maven JAR blocked by missing `mvn` | Linux JAR/resource/JVM smoke |
| C/C++ | Rust FFI DLL built; CMake SDK blocked by missing `cmake` | Linux CMake SDK package gate |
| .NET | Rust P/Invoke DLL built; NuGet blocked by missing `dotnet` | Linux/Windows/macOS RID gates |
| WASM | blocked by unavailable `wasm32-unknown-unknown` std mirror | real wasm32 web/node/bundler gate |
| Android/iOS | not host-buildable on this Windows checkout | dedicated hosted runner gates |

## Release commands

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
cargo test --workspace --locked
```

The tag must match the workspace version exactly:

```bash
git tag -a v0.1.5 -m "Release v0.1.5"
git push origin v0.1.5
```

The tag triggers the multi-language release workflows. They create or update
the GitHub Release, attach validated supplemental artifacts, and generate
`SHA256SUMS.multilang`. Public registry publication remains a separate,
explicit operation and should only be documented after clean-install smoke
tests pass in that registry.
