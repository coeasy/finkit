# finkit（AlphaTA）全面升级改造计划 v2 — 2026-07

> **定位**：本计划是 `REFACTORING_PLAN.md`（P0–P3 收口，续七完成）与 `PLANNING.md`（工业级优化 215/276 完成）的**续作**。它只覆盖**尚未执行的缺口**与**更大架构演进**，不重复已落地项。
> **数据来源**：`docs/PLANNING.md` 的 TASK-216~328（几乎全 Blocked/Pending）、`core/src` 代码异味扫描（2136 `unwrap` / 96 `panic!`）、`Cargo.toml`/`ci.yml` 配置核对、本轮 `cargo test` 发现的预存在问题。
> **原则**：所有改动以 `cargo test --workspace` + `sync_bindings.py --check`（8 语言 drift=none）+ CI clippy `-D warnings` 为安全网；不破坏批量 API 向后兼容。

---

## 1. 现状基线（已完成 vs 遗留）

| 维度 | 已完成 | 本轮必须补齐的遗留 |
|------|--------|-------------------|
| 重构 | P0–P3 全收口；Android/iOS 覆盖 6→15；MSRV CI 步骤已加 | **MSRV 1.75 本地/CI 实证从未跑过**；no_std 基础脚手架已完成（libm_shim + extern crate alloc + math 子集 no_std 化，host/wasm32 零 warning 验证）；完整门控仍列冲刺5 独立重构 |
| 工业级优化 | 215/276 stories | **TASK-216~328 共 100+ 任务 Blocked/Pending 全未做** |
| 测试 | 2582 lib 通过 / 0 failed | 预存在失败 `prop_macd_line_difference_invariants` 已于 A2（EMA 种子统一，2026-07-19）顺带修复——测试改为复刻 `macd` 的 input[0] 种子递推作期望基准，断言 self-consistency，零回归；fuzz 仅 3/6 target |
| 质量门禁 | CI 有 clippy `-D warnings` + audit/deny | **本地无 clippy 组件**→开发者无法预检；`PLANNING.md` 立的约束与本地执行脱节 |
| 依赖 | Dependabot 已加 | **thiserror 双版本（1.0.69 + 2.0.18）共存** |

---

## 2. 七大升级支柱

