# Finkit 第一版（基础计算引擎）对齐与修复规划

> 生成日期：2026-08-30 | 状态：待评审
> 范围：对齐全部既有版本方案，识别完成第一版（完整基础计算引擎）的潜在问题，给出统一修复计划。

---

## 1. 统一版本路线（本文档的唯一权威叙事）

将仓库内所有版本方案收敛为一条四级路线（与既定规划一致）：

| 阶段 | 版本号 | 主题 | 核心交付 |
|------|--------|------|----------|
| V1 | **v0.1.0** | 完整基础计算引擎 | 指标系统、公式系统、因子系统、高性能 Runtime、Python/Rust/CLI、多语言稳定 ABI、CI/Benchmark/Release |
| V2 | v0.2.0 | 指标目录大规模扩展 | TA-Lib 全量兼容、更多公式终端兼容 |
| V3 | v0.3.0 | 跨语言 SDK 全面成熟 | WASM/Node/Java/.NET/Go 正式发布 |
| V4 | v1.0.0 | API/ABI 稳定承诺 | 完整兼容矩阵、长期生产支持 |

**关键决策 D-1（阻塞项）**：当前 workspace 版本为 `1.0.0`，但按四级路线第一版应为 `v0.1.0`。建议**将 workspace 版本统一降为 `0.1.0`**：
- 理由：`1.0.0` 意味着 semver 稳定承诺，而当前 API 尚未经生产验证、CI/发布基建未闭环、绑定成熟度声明与事实不符（见 §4-P0-4）。以 1.0.0 起步会透支 v1.0.0"稳定承诺"的语义。
- 影响面：`Cargo.toml`（workspace.package.version）、`ffi/python-binding/pyproject.toml`、`ffi/node-binding/package.json`、`docs/generated/version-matrix.md`（重新生成）、各 README 中的版本字符串。
- 若坚持保留 `1.0.0`，则必须先补齐 §5 中全部 P0 项后再打 tag，且 v2/v3 需顺延为 1.1/1.2，v1.0 稳定承诺改称 2.0 LTS（不推荐，与现有文档叙事冲突更大）。

**关键决策 D-2（阻塞项）**：命名统一。当前三套命名并存：仓库 `coeasy/finkit`、crate/包 `alpha-ta-*` / `alpha_ta` / `@alphata/node`、以及一个未执行的 `scripts/rename_to_rusta.py`（拟改名 Rusta）。建议**统一为 `finkit`**（与仓库名、近期 v0.1.0 规划文档、Go 模块路径 `github.com/coeasy/finkit/go/ta` 一致），在发布任何公开包之前完成一次性重命名，并删除 `rename_to_rusta.py` 等干扰脚本。

---

## 2. 既有版本方案盘点与处置

仓库现存 4 套相互冲突的版本叙事，处置如下：

| # | 文档 | 叙事 | 与统一路线的冲突 | 处置 |
|---|------|------|------------------|------|
| 1 | `docs/plan/FINKIT_V0.1.0_ROADMAP.md` / `_VERSION_SPEC.md` / `_ARCHITECTURE_DESIGN.md` | v0.1.0 基础版（近期新增，3 个 docs commit） | 里程碑内容过于保守（仅 12 个指标），实际代码远超 | **保留为 V1 骨架，按本文档 §5 扩充验收标准** |
| 2 | `docs/plan/FINKIT_V2_OPTIMIZATION_PLAN.md` | C++20 核心，v0.1→v1.0 | **技术路线与事实相反**（实际为 Rust workspace） | **作废重写**：改写为"V2 指标目录扩展方案"，删除 C++ 表述 |
| 3 | `docs/PLANNING.md` + `docs/PROGRESS.md`（AzaLoop） | 215/276 story，55 pending + 6 blocked（P0：TASK-216~248、301~328） | 任务粒度与版本无映射；进度日志未更新（最新会话成果未记录） | **保留为任务底册**：P0 未完成项全部编入 §5 修复计划 |
| 4 | `docs/RELEASE_NOTES_v2.0.md` | "v2.0 发布说明"，但标注 crate 版本 1.0.0，且承认 release/perf-gate/fuzz/docs 工作流"为后续规划" | 版本号自相矛盾；发布说明先于发布存在 | **归档**（移入 `docs/archive/`），V1 发布后再按实际内容重写 |

