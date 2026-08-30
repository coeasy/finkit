# Finkit 深度审计与优化改进方案

> 生成日期：2026-08-30 | 前置：`FINKIT_V1_ALIGNMENT_AND_FIX_PLAN.md`（阶段 A 已执行，提交 `ecb7c17`）
> 范围：功能点全貌梳理 → 潜在问题 → 断链清单 → 深度优化改进方案

---

## 1. 功能点全貌（现状盘点）

| 功能域 | 现状 | 规模/证据 | 成熟度 |
|--------|------|-----------|--------|
| 指标系统 | 批量 + 流式 + 参数扫描三模式 | 注册表 **236 项**（`docs/indicator_registry.json`），14 分类 | 高（golden/property/兼容测试齐备） |
| 流式引擎 | O(1) 增量更新、checkpoint/restore | **98 个流式指标**，Builder API | 高 |
| 公式引擎 | 解析器/字节码/JIT/SIMD 四执行路径 | `core/src/formula/`（engine/bytecode/jit/sandbox）+ `pine/` | 中：四路径差分一致性测试缺失 |
| Pine 兼容 | 语法映射到 AST 后复用执行管线 | 兼容率：解析 69% / 映射 46% / 求值 **31%** | 低（V2 目标，当前对外表述需收敛为 TDX 方言） |
| 因子系统 | features 引擎（engine/matrix/selection/labels） | `core/src/features/` | 中低：无 Factor 一级 API，FFI/Python 暴露薄弱 |
| 图形态 | 60+ 蜡烛图 + 15+ 图形态 | `core/src/patterns/` | 中 |
| 交易辅助 | backtest / risk / metrics / sector | `core/src/backtest.rs` 等 | 中（对外叙事中未定义版本归属） |
| 可视化 | 独立 crate，SVG/PNG/HTML/JSON 渲染 + decimate | `visualization/` | 中 |
| 多语言绑定 | Python/Node/C/.NET/Java/Go/Android/iOS/WASM | 9 个绑定目录 | Tier1/2 未发布（beta/experimental） |
| CLI | 指标计算/公式/流式/图表命令 | `cli/src/main.rs` | 中（命名仍 `alpha_ta`，帮助文本待对齐） |
| Fuzz | 4+ 目标（公式解析/指标/公式 JIT） | `fuzz/fuzz_targets/` | 低：未接入 CI（cargo-fuzz 依赖 MSRV/nightly） |
| 构建/打包 | 一键构建 + Docker + 多格式安装包 | `build-usage*.sh/.ps1`、`scripts/build-installer.sh` | 中（脚本深、CI 浅） |

> `core/src/` 内 **无 TODO/FIXME/unimplemented! 残留**，代码卫生良好。

---

## 2. 潜在问题（按严重度）

### P0（发布阻塞 / 诚信问题）

| # | 问题 | 位置 | 证据 |
|---|------|------|------|
| P-1 | **CI 覆盖严重不足**：仅 5 job；clippy 仅 `-p finkit` 为硬门禁（workspace 仅 advisory）；FFI 绑定、Node/Python 测试、性能门禁、文档校验全不在 CI | `.github/workflows/ci.yml`（文件尾注释自认 "intentionally NOT part of this minimal core gate"） | 单 workflow，对比绑定 9 个目录 |
| P-2 | **交付物声明失真**：PROGRESS.md 宣称 "CI 性能门禁：perf-gate.yml + bench_gate.sh" 交付，实际 `perf-gate.yml` **不存在**（bench_gate.sh 存在但无 workflow 调用） | `docs/PROGRESS.md` 最终交付物段 | Glob workflows 仅 ci.yml |
| P-3 | **版本/包名演示链路断裂**（详见 §3 断链）：README 快速开始示例无法照抄执行 | `README.md` L83/L99/L103 | 108 处旧命名残留 |
| P-4 | 基准基线未入库：version-matrix 记录 criterion 索引 0 条，性能宣称（"1.3x–3.2x faster"）无可复现基线 | `docs/generated/version-matrix.md` | — |

### P1（质量风险）