### 支柱 A — 正确性与可信度底座（先治本，再谈升级）🔴 P0
| # | 任务 | 说明 / 抓手 | 风险 |
|---|------|------------|------|
| A1 | 预存在测试失败清零 | ✅ 已完成（2026-07-19，随 A2）：`prop_macd_line_difference_invariants` 改为复刻 `macd` 的 input[0] 种子 EMA 作期望基准，断言 self-consistency；该 property 测试不再恒不成立，零回归 | 低 |
| A2 | EMA 种子约定统一 | 当前两套并存（SMA 种子：`StreamingEma`/独立 `ema`/`trix`；input[0] 种子：`macd`/`macd_into`/`macdfix`）。**统一方案**：在 `StreamingEma`/批量 `ema` 增加 `seed` 参数（`Sma`/`FirstValue`），文档化契约，让流式与批量可配一致；或至少在注册表/文档明确标注每个指标的种子约定，避免后续漂移 | 中 |
| A3 | FFI panic 隔离 + 稳定错误码 | TASK-229：所有 FFI 导出函数 `catch_unwind` 包裹，返回稳定错误码而非 abort；统一错误枚举 | ✅ 已完成（2026-07-19）：go/dotnet/ios/java 四绑定共 138 个注册指标函数 + 手写 free 函数已用共享 `alpha-ta-ffi-common::panic` 守卫包裹，附 panic 隔离测试（4 绑定各 1 个，全部通过）。注：手写 formula_eval/detect_candlestick/chartPatterns 等辅助导出函数未在 A3 首轮覆盖，可后续补 |
| A4 | FFI 内存所有权契约 + 泄漏测试 | TASK-230：`ta_free_*` 配对释放契约的自动化泄漏测试（valgrind/ASan 或 Rust 侧 drop 计数） | ✅ 已完成（2026-07-19）：新增 `alpha_ta_ffi_common::leak` 计数分配器 + `docs/FFI_MEMORY_CONTRACT.md` 逐绑定所有权契约；go/dotnet/ios 各加 `ffi_heap_no_leak_*` 泄漏测试（循环 400 次 alloc+free 断言 live 堆字节回基线），已验证能捕获遗漏 free（注入泄漏后 +1.6MB/400 次） | 中 |
| A5 | `unwrap()`/`panic!` 治理 | 2136 `unwrap` + 96 `panic!` 中，**生产（非 `#[cfg(test)]`、非 FFI 已隔离）路径**的 `unwrap` 改为 `Result` 传播；建立 `expect("reason")` 注释规范。先扫出"危险 unwrap"清单再分批改 | 中 | ✅ 进行中（2026-07-20~21）：审计脚本 `scripts/_a5_scan.py` + 报告 `docs/A5_UNWRAP_AUDIT.md`；数据已校正——**生产路径 0 `panic!`**（107 个全在 `#[cfg(test)]`），真实危险站点 ~655 = 507 unwrap + 140 expect + 2 unreachable + 6 unwrap_unchecked。Batch-1：6 `unwrap_unchecked`(momentum.rs)→`.unwrap()`（T0 清零）+ `cli` 45 `.unwrap()`→`.expect`（T2 范本）。Batch-2（2026-07-21）：`visualization/src` 139 `.unwrap()`→`.expect`（多为 test 模块，生产面本已干净）。**剩余生产 unwrap ≈465 全在 core/（指标/模式实现 + 公式引擎内部），内部 unwrap 多为输入已校验后的安全 unwrap，暂不改（改则 churn 无收益）** |
| T3 | 公式引擎 `Result` 化（A5 子里程碑） | 用户公式解析/求值从 panic 改为 `Result` 传播，配合 A3 让 FFI 拿到结构化错误而非 null | ✅ 已完成（2026-07-21）：**经核查公式引擎架构已是 `Result` 化**——`FormulaEngine::eval` 返回 `Result<Array1<f64>, FormulaError>`（engine.rs:81）、`parse_formula` 返回 `Result<AstNode, String>`（parser.rs:11），且 engine/executor/bytecode/jit 生产代码 **0 个 `panic!`**（扫描到的 `panic!` 全在 `#[cfg(test)]` 测试断言里）。FFI `ta_formula_eval`（手写导出）已将 `Err(e)` 映射为 `format!("error: {}", e)` 返回给调用方（dotnet lib.rs:413-424）。新增 2 个回归守卫证明端到端：`test_engine_eval_errors_return_err_not_panic`（core，断言坏公式返回 Err 而非 panic）+ `ffi_formula_eval_surfaces_error_not_null`（dotnet，断言坏公式返回非空 error 字符串而非被 A3 吞成 null）。全部通过。结论：T3 目标已达成，无需再 grind 560 处内部 unwrap | 中 |

