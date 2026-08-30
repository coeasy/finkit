# AlphaTA 优化改进计划

**版本**: v1.0
**日期**: 2026-07-12
**适用范围**: `alpha-ta-core` (Rust 指标库) 执行效率、内存占用、数值准确性。

---

## 0. 现状基线 (事实)

| 项 | 当前状态 |
|----|----------|
| 工作区 | 存在**大量未提交的 Phase 3 WIP** (momentum.rs +1257 行、cycle.rs +1240、streaming 重构为子目录 144 文件)。需先固化基线。 |
| 单测 | `cargo test -p alpha-ta-core --release --lib` → **2598 通过 / 0 失败**。 |
| TA-Lib golden | `core/tests/golden/talib/` **为空** → 所有 `golden_talib_*` 测试**跳过** (无参照数据)。 |
| Python+talib | 本机**无可用 numpy+talib 环境** (Store 占位符 stub)；`scripts/gen_talib_golden.py`、`bench_alpha_vs_talib_python.py` 无法运行。 |
| 性能对比 | 平均加速比 ~1.61x；优于 TA-Lib 32/43 (74.4%)；**10 个指标 < 1.0x**。 |
| 数值准确 | 25/43 (58.1%) 与 TA-Lib 一致；**18 个指标不匹配**。 |

### 仍需优化的慢指标 (vs TA-Lib)

| 指标 | 当前加速比 | 实现现状 | 优化点 |
|------|-----------|----------|--------|
| ADOSC | 0.73x | AD 线写满 scratch buffer，再双 EMA 两次遍历 | 融合为单遍；消除 scratch |
| APO | 0.86x | 2 次全量 `sma()` + 1 次差值 = 3 分配 | 合并为单遍运行 SMA，1 次分配 |
| KAMA | 0.89x | 已单遍自适应 O(1) | 微调，接近即可 |
| MINUS_DI | 0.91x | 已有 `compute_di_only` 快路径 | 确保长输入走快路径 |
| WMA | 0.93x | 有 `wma_simd` (AVX2 前缀和) | 默认走 SIMD 路径 |
| PLUS_DI | 0.93x | 同 MINUS_DI | 同上 |
| AD | 0.94x | 已 SIMD `simd_ad_line` | 保持 |
| MACD | 0.97x | FMA EMA 递推，已较优 | 保持，微调 |

### 数值不匹配指标 (18 个)

`MAMA, T3, ADX, DX, MACD, MINUS_DI, PLUS_DI, TRIX, AD, ADOSC, ATR, VAR, HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDLINE, HT_TRENDMODE`

---

## 1. 目标与验收指标 (KPI)

| 维度 | 当前 | 目标 | 验收方式 |
|------|:----:|:----:|----------|
| 平均加速比 vs TA-Lib | ~1.61x | **≥ 2.0x** | `bench_alpha_vs_talib_python.py` |
| 优于 TA-Lib 比例 | 74.4% | **≥ 90%** | 同上 |
| 最低加速比 | 0.45x (AROON) | **≥ 0.9x** | 同上 |
| 数值准确率 | 58.1% | **≥ 90%** | `golden_talib_*` 全绿 + 差分脚本 |
| 慢指标清零 | 8 个 <1.0x | **全部 ≥ 1.0x** | 同上 |
| TA-Lib golden | 空 | **22 指标 JSON 已提交** | `core/tests/golden/talib/*.json` 存在 |
| 单测全绿 | 2598 | 稳定 ≥ 2600 | `cargo test` |

---

## 2. 阶段划分与优先级

> 原则：**测量先行、基准确立、再微调**。先解决"无法验证"这个最大风险，再做优化，最后做精度。

---

### Phase 0 — 基线治理与测量基础设施（P0，前置，阻塞后续）

**目的**：建立可重复的性能/精度测量基线；否则无法判断任何优化是否有效。

