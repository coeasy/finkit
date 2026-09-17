# Finkit Historical Benchmark Snapshot

> **快照日期**: 2026-06-24  
> **记录环境**: Windows 10, x86_64 AVX2, Rust 2021 edition  
> **构建**: `--release` via Criterion.rs
>
> 本文件保留一组历史实测数据，便于追溯和复现；它不是当前 head 的“权威性能承诺”。任何当前竞品结论都应使用 `scripts/bench-vs-talib.sh` 或定期 `competitive-benchmark.yml` 生成的 commit-bound 证据，并同时记录 CPU、编译器、数据规模和 TA-Lib 版本。

---

## 1. 核心指标性能（10K bars，历史快照）

| 指标 | Finkit (µs) | TA-Lib C (µs) | 当次性能比 | 状态 |
| --- | ---: | ---: | ---: | :---: |
| SMA(20) | 12.75 | 20.19 | **1.58x faster** | ✅ |
| EMA(12) | 20.73 | 29.66 | **1.43x faster** | ✅ |
| RSI(14) | 26.60 | 55.12 | **2.07x faster** | ✅ |
| MACD(12,26,9) | 97.53 | 101.07 | **1.04x faster** | ✅ |
| BBANDS(20,2) | 41.74 | 56.53 | **1.35x faster** | ✅ |
| ATR(14) | 39.78 | 61.28 | **1.54x faster** | ✅ |

**历史结论**：在这一次 Windows x86_64 AVX2 快照中，上述 6 个配对指标的 Finkit point estimate 均快于当时的 TA-Lib C 对照，其中 RSI 为 2.07x。该结论只适用于这次记录，不应扩展为所有机器、所有版本或所有指标均更快。

---

## 2. 流式指标性能（历史快照）

| 指标 | 10K (µs) | 100K (µs) | 500K (µs) | ns/val (500K) |
| --- | ---: | ---: | ---: | ---: |
| SMA(20) | 22 | 220 | 2,200 | **0.44** |
| EMA(12) | 29 | 290 | 2,900 | **0.58** |
| RSI(14) | 93 | 930 | 9,300 | **1.86** |

这些数据用于观察当时的线性扩展特征；当前 Streaming 性能应重新在目标硬件上执行现行 benchmark。

---

## 3. 公式引擎性能（历史快照）

| 指标 | 原生 (µs) | 公式引擎 (µs) | 当次开销 |
| --- | ---: | ---: | ---: |
| SMA(20) | 12.75 | 16.58 | 1.30x |
| EMA(12) | 20.73 | 55.14 | 2.66x |
| RSI(14) | 26.60 | 42.82 | 1.61x |

这组数据只反映当时版本的一次测量。当前 Formula 性能重点应继续比较 parse+execute 与 compile-once/eval-many，并把计划复用、scratch reuse 和 end-to-end binding 成本纳入同一证据链。

---

## 4. 当前性能门禁应看哪里

现行性能合同不再由这份历史 Markdown 数字决定，而由代码和 CI 门禁决定：

- `core/tests/memory_regression.rs` — caller-owned hot path allocation；
- `core/tests/performance_regression.rs` — O(n) 相对复杂度、HT_SINE release throughput、DirtyRange 行级效率与 full-equivalence；
- `core/benches/talib_c_comparison.rs` — Finkit vs TA-Lib C 配对基准；
- `scripts/bench_report.py` — schema、paired-row 校验与 competitor gate；
- `.github/workflows/competitive-benchmark.yml` — 定期 TA-Lib 0.7.1 head-to-head 证据。

---

## 5. 当前推荐复现方法

```bash
# 正确性与性能回归
cargo test -p finkit --test memory_regression --release --locked -- --test-threads=1
cargo test -p finkit --test performance_regression --release --locked -- --test-threads=1

# TA-Lib C 同机配对 + 环境记录 + 可选 precision
./scripts/bench-vs-talib.sh --precision
```

有效竞品报告至少应同时保留：commit SHA、working-tree 状态、CPU/平台、Rust/Cargo、TA-Lib 版本、数据规模、参数和 paired benchmark rows。

---

## 6. 进一步阅读

- [Benchmark results](benchmark-results.md) — 当前性能合同与证据入口；
- [Finkit vs TA-Lib C](BENCHMARK_VS_TALIB.md) — 竞品基准方法与 claim rules；
- [竞品对比与超越路线](competitive-positioning-zh.md) — TA-Lib / VectorBT / Pandas TA Classic / ta-rs 的系统级对比。

**结论**：这份文件的价值是“保留原始历史事实”，而不是“把历史数字当作永久结论”。Finkit 的性能优势必须持续由当前 commit 上的可复现证据证明。