其他需同步收敛的口径文档：`docs/BINDING_TIERS.md`（成熟度声明不实，见 P0-4）、`docs/COMPAT_MATRIX.md`（数据过期，见 P0-7）、`core/README.md`（宣称 177 指标，注册表实际约 236 项，口径不一）。

---

## 3. 第一版完成度现状（按统一路线逐项盘点）

| V1 目标 | 现状 | 证据 | 差距 |
|---------|------|------|------|
| 指标系统 | 基本达成 | 注册表约 236 项（`docs/indicator_registry.json`）；批量+流式+参数扫描 | 数量口径不一致（core/README 称 177）；registry 与 FFI/公式引擎注册三方一致性无 CI 校验 |
| 公式系统 | 部分达成 | `core/src/formula/`（parser/bytecode/jit/executor/sandbox）+ `pine/` 子模块；TDX/THS/DZH 兼容测试存在 | Pine 兼容率仅：解析 69% / 映射 46% / 求值 31%（`docs/PINE_COMPAT_MATRIX.md`）；第一版对外表述需降级为"通达信系方言"或补齐 |
| 因子系统 | 部分达成 | `core/src/features/`（engine/matrix/selection/labels 等） | 对外暴露与文档薄弱；`factor` 作为一级概念未在版本叙事中定义（价值/质量因子等基础因子集缺失） |
| 高性能 Runtime | 基本达成 | SIMD/AVX-512 分发、流式环形缓冲、零分配 API、Criterion 基准 24 个 bench 文件 | CI 无性能回归门禁；`version-matrix.md` 记录 criterion 基线索引为 **0**（基线未入库） |
| Python/Rust/CLI | 基本达成 | `ffi/python-binding/`（含 .pyi、GIL 释放、零拷贝）、`core/`、`cli/` | Python 多平台 wheel 未发布；CLI 能力面未与文档对齐 |
| 多语言稳定 ABI | 部分达成 | C 头文件 `ffi/c-binding/include/alpha_ta.h` 由 `generated.rs` 从注册表生成；`ffi-common` 有 panic 隔离（panic.rs）与错误码（error.rs） | panic guard 是否覆盖全部导出函数未审计（AzaLoop TASK-229）；cbindgen 头文件与代码无 CI 自动校验（TASK-231）；内存契约泄漏测试缺失（TASK-230） |
| CI/Benchmark/Release | **未达成** | 仅 `ci.yml`（fmt/clippy/core test/doc/audit），且**当前在工作区被意外删除**；clippy 仅 `-p finkit` 为硬门禁；无 tag、无根级 release 工作流；远程推送被认证阻塞 | 详见 §4 |

---

## 4. P0 问题清单（第一版发布的阻塞项）