| # | 任务 | 产出 | 负责 |
|---|------|------|------|
| 0.1 | **提交当前 WIP**，锁定工作区基线 (momentum/cycle/statistics/volatility/streaming 重构) | 干净 git 基线 | 全组 |
| 0.2 | 搭建可用 Python 环境：安装 **numpy + talib (0.6.x)** (pyenv/pip/conda 任一)，或提供夜间文档 | `bench/` 可跑 | 环境 |
| 0.3 | 运行 `scripts/gen_talib_golden.py` 生成 `core/tests/golden/talib/*.json` (22 个指标) | JSON 入库 | 数据 |
| 0.4 | 恢复主基准脚本至 `scripts/bench_alpha_vs_talib_python.py` (已覆盖 158 函数) | 基准入口 | 数据 |
| 0.5 | 建立 CI 门禁：性能回归阈值 + 精度回归阈值 (参考 `.github/scripts/check_memory_regression.py`) | CI | 设施 |

**验收**：`golden_talib_*` 不再 skip；`bench*` 能在 CI 跑出与新增阈值对比的数字。

---

### Phase 1 — 性能优化（逐个 <1.0x 指标清剿）(P1)

> 复用已验证的 `core/examples/bench_before_after.rs` 同二进制对比法，保证数值零差异。

| # | 指标 | 做法 | 预期 | 文件 |
|---|------|------|------|------|
| 1.1 | **APO** | 用运行缓冲一遍算出 fast/slow SMA 并直接差分出结果，1 次分配（现 3 遍） | 0.86x → **≥1.2x** | `momentum.rs:1407` |
| 1.2 | **ADOSC** | AD 线与 fast/slow EMA 融合为单遍，去掉全量 scratch buffer | 0.73x → **≥1.1x**，省 ~2×len×8B | `volume.rs:110` |
| 1.3 | **WMA** | 默认调用已存在的 `wma_simd` (AVX2)；标量保留 fallback | 0.93x → **≥1.1x** | `moving_avg.rs:513/655` |
| 1.4 | **MINUS_DI / PLUS_DI** | 恒走 `compute_di_only` 快路径（现仅在 len≥2× 走），复用 ADX 家族 | 0.91/0.93x → **≥1.0x** | `momentum.rs:1713/1772` |
| 1.5 | **KAMA** | 微调；确认 Efficiency Ratio 缓存无冗余除法 | 0.89x → **≥1.0x** | `moving_avg.rs:974` |
| 1.6 | 高位指标 `ema/sma/wma_into` 覆盖，补零分配 `_into` 变体 API | 提供零拷贝接口 | 跨文件 |
| 1.7 | 重新生成 benchmark 报告 (Criterion + Python 双口径) | 更新 `ALPHATA_VS_TALIB_COMPARISON_REPORT.md` | - |

**验收**：`#1.1–1.6` 每个在同二进制中实测加速比随机区提升，且 `max|diff|==0` 或误差 <1e-12。

---

### Phase 2 — 数值准确性提升（18 个不匹配 → ≥90%）**(P1)**

**目的**：逐指标对齐 TA-Lib 语义（种子、warm-up、tie-break、Wilder 平滑、NaN 映射）。

| # | 指标 | 常见根因 | 工具 |
|---|------|----------|------|
| 2.1 | **ATR / TRANGE** | TR 起始/warm-up 种子差异 | `scripts/debug_atr_impl.py` |
| 2.2 | **MACD / TRIX / APO** | SMA 种子偏移、signal 起点 | `investigate_talib_macd.py`、`test_macd_*` |
| 2.3 | **ADX / DX / +DI/-DI** | Wilder 平滑起始、RMA 种子 | `debug_warmup.py` |
| 2.4 | **VAR** | 总体/样本方差 (n vs n-1)、`bias` 参数 | `statistics.rs:1064` |
| 2.5 | **AD / ADOSC** | 累加舍入顺序、NaN 起始 | 差分归零后对比 |
| 2.6 | **MAMA / T3** | 系数/种子差异 (T3 已知) | 精度诊断脚本 |
| 2.7 | **HT_* (7 个)** | 周期估计、翘曲相位实现差异 | `analyze_precision_issues.py` |
| 2.8 | 全部指标 warm-up 对齐 | golden `null` ↔ AlphaTA `NaN` 严格一致 | `gen_talib_golden.py` |

