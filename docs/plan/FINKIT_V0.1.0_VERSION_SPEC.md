# Finkit v0.1.0 Version Specification

## Canonical Version

```text
0.1.0
```

Git tag:

```text
v0.1.0
```

## Rule

Finkit 采用单一 release train。同一次 GitHub Release 中，所有源码 workspace、语言 package metadata、native package metadata 和运行时版本必须使用相同 SemVer。

v0.1.0 对齐要求：

| Surface | Required version |
|---|---:|
| Rust workspace | 0.1.0 |
| Cargo.lock local workspace crates | 0.1.0 |
| Python distribution `finkit` | 0.1.0 |
| Python `finkit.__version__` | 0.1.0 |
| Node main package | 0.1.0 |
| Node native packages | 0.1.0 |
| Java Maven artifact | 0.1.0 |
| .NET NuGet artifact | 0.1.0 |
| GitHub tag | v0.1.0 |

Go、C、WASM、Android、iOS 等 Rust workspace component 继承 workspace `0.1.0`；没有独立 package manifest 的发行面以 Git tag `v0.1.0` 为准。

## Compatibility Names

v0.1.x 允许下列历史名称作为内部/兼容名称继续存在：

- `alpha-ta-core`
- `alpha-ta-*` Rust crates
- Python native module `alpha_ta`
- Node `@alphata/*`
- Maven `com.alphata:alpha-ta`
- NuGet `AlphaTA`

它们不代表不同版本线，全部属于 Finkit v0.1.0。

公共 Python API 和 CLI 从 v0.1.0 开始使用：

```text
finkit
```

## Automated Enforcement

CI 必须执行：

```bash
python scripts/check_version_alignment.py
```

检查失败时禁止合并和创建 release。

## Version Bump Procedure

未来升级版本时必须在同一个 PR 内完成：

1. 更新 workspace canonical version
2. 更新各语言 package manifest
3. 更新 Python runtime `__version__`
4. 刷新 Cargo.lock
5. 更新 release workflow/tag contract
6. 运行 version-alignment CI
7. 全量安装 smoke test
8. 合并后再创建对应 Git tag

禁止先打 tag、后补版本文件。
