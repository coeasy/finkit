# Benchmark Results · 性能基准

> 完整方法与逐指标数据见 [BENCHMARK_REPORT.md](./BENCHMARK_REPORT.md) 与
> [BENCHMARK_VS_TALIB.md](./BENCHMARK_VS_TALIB.md)。本页为对照 TA-Lib 的简明汇总。

## 结论速览

- 核心指标相对 TA-Lib C 整体快 **1.2x–3.2x**（真实 FFI 调用，非内存内纯函数）。
- 流式接口为 **O(1)/bar**，支持 1M 根 bar 级别实时推送。
- 公式引擎（`FormulaEngine`）带真 LRU 编译缓存，缓存命中可提速约 **23x**。
- SIMD（AVX2/AVX-512/WASM）对批量 `add` / `mul` / `sma` 等指令可达 5–37x 的吞吐提升。

## 典型对照（概数）

| 指标 | Finkit vs TA-Lib | 说明 |
|------|-------------------|------|
| SMA / EMA / WMA | 1.5x–2.6x | 零分配 + SIMD 求和 |
| MACD | 1.3x–2.0x | 多周期 EMA 单趟 |
| RSI / CMO | 1.2x–1.8x | FMA 归一化 |
| ATR / TRANGE | 1.5x–2.2x | 流式滚动 min/max |
| 线性回归族 | 2.0x–3.0x | 1M 规模门禁用例 |

## 复现

```bash
cargo bench --workspace --bench talib_comparison_bench
cargo bench -p finkit --bench zero_alloc_bench     # 零分配验证
cargo bench -p finkit --bench memory_profile --features memory-profile
```

目录说明：

- `core/benches/`：Criterion 基准源码（含 harness=false 的自定义测量）。
- `target/criterion/report/`：本地 HTML 报告（gitignore，不入库）。
- benchmark 构建 profile 使用 `lto="fat"` + `target-cpu=native`，测得的是**本机**单线程极限吞吐，不同机器数值会不同。