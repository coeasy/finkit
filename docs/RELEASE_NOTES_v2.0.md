# Release Notes v2.0

> AlphaTA（finkit）核心为 Rust 1.0.0，本页说明自 v1.x 系列起的 v2.0 变更。
> 逐版本变更记录见 [CHANGELOG.md](../CHANGELOG.md)。

## 版本信息

- Crate 版本：`1.0.0`；MSRV 1.75；许可证：MIT OR Apache-2.0。
- 仓库身份：`https://github.com/coeasy/finkit`（统一了先前分散的 URL 引用）。

## Highlights

- **核心库**：150+ 批量指标 + 98 个流式 O(1) 指标 + 60+ K 线形态 / 15+ 图表形态。
- **公式引擎**：parser→AST→bytecode→JIT→SIMD，多方言（TDX 100% / THS 96.3% / DZH 100% / 文华 90% / Pine ~60%），309 个模板，真 LRU 缓存。
- **ML 特征工程**：`features` 模块（pca/wavelet/fourier/garch/regime/meta_labels/importance/selection）。
- **多语言绑定**：Python、Node、Go、Java、.NET、C/C++、iOS、Android、WASM 一核心多入口。
- **性能**：相对 TA-Lib 1.2x–3.2x；SIMD 指令级 5–37x；零分配热路径。

## Breaking / 注意

1. 仓库 URL 统一为 `coeasy/finkit`（见 [MIGRATION.md](./MIGRATION.md)）。
2. FFI（C ABI）函数统一 `ta_*` 前缀，需配对释放。
3. 绑定产物结构调整为 `dist/{python,java,node,go,c,dotnet}`。

## 后续规划

- 完整 CI 套件（release/perf-gate/fuzz/docs 工作流）落地。
- 各绑定独立演进发布号。