| # | 问题 | 证据 | 风险 |
|---|------|------|------|
| P0-1 | **CI 工作流被意外删除** | `git status` 显示 ` D .github/workflows/ci.yml`（已提交于 de607ed，工作区未暂存删除） | 推送后仓库无任何 CI |
| P0-2 | **版本号冲突**：workspace=1.0.0 vs 规划 v0.1.0 | 根 `Cargo.toml` L21、`version-matrix.md` | semver 语义透支（见 D-1） |
| P0-3 | **命名三体混用**：finkit / alpha-ta / rusta | `pyproject.toml`（alpha-ta）、`package.json`（@alphata/node）、`Cargo.toml`（alpha-ta-*）、`scripts/rename_to_rusta.py` | 公开包名一旦发布难以回收（见 D-2） |
| P0-4 | **绑定成熟度声明不实**：`BINDING_TIERS.md` 将 Python/Node/WASM 标为 `stable`、"PyPI/npm 发布"，实际无任何包已发布、无 tag | 仓库无 tag、CHANGELOG 全部 `[Unreleased]`、根 ci.yml 注释明确"FFI 绑定不在 CI 内" | 文档诚信问题（AzaLoop TASK-219 文档诚信审计的核心诉求） |
| P0-5 | **CI 覆盖不足**：仅 core 单包；无绑定构建矩阵、无 ABI 校验、无性能回归门禁、无文档 CI | `ci.yml` 内容；AzaLoop TASK-217/223/224/231/248 均未完成 | "CI passing"验收标准不成立 |
| P0-6 | **远程仓库未推送** | git remote 认证阻塞（GCM 交互登录或 PAT） | 无法触发 CI/Release，V1 验收无从验证 |
| P0-7 | **TA-Lib 兼容矩阵过期且结论为 0%**：`COMPAT_MATRIX.md` 记录 22 个指标全部 skip（"golden missing"），但 `tests/golden/talib/*.json` 22 个文件实际存在 | 文档生成于 2026-06-23 template/dry-run 模式，源路径为 Windows 反斜杠 `target\talib_compat_report.json` | 兼容性宣称无事实支撑（TASK-222：矩阵自动生成并入 CI） |
| P0-8 | **AzaLoop 6 个 blocked 任务未解除**（TASK-216~220） | `docs/PROGRESS.md` L133-147 | 均为 P0 基建（git/CI/数据集/文档审计/TA-Lib 参考生成器） |
| P0-9 | **进度记录失真**：最新会话完成的 CI 建立、初始提交（de607ed，1134 文件）等成果未回写 PROGRESS/PLANNING | `docs/PLANNING.md` 状态停留在 2026-06-24 | 规划文档失去可信度 |
| P0-10 | **基准基线未入库**：`version-matrix.md` 记录 criterion 基准索引 0 条 | `docs/generated/version-matrix.md` L26 | benchmark 验收标准无数据 |

---

## 5. 修复计划

### 阶段 A：止血（P0，发布前必须完成）✅ 已执行 2026-08-30

| ID | 任务 | 验收标准 | 涉及位置 | 对应 AzaLoop | 状态 |
|----|------|----------|----------|--------------|------|
| A-1 | 恢复被删除的 `ci.yml` 并提交 | `git status` 干净；CI 文件在 HEAD | `.github/workflows/ci.yml` | TASK-217 | ✅ |
| A-2 | 执行 D-1 版本决策（降为 0.1.0） | 全 workspace、pyproject、package.json、version-matrix 一致为 0.1.0 | 根 `Cargo.toml` 等 4 处 | TASK-219 | ✅ |
| A-3 | 执行 D-2 命名决策（统一 finkit） | crates/PyPI/npm/Maven/NuGet/Go 清单文件全部使用统一名；删除 `rename_to_rusta.py` | 各绑定清单 + scripts | TASK-219 | ✅ |
| A-4 | 重写 `BINDING_TIERS.md` 成熟度 | 文档无未发布却宣称 stable 的条目 | `docs/BINDING_TIERS.md` | TASK-235 | ✅ |
| A-5 | 修复 `gen_compat_matrix.py` 并重新生成矩阵 | 22 个 golden 有真实 pass/fail 结论 | `scripts/gen_compat_matrix.py`、`docs/COMPAT_MATRIX.md` | TASK-222 | ✅ |
| A-6 | 重建 PROGRESS/PLANNING 状态 | 进度数据与 git log 一致 | `docs/PLANNING.md`、`docs/PROGRESS.md` | TASK-216 | ✅ |
| A-7 | 推送远程后打 `v0.1.0-rc.1` tag | 远程可见 tag 与分支 | GitHub | — | ⏳ 待认证 |

### 阶段 B：CI/Benchmark/Release 闭环（第一版"CI passing"验收的核心）