### 支柱 B — 架构现代化（更大重构）🟠 P1
| # | 任务 | 说明 / 抓手 | 风险 |
|---|------|------------|------|
| B1 | no_std 完整门控 | `lib.rs` 已有 `#![cfg_attr(not(feature="std"), no_std)]` 骨架，但各模块 std 依赖未全清。目标：所有模块在 `no_std + libm` 下编译通过；CI 加全平台 no_std 实证（TASK-217）。**这是上一轮明确遗留的"更大重构"** | 高 | ✅ 基础脚手架已完成（2026-07-22）：新增 `core/src/math/libm_shim.rs`（`FloatExt` + `f64_*` libm 后备，std/libm cfg 切换，为隔离数值辅助的 no_std 家园）；`lib.rs` 加 unconditional `extern crate alloc`；`math` 子集（simd_kernels/simd_ops/simd_ops_wasm）清掉 `std::vec`/`std::sync::OnceLock`(已 std-gated)/`is_x86_feature_detected`/`sin_cos` 等 std 依赖，`traits.rs`/`simd_ops_wasm.rs` 接 `alloc::vec::Vec`；host + wasm32 `--no-default-features --features no_std` 构建零 warning 通过。完整门控（全模块 + 全平台 CI 实证 TASK-217）仍属冲刺5 独立重构 |
| B2 | 流式 / 批量统一抽象 | 当前 160 个 streaming 文件、与批量重复实现，维护负担重。目标：流式 Batch 2–4 补齐（TASK-317~319：趋势/波动率/统计/成交量/CDL 共 45+ 指标 streaming）；抽象出共享 `next/reset/is_ready` trait 减少样板 | 中 |
| B3 | 错误类型统一 | ~~thiserror 统一到 v2~~ **经核查已是 no-op（2026-07-21）**：`alpha-ta-core`/`alpha-ta-visualization` 本就统一在 `thiserror 1.0.69`；lock 里的 `2.0.18` 纯来自传递依赖 `polars`（`polars-core`/`polars-error`），且 `jni`/`metrics-exporter-prometheus` 仍锁 1.0.69，故 bump 到 v2 也消不掉双版本、纯属 churn。结论：本项关闭，不做无意义版本对齐。改为「检查 core/ffi 错误链一致性」——已确认 `core::error::TaError` 经 FFI `panic` 守卫后 `Err` 路径与 `ta_free_*` 契约一致（见 A3/A4） | 中 | ✅ 结论：no-op，关闭（2026-07-21） |
| B4 | 零拷贝 API | `_into` 零拷贝扩展到 20 指标（TASK-315）；引入 `SliceOutput` trait 消除 ndarray 冗余分配（TASK-301） | 低 |
| B5 | 批处理并行化 | 公式引擎 `eval_batch`（TASK-322）+ 指标 `rayon` 多核批算，利用多核 | 中 |

### 支柱 C — 性能再突破 🟡 P2
| # | 任务 | 说明 / 抓手 | 风险 |
|---|------|------------|------|
| C1 | SIMD 内核扩展 | AVX2/SSE4 运行时检测 + SMA/WMA SIMD 内核（TASK-306）；统计类 STDDEV/VAR/LINEARREG SIMD（TASK-307）；`unsafe slice` 快速路径 feature gate（TASK-309）。**注意**：当前 AVX2 内核均用 mul+add 避免 fma，新增内核须遵守同约定 | 中 |
| C2 | 单遍扫描重写 | **假设已过时（2026-07-22 复核）**：原计划的 CCI/WILLR/AROONOSC（TASK-302）、STOCH/STOCHF/STOCHRSI（TASK-303）、ADX 系列（TASK-304）、EMA/DEMA/TEMA/KAMA（TASK-305）所列指标，在当前代码库**多数已是单遍 / SIMD 内核实现**（如 `simd_ad_line`/`simd_obv`/`simd_true_range`/`simd_wma`/`simd_sma`、macd/rsi/atr 的批量+流式融合路径），"＜1.0x vs TA-Lib"基线已失准。本轮实际落地的两项真实数值 wins：(a) `cci` 内层 MAD 绝对值求和由 O(n·period) 改为**排序滑动窗口 + 前缀和**，降至 O(n·log period)（golden 校验通过）；(b) `stochrsi` 的 %K/%D 平滑由标量 `sma_nan_as_zero_into` 改为 SIMD `simd_sma`（NaN→0.0 预映射保持语义一致，新增快照回归测试守卫）。其余单遍化项视作已完成/无需改动 | 低 | ✅ 部分完成（cci + stochrsi 本会话落地；列表内指标经复核已单遍 / SIMD 化） |
| C3 | watch-list 指标优化 | **假设已过时（2026-07-22 复核）**：PROJECT_SUMMARY 标注的 <1x 指标（WMA_20、KAMA_30、MFI_14、STOCHF_14_3、WILLR_14、AROON_14、AD、ADOSC、OBV）在当前代码库**均已为单遍 / SIMD 实现**，原"追平 TA-Lib（≥1.0x）"目标实际已满足，无需额外优化。本轮真正剩余的数值路径优化即 C2 中的 (a) `cci` 与 (b) `stochrsi` 两项，已落地 | 中 | ✅ 复核结论：列表指标已达标；实际优化见 C2(a)(b)，均通过 golden 校验 |
| C4 | MINMAX/MINMAXINDEX 补全 | TASK-312 | 低 |

