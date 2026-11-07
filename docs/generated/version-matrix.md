# Version Matrix

> **SSOT** — auto-generated from workspace `Cargo.toml` and binding manifests (including .NET and Java metadata).
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Canonical workspace version: **`0.3.1`**

| Package | Path | Version | Source | Match |
|---------|------|---------|--------|-------|
| workspace | `Cargo.toml` | 0.3.1 | workspace.package | ✅ canonical |
| qx-finkit | `core/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-factor-analysis | `factor-analysis/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-visualization | `visualization/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-ffi | `ffi/c-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-ffi-common | `ffi/ffi-common/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-python | `ffi/python-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-node | `ffi/node-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-go | `ffi/go-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-dotnet | `ffi/dotnet-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-ios | `ffi/ios-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-java | `ffi/java-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-android | `ffi/android-binding/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-cli | `cli/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-wasm | `wasm/Cargo.toml` | 0.3.1 | workspace | ✅ |
| finkit-python (pyproject) | `ffi/python-binding/pyproject.toml` | 0.3.1 | project.version | ✅ |
| finkit-node (package.json) | `ffi/node-binding/package.json` | 0.3.1 | version | ✅ |
| finkit-dotnet (.csproj) | `ffi/dotnet-binding/src/Finkit/Finkit.csproj` | 0.3.1 | Version | ✅ |
| finkit-java (pom.xml) | `ffi/java-binding/pom.xml` | 0.3.1 | project.version | ✅ |

## Benchmark data

Benchmark statistics are deliberately **not** indexed here. The
number of Criterion JSON files on one machine is not a fact about
the repository, so recording it would make this file disagree
between a developer checkout and CI. Generate the report locally
instead:

```bash
cargo bench -p qx-finkit --locked
python scripts/gen_benchmark_report.py   # → docs/BENCHMARK_REPORT.md
```

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check
```