| ID | 任务 | 验收标准 | 对应 AzaLoop |
|----|------|----------|--------------|
| B-1 | CI 扩容：clippy 全 workspace `-D warnings` 硬门禁；Python（maturin sdist 构建+pytest）、Node（npm test）、C（cmake 构建+测试）三个绑定 job | 根 ci.yml 含 5+ job；绑定破坏会被 CI 拦截 | TASK-217 |
| B-2 | cbindgen 头文件自动生成 + ABI 一致性 CI 校验 | CI job 对比生成头文件与入库头文件 diff 为空 | TASK-231 |
| B-3 | FFI panic 隔离全量审计 | 所有导出函数均经 `ffi_catch_*` 包装；新增 lint/测试验证 | TASK-229 |
| B-4 | FFI 内存契约泄漏测试（valgrind/`leak.rs`）入 CI | 泄漏测试绿灯 | TASK-230 |
| B-5 | Criterion 基准基线入库 + 性能回归门禁（容差策略） | `target/criterion` 关键基线提交；perf-gate job 绿 | TASK-223/224 |
| B-6 | 发布流水线：`release.yml`（tag 触发：maturin→PyPI、npm publish、crate publish），含 secret 预配置说明 | 打 tag 后自动产出 PyPI/npm 公开包 | TASK-232 |
| B-7 | 文档 CI：`gen_ssot_docs.py --check` + 链接检查 + mdBook 构建 | 文档失同步即 CI 失败 | TASK-248/246 |

### 阶段 C：第一版内容补全（对齐"完整基础计算引擎"承诺）

| ID | 任务 | 验收标准 | 对应 AzaLoop |
|----|------|----------|--------------|
| C-1 | 因子系统定位与最小因子集：在 `features` 之上定义 Factor API（动量/波动/质量等 10~15 个基础因子）+ Python/FFI 暴露 + 文档 | `pip` 用户可一行计算因子矩阵 | — |
| C-2 | 公式系统第一版表述收敛：TDX/THS/DZH 方言为 V1 承诺面，Pine 明确标注为 V2 目标 | README/文档不再笼统宣称"公式终端兼容" | TASK-225/239~244 |
| C-3 | 四执行路径（解释器/字节码/JIT/SIMD）差分一致性测试 | 同一公式四路径输出逐位一致 | TASK-226 |
| C-4 | 指标数量口径统一：registry 为 SSOT，README 数字由脚本生成 | 177/236 矛盾消除 | TASK-246 |
| C-5 | FFI 错误码→Python 语义化异常映射完善 | 每个错误码有对应 Python 异常类 | TASK-234 |
| C-6 | Tier1 绑定深化：包内测试、类型 stub（Python .pyi 完整、Node .d.ts）真实可用 | 上游包测试矩阵绿灯 | TASK-236/325 |

### 阶段 D：V1 验收（Definition of Done）

- [ ] A/B/C 全部任务关闭，CI（含绑定矩阵）在 main 绿灯
- [ ] `v0.1.0` tag 触发 release 流水线，PyPI/npm/crates.io 公开可安装
- [ ] `pip install finkit==0.1.0` / `npm install finkit` 快速开始示例可复现
- [ ] COMPAT_MATRIX 真实结论 ≥ 22 指标全 pass；基准基线入库
- [ ] CHANGELOG 出现第一个 released 版本段落；RELEASE_NOTES 按事实重写
- [ ] BINDING_TIERS 成熟度与实际发布状态一致

---

## 6. 后续版本对齐（V2/V3/V3+ 的任务来源映射）

| 版本 | 主题 | 任务来源（AzaLoop 编号） | 备注 |
|------|------|--------------------------|------|
| V2 (0.2.0) | TA-Lib 全量兼容 + 公式终端扩展 | TASK-018~031 已完成部分为存量；新增：全量 golden 生成器（220）、逐函数比对器（221）、Pine 语料（242）、Pine series 语义（241/320/321） | 兼容矩阵自动生成是 V2 的核心度量 |
| V3 (0.3.0) | SDK 全面成熟 | Java/.NET/Go 从 Tier2→正式发布：release-java.yml / release-dotnet.yml 已有骨架待启用；Go module 发布（`go get github.com/coeasy/finkit`） | 各目录下的 release 工作流移入根编排 |
| V4 (1.0.0) | API/ABI 稳定承诺 | 兼容矩阵全量绿、MSRV 锁定（当前 1.75）、语义化弃用流程、LTS 支持窗口文档 | 1.0 之前冻结破坏性变更节奏 |

---

## 7. 立即行动（本周）

1. `git checkout -- .github/workflows/ci.yml` 恢复 CI（或重新提交）
2. 确认 D-1（版本号）与 D-2（命名）两项决策
3. 解决 GitHub 推送认证（PAT），推送 5 个本地提交
4. 按阶段 A 顺序执行 A-2 ~ A-6，随后进入阶段 B