| # | 问题 | 位置 | 证据 |
|---|------|------|------|
| P-5 | `core/src/batch.rs` 多处 `unwrap()`/`as_ref().unwrap()`（L139–L258 输出切片、结果解包）——批量主路径 panic 风险 | `core/src/batch.rs` | 已有 `docs/A5_UNWRAP_AUDIT.md` 审计文档但未闭环 |
| P-6 | FFI panic 隔离正确性依赖生成代码，无自动审计：`#[no_mangle]` 导出数 vs `ffi_catch_*` 包裹数无 CI 校验 | `ffi/*/src/lib.rs`、`ffi-common/src/panic.rs` | 注释明言"应围绕 registry-driven generated function 使用" |
| P-7 | cbindgen 头文件与 Rust 源无自动 diff 校验，头文件漂移风险 | `ffi/c-binding/include/finkit.h` | TASK-231 未完成 |
| P-8 | 公式四路径（解释器/字节码/JIT/SIMD）无差分一致性测试，性能优化路径与参考路径可能输出不一致 | `core/src/formula/` | TASK-226 未完成 |

### P2（叙事/结构债务）

| # | 问题 | 位置 |
|---|------|------|
| P-9 | 指标数量口径三重矛盾：README "150+"、core/README "177"、registry 实际 236 | 三处文档 |
| P-10 | docs/ 下 8+ 套规划/总结文档并存（PLANNING/PROGRESS/UPGRADE_PLAN_2026/OPTIMIZATION_PLAN_2026/OPTIMIZATION_REFACTORING_PLAN/REFACTORING_PLAN_2026-08/RELEASE_NOTES_v2.0/PRD），读者无法判断权威叙事 | `docs/` |
| P-11 | RELEASE_NOTES_v2.0.md 未归档（规划文档要求移入 archive） | `docs/` 根 |
| P-12 | CLI 二进制名/帮助文本仍为 `alpha_ta` | `cli/src/main.rs` L15/L206 |

---

## 3. 断链清单（旧命名/失效引用，共 **108 处 / 32 文件**）

| 断链类型 | 重灾区（文件→命中数） | 性质 |
|----------|----------------------|------|
| 旧包名安装命令 | `README.md`(17)、`docs/installation.md`(13)、`docs/QUICK_START.md`(5)、`examples/README.md` | `npm install @alphata/node`、`pip install alpha-ta`、`cargo add alpha-ta-core`、`dotnet add package alpha_ta` |
| 旧 crate 名 | `docs/api-reference.md`(5)、`docs/ALPHATA_VS_TALIB.md`(9)、`docs/formula/README.md` | `use alpha_ta_core`、`alpha-ta-core = "1.0.0"` |
| Node npm 子包名 | `ffi/node-binding/README.md`(7) 与 8 个 `npm/*/package.json` 内部引用 | `@alphata/node` vs `finkit` |
| Docker 镜像名 | `Dockerfile`(3)、`docker-compose.yml`(2)、`Makefile`(3) | `alpha_ta/builder:latest` |
| code-wiki 历史文档 | `docs/code-wiki/*`(14)、`docs/architecture/*`(6) | 叙事性历史，可标注或归档 |
| 标题/品牌 | `README.md` L1 标题仍为 "# AlphaTA" | 用户第一眼即断链 |
| 生成器脚本 | `scripts/gen_binding.py`(3)、`scripts/generate_golden.py`(1) | 生成产物名仍为旧名（若再运行会回写旧命名） |

> Cargo.lock 已干净（0 命中）；`import finkit`/`-p finkit-*` 功能性引用已在前次提交清零。

---

## 4. 改进方案（四阶段）

### 阶段 B+：断链清零与叙事收敛（P0，纯文档+脚本，无代码风险）