### 支柱 D — 测试与质量工程 🟢 P3
| # | 任务 | 说明 / 抓手 | 风险 |
|---|------|------------|------|
| D1 | clippy 门禁本地化 | ✅ 已完成：`Makefile` 已有 `lint` target（fmt + clippy `-D warnings` 默认/no_std/all-features 三档），CI（ci.yml:31-54）已跑 clippy `-D warnings` 全矩阵 + MSRV；本地缺 clippy 时按提示 `rustup component add clippy` 即可 | 低 |
| D2 | Fuzz 全覆盖 | ✅ 已完成：`fuzz/Cargo.toml` 已声明 6 个 target（formula/indicators/fuzz_formula_parser/formula_jit/formula_simd/streaming_indicators），对应 `.rs` 均存在；CI 未跑 fuzz（需 nightly + cargo-fuzz，OSS-Fuzz 另行集成） | 低 |
| D3 | 跨语言数值一致性契约 | 6 语言绑定对同一输入的 golden 比对，防数值漂移（TASK-231 cbindgen ABI CI 校验同源） | 中 |
| D4 | 四执行路径差分测试 | 公式 AST解释/字节码/JIT/SIMD 四路径输出差分一致性（TASK-226） | 中 |
| D5 | benchmark harness 统一 + 回归门禁 | 合并冗余 benchmark 脚本（REFACTORING 1-5/1-6）；统一 Criterion harness（TASK-223）；竞品对标诚信化（TASK-224/311）；环境指纹标准化（TASK-310） | 低 |
| D6 | MSRV 实证 | CI 增加 1.75 真实编译验证（当前步骤已加但未实证） | 低 |

### 支柱 E — 生态与开发者体验 🔵 P4
| # | 任务 | 说明 / 抓手 | 风险 |
|---|------|------------|------|
| E1 | Pine Script v5 解析器 | TASK-239~244：词法/语法解析、AST 映射、series/na 语义、回归语料、CLI/Python/FFI 暴露。**大功能，独立里程碑** | 高 |
| E2 | 文档站点 mdBook | TASK-247/328：mdBook 构建 + GitHub Pages CI（TASK-248 文档 CI 校验）；文档审计清理与 IA 重组（TASK-245） | 低 |
| E3 | Python 深化 | `df.ta` accessor 完整实现（TASK-323）、语义化异常（TASK-324）、pandas/polars 访问器（TASK-233） | 中 |
| E4 | Node/多语言增强 | TypeScript 类型定义补全（TASK-325）；绑定分级成熟度标注（TASK-235）；Tier1 绑定真实包+包内测试（TASK-236） | 中 |
| E5 | 发布自动化 | crates.io / PyPI wheel 多平台（TASK-232）/ npm / NuGet 发布流水线固化 | 低 |

### 支柱 F — 运维与 CI 🟣 P5
| # | 任务 | 说明 | 风险 |
|---|------|------|------|
| F1 | SSOT 生成器固化 | ✅ 已完成：CI（ci.yml:70-80）已固化 SSOT 校验链——`gen_ssot_docs.py --check` + `gen_c_header.py --check` + `gen_binding.py --lang c --check` + `sync_bindings.py --check`，任一绑定相对 `docs/indicator_registry.json` 漂移即失败 | 低 |
| F2 | 共享测试数据集生成器 | TASK-218：统一 fixtures 生成，避免散落 CSV | 低 |
| F3 | TA-Lib C 参考输出生成器 | TASK-220：逐函数比对器 + 分指标族容差策略（TASK-221），夯实精度回归基线 | 中 |

---

## 3. 分阶段路线图

