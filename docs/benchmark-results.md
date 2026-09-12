# Benchmark Results · 性能基准

> 本页是性能证据入口，不把历史单机数字包装成永久产品承诺。完整方法见 [BENCHMARK_VS_TALIB.md](./BENCHMARK_VS_TALIB.md)，竞争策略见 [competitive-positioning-zh.md](./competitive-positioning-zh.md)。

## 当前性能合同

Finkit 将性能验证拆成四层：

1. **正确性先行**：TA-Lib parity、batch/streaming convergence、full/range equivalence；
2. **算法退化门禁**：O(n) 热路径、zero-allocation `_into`、HT_SINE release throughput；
3. **同机竞品配对**：Finkit vs TA-Lib C / Rust `ta`；
4. **产品工作负载**：Formula compile-once/eval-many、FactorPlan、DirtyRange、multi-language end-to-end。

CI 中 `Memory and performance regression` 使用 `--release --test-threads=1` 执行性能合同，避免把普通 debug unit-test 并发和共享 runner 抖动误判成实现退化。

## 当前阻断门禁

### Zero-allocation hot path

`core/tests/memory_regression.rs` 锁定 caller-owned SMA `_into` 热路径不产生堆分配。

### Relative algorithmic advantage

`core/tests/performance_regression.rs` 将 optimized rolling SMA 与故意的 O(n × period) naive reference 放在同一进程比较，防止线性 rolling kernel 回退成重复窗口求和。

### HT_SINE throughput

历史合同 `< 1000 ns/bar` 保留，但已经从普通 debug unit test 的不稳定墙钟判断迁移到：

```bash
cargo test -p finkit --test performance_regression --release --locked -- --test-threads=1
```

通过 warm-up + 多轮中位数，在正确的 optimized build 中执行绝对吞吐预算。

## TA-Lib C head-to-head

一键运行：

```bash
./scripts/bench-vs-talib.sh --precision
```

脚本现在会记录：

- commit SHA；
- dirty working tree 状态；
- OS / architecture / processor；
- Rust / Cargo；
- Python；
- TA-Lib 版本；
- Criterion paired rows。

输出到 `dist/bench/`：

- `environment.json`；
- `results.json`；
- `summary.md`；
- `finkit-vs-talib.md`；
- 可选 precision JSON/Markdown。

如果没有发现足够的 Finkit / TA-Lib paired rows，报告器支持 `--require-pairs` 直接失败，避免“没有真实数据却生成成功报告”。

## 定期竞争基准

`.github/workflows/competitive-benchmark.yml` 每周在固定 GitHub Linux runner 上：

- 安装 TA-Lib 0.7.1；
- 编译并运行真实 `talib_c_comparison` Criterion benchmark；
- 要求至少存在有效配对结果；
- 阻止超过 TA-Lib 25% 的严重退化；
- 上传 environment + report + machine-readable results artifact。

25% 是**严重退化保护线**，不是最终目标。对于高频核心 watchlist，长期目标仍然是 `Finkit <= TA-Lib`，即 `speedup >= 1.0x`。

## 历史快照如何阅读

仓库中的 `BENCHMARK_REPORT.md` 当前保存的是 **2026-06-24 Windows x86_64 AVX2** 快照。该快照记录的六个核心配对结果为：

| 指标 | Finkit | TA-Lib C | 当时 speedup |
| --- | ---: | ---: | ---: |
| SMA(20) | 12.75 µs | 20.19 µs | 1.58x |
| EMA(12) | 20.73 µs | 29.66 µs | 1.43x |
| RSI(14) | 26.60 µs | 55.12 µs | 2.07x |
| MACD(12,26,9) | 97.53 µs | 101.07 µs | 1.04x |
| BBANDS(20,2) | 41.74 µs | 56.53 µs | 1.35x |
| ATR(14) | 39.78 µs | 61.28 µs | 1.54x |

这些值只能表述为“该 commit / 该机器 / 该工具链的测量结果”，不能直接扩展成“Finkit 永远快 1.2–3.2x”。

## Benchmark 复现

```bash
# 专用 regression gates
cargo test -p finkit --test memory_regression --release --locked -- --test-threads=1
cargo test -p finkit --test performance_regression --release --locked -- --test-threads=1

# Rust ecosystem comparison
cargo bench -p finkit --bench competitive_bench --locked

# TA-Lib C paired comparison（需安装 TA-Lib C）
cargo bench -p finkit --bench talib_c_comparison --features talib-c --locked

# 构建全部 benchmark，防止 benchmark 源码腐化
cargo bench -p finkit --no-run --locked
```

## 接下来真正需要优化的性能面

单指标微基准只是一部分。下一阶段必须持续补齐：

- Formula parse+eval vs compile-once+eval-many；
- FactorPlan discovery vs planned execution；
- Factor full vs DirtyRange range vs range-into；
- 100K / 1M rows 下 recomputed rows 比例；
- BufferArena / scratch allocation；
- Python NumPy / Node TypedArray / C ABI 的 end-to-end binding 成本；
- 多指标 compute-many 公共中间结果复用。

Finkit 的性能目标是：**在等价正确性下，让一次计算足够快，让重复计算越来越便宜，让局部数据变化只付局部成本。**