| ID | 任务 | 验收标准 |
|----|------|----------|
| B+1 | **README 彻底翻新**：标题改 Finkit；Node/Rust/安装表/快速开始全部对齐 `finkit`；指标数改由脚本生成（以 registry=236 为 SSOT） | 全文 0 处旧命名；示例命令与真实包名一致 |
| B+2 | 文档旧命名清洗：installation/QUICK_START/api-reference/ALPHATA_VS_TALIB/examples/README/node-binding README（npm 子包描述与 8 个 npm/*/package.json 一致性） | 全库 Grep 旧包名命令清零（code-wiki/architecture 归档除外） |
| B+3 | Docker/Makefile 镜像与服务名统一 `finkit/builder` | docker build/run 示例可执行 |
| B+4 | CLI 更名：`#[command(name = "finkit")]` + 帮助文本（保留 `alpha_ta` 公式方言枚举名，那是域概念非品牌） | `finkit --help` 一致 |
| B+5 | 叙事收敛：RELEASE_NOTES_v2.0.md 归档；docs/INDEX.md 增设「权威文档」区，其余规划文档标注 superseded 并链接到权威版 | docs 首页唯一入口 |
| B+6 | PROGRESS.md 交付物声明修正（perf-gate.yml 标注 planned）| 声明与文件系统一致 |
| B+7 | 生成器防回写：gen_binding.py / generate_golden.py 输出名改 finkit，防止再运行时回退旧命名 | 重新生成产物名正确 |

### 阶段 B：CI/Benchmark/Release 闭环（承接前规划阶段 B，P0）

| ID | 任务 | 验收标准 |
|----|------|----------|
| B-1 | CI 扩容：clippy `--workspace --all-targets -D warnings` 硬门禁；新增 Python（maturin + pytest）、Node（npm test）、C（cmake）三个绑定 job | ci.yml ≥ 8 job，绑定破坏被拦截 |
| B-2 | cbindgen 头文件 diff 校验 job | 生成头文件与入库 diff 为空 |
| B-3 | FFI panic 隔离审计：脚本统计 `#[no_mangle]` 数 vs `ffi_catch_*` 包裹数，不一致即失败 | 审计脚本入库且 CI 绿 |
| B-4 | perf-gate.yml 新建（兑现声明）：调 bench_gate.sh，容差策略入库 | 打 tag/nightly 触发，基线入库 |
| B-5 | release.yml：tag 触发 maturin→PyPI / npm publish / crates.io publish + secrets 说明 | v0.1.0 tag 自动产包 |
| B-6 | 文档 CI：`gen_ssot_docs.py --check` + md link 检查 | 文档失同步即红 |

### 阶段 C：质量加固（P1 代码项）

| ID | 任务 | 验收标准 |
|----|------|----------|
| C-1 | batch.rs unwrap 清零（返回 Result 或 expect 带上下文），闭环 A5_UNWRAP_AUDIT | clippy `clippy::unwrap_used`（core 生产路径）通过 |
| C-2 | 公式四路径差分测试：同公式同数据，解释器 vs 字节码 vs JIT vs SIMD 逐位一致 | `cargo test -p finkit formula_differential` 绿 |
| C-3 | Pine 兼容基线测试固化：31% 率入 CI 防退化，Pine 完整支持移 V2 | 基线测试绿 |
| C-4 | 因子系统 Factor API：features 之上定义 10~15 个基础因子（动量/波动/质量）+ Python 暴露 | `ta.factor_matrix(df)` 可用 |

### 阶段 D：V1.0 验收（Definition of Done）

- [ ] B+/B/C 全绿，main 分支 CI（含绑定矩阵）绿
- [ ] `v0.1.0` tag → release 流水线产出可公开安装包
- [ ] README 快速开始三条命令（pip/npm/cargo）真实可复现
- [ ] 断链归零：全库旧命名命令级残留 = 0（历史归档文档除外）
- [ ] COMPAT_MATRIX 22 指标全 pass + 基准基线入库 + perf-gate 绿
- [ ] 指标数量口径唯一（registry 驱动，脚本生成）

---

## 5. 执行顺序建议

1. **立即**：阶段 B+（纯文档，1 次提交可完成，风险最低、用户可见度最高）
2. **随后**：B-2/B-3/B-6（CI 加固三件套，不需外部 secret）
3. **并行**：C-1/C-2（代码质量，独立可测）
4. **需用户配合**：B-5 release secrets、A-7 远程推送认证
5. **V2 议题**（不在本方案）：Pine 语义补全、TA-Lib 全量兼容矩阵、指标数量三模式对齐缺口