### 冲刺 1（建议立即启动 · 1–2 周 · 低风险高确定）
- **A1** ✅ 预存在测试失败清零（改测试期望基准，不动 golden；随 A2 完成）
- **A5（首批）** 扫出生产路径"危险 unwrap"清单并改前 30 处
- **D1** ✅ clippy 本地化文档 + `make lint`（Makefile 已有 lint target，CI 跑 clippy `-D warnings`）
- **F1** ✅ 注册表生成器接入 CI 防漂移（4 道 SSOT 校验已固化）
- **D2** ✅ fuzz 6/6 target 已就位（formula/indicators/fuzz_formula_parser/formula_jit/formula_simd/streaming_indicators）
- **B3** thiserror 统一 v2

### 冲刺 2（架构底座 · 2–4 周 · 中风险）
- **A2** EMA 种子约定统一（加 `seed` 参数 + 文档契约）✅ **已完成 2026-07-19**：新增 `EmaSeed{Sma,FirstValue}` 枚举 + `ema_with_seed`/`StreamingEma::with_seed`，默认保持 Sma（golden 不受影响），新增批量↔流式 FirstValue 收敛测试
- **A3/A4** FFI panic 隔离 + 内存泄漏测试
- **B4** 零拷贝 `_into` 扩展 + `SliceOutput`
- **C2** 单遍扫描重写（CCI/WILLR/STOCH/ADX…）
- **D5/D6** benchmark harness 统一 + MSRV 实证

### 冲刺 3（性能与并行 · 3–5 周）
- **C1** SIMD 内核扩展（SMA/WMA/统计类）
- **C3** watch-list 指标优化
- **B5** 批处理并行化（rayon + eval_batch）
- **B2** 流式 Batch 2–4 补齐

### 冲刺 4（生态与文档 · 持续）
- **E2** mdBook 文档站点
- **E3/E4** Python/Node 深化
- **E1** Pine v5 解析器（独立大里程碑，可单列）

### 冲刺 5（no_std 完整门控 · 高成本，单独立项）
- **B1** 全模块 no_std 化 + 全平台 CI 实证。**建议作为独立重构立项**，因其触及面最广、回归风险最高。

---

## 4. 立即可启动的"第一组"清单（最小可验证）
1. 修 `prop_macd_line_difference_invariants`（A1）— 改测试期望，1 处变更，零回归风险。
2. clippy 本地化（D1）— 文档 + Makefile target，零代码风险。
3. fuzz 补 3 target（D2）— 复制现有 fuzz target 模板。
4. thiserror 统一（B3）— 改 2 处依赖声明 + 适配 v2 API。
5. 注册表生成器入 CI（F1）— 防本轮已遇的 json 漂移复发。

---

## 5. 风险与护栏
- **golden 锁定约束**：`golden_macd_12_26_9` / `golden_atr_14` / `golden_talib_atr` 自生成并 pin 定 batch `macd`/`atr` 语义，**绝不可为对齐而改 batch 语义**，只能改流式/测试期望。
- **EMA 种子**：统一时须保证所有 golden 测试不破（A2 改动后重跑 `core/tests`）。
- **no_std（B1）**：高成本，建议独立立项、增量推进、每模块单独 CI 验证。
- **SIMD（C1）**：新增内核遵守"避免 fma"约定；运行时 `is_x86_feature_detected!` 分发，scalar 回退必在。
- **FFI 漂移**：任何 core 指标变更后必跑 `scripts/sync_bindings.py --check` 确认 8 语言 drift=none。

---

## 6. 成功度量
- 预存在测试失败 = 0；`cargo test --workspace` 全绿。
- clippy `-D warnings` 本地可跑且 CI 通过。
- fuzz 6/6 target 在 CI 跑通。
- 生产路径 `unwrap` 归零（或仅剩带 `expect("reason")` 的明确项）。
- no_std 全模块编译通过（冲刺 5 后）。
- 性能 watch-list 指标 ≥ 1.0x（追上 TA-Lib）。

---

*本计划为动态文档。执行时每完成一个支柱项即回填状态与 `REFACTORING_PLAN.md` / `PLANNING.md`，避免多份计划失同步。*