**通用方法**：
1. 用 `scripts/diagnose_accuracy.py` / `full_accuracy_diagnose.py` 逐指标 dump max/mean abs-diff。
2. 单指标单元测试 + 差分调试脚本 (已有多个 `debug_*`)。
3. 每个修复**先写失败的单测**再改，杜绝回归。
4. 修复后**重新生成该指标 golden JSON**。

**验收**：`golden_talib_*` 全绿；`bench_vs_talib_precision.py` 通过率 ≥90%。

---

### Phase 3 — 内存占用优化 **(P2)**

| # | 位置 | 现状 | 目标 |
|---|------|------|------|
| 3.1 | 所有多遍指标 (APO/ADOSC/MACD/ADX) | 全量中间数组 | 单遍融合，零中间分配 |
| 3.2 | 补齐 `*_into` 零拷贝变体 | 部分缺失 | 全指标覆盖 |
| 3.3 | 复用已 SIMD 的 AD/OBV/WMA 内核 | - | 归零中间分配 |
| 3.4 | `check_memory_regression.py` 纳入 CI | - | 内存分配数回归门禁 |

**验收**：基准脚本内的 allocation 计数（valgrind / allocation counting）下降，10K 点各指标中间分配 ≤1。

---

### Phase 4 — 自动化、报告与治理 **(P2)**

| # | 任务 | 产出 |
|---|------|------|
| 4.1 | CI：性能 + 精度 + 内存分配三合一门禁 | 每次 PR 自动跑 |
| 4.2 | `cargo bench`(Criterion) 全指标，统计显著提升 | `target/criterion` 报告 |
| 4.3 | 自动更新 `ALPHATA_VS_TALIB_COMPARISON_REPORT.md` | 一键报告 |
| 4.4 | 建立指标优化对比登记表 (owner/基线→目标/状态) | 见 §5 |

---

## 3. 执行顺序建议

```
Phase 0 (基线/环境) ──► Phase 1 (性能 1.1-1.6)
                          │
                          ▼
                Phase 2 准确性 (2.1-2.8)
                          │
                          ▼
        Phase 3 内存 ──► Phase 4 自动化/报告
```

> 说明：Phase 1 与 Phase 2 可并行两条线；但**每个指标必须先过精度门禁再谈性能**（避免"快但错"）。

---

## 4. 风险与依赖

| 风险 | 影响 | 缓解 |
|------|------|------|
| 无 numpy+talib 环境 | golden 与精度流程全阻塞 (P0) | 先布局此依赖；或用 Rust `ta` crate 作临时参照 |
| 工作区 WIP 未提交 | 回滚/协作混乱 | Phase 0.1 优先 |
| SIMD 路径在窄平台回退 | 加速不一致 | 保留标量 fallback |
| 浮点累加顺序改变 | 精度抖动 | 融合重构期间开启精度门禁 |
| TA-Lib 语义差异 (Wilder 等) | 精度目标难以 100% | 以官方 C 源码注释为准，`max-diff≤1e-6` 为达标 |

---

## 5. 指标优化对比登记表

| 度量 | 基线 (前) | 目标 | 实测 | 状态 | 负责人 |
|------|:--:|:--:|:--:|:--:|:--:|
| 平均加速 | 1.61x | ≥2.0x | - | 进行中 | |
| 优于 TA-Lib | 74.4% | ≥90% | - | - | |
| 最低速比 | 0.45x (AROON) | ≥0.9x | - | ✅ 已修复窗口 | |
| 数值准确 | 58.1% | ≥90% | - | 需 Python 环境 | |
| APO | 0.86x | ≥1.2x | **4.9x** (0 diff) | ✅ P1.1 单遍 | |
| ADOSC | 0.73x | ≥1.2x | 0.53x | ⚠️ 融合回归 SIMD，已回退 | |
| WILLR | 0.34x | ≥0.9x | **1.7x** (0 diff) | ✅ | |
| AROON 批 | 0.35x | ≥0.9x | **2.1x** (0 diff) | ✅ | |
| AROON 流式 | 偏差 | 对齐 | 0 diff | ✅ 窗口修复 | |
| WMA | 0.93x | ≥1.0x | 已 O(1) 增量 | (skip, 风险>收益) | |
| KAMA | 0.89x | ≥1.0x | - | 待定 | |
| +DI/-DI | 0.92x | ≥1.0x | - | 待定 | |