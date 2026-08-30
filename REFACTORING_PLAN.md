# AlphaTA（finkit 工作区）结构梳理与优化/重构计划

> **状态：计划已全面执行并收口（2026-07-18 续七）。** 经过 7 轮迭代，全部 P0/P1/P2/P3 项均已落地；核心库「可关闭 `indicators-all` 按需裁剪」的承诺已通过修复 9 类别 feature 独立编译的真 bug 兑现（libm 常驻 + 类别依赖边 + simd_ops/mod.rs 门控）；Android/iOS 指标覆盖补齐到 15，并补齐绑定 smoke 测试；冗余 `python-wheels.yml` 已删除。**所有改动均已 `cargo check --workspace`、各绑定 `cargo test`、`sync_bindings.py --check`（8 语言 drift=none）验证通过。** 遗留：MSRV 本地 1.75 验证因 toolchain 安装受阻（CI 步骤已加，待联网环境实证）；no_std 完整门控为更大重构，本期未做。

> 适用范围：`P:\llm_code\finkit` 整个 Cargo workspace。

---

## 1. 项目现状速览

| 维度 | 现状 |
|------|------|
| 产品 | AlphaTA —— 高性能金融技术分析 Rust 库（150+ 指标，对标 TA-Lib） |
| 工作区目录名 | `finkit`（与 crate 名 `alpha_ta-*` 不一致，见 §4-命名） |
| Workspace members | 13 个：`core`、`visualization`、`cli`、`wasm`、`ffi/c-binding`、`ffi/ffi-common`、`ffi/python-binding`、`ffi/node-binding`、`ffi/go-binding`、`ffi/dotnet-binding`、`ffi/ios-binding`、`ffi/java-binding`、`ffi/android-binding` |
| 核心代码量 | `core/src` 300 个 `.rs`、约 129.6k LOC |
| FFI 绑定 | 8 种语言，**全部由注册表驱动同步**（`scripts/sync_bindings.py`） |
| 构建入口 | **`Makefile`**（唯一权威）+ `build-usage.{sh,ps1}` |
| 指标注册表 | `docs/indicator_registry.json`（526KB，78 指标）—— 单一事实源（SSOT） |
| 代码生成器 | 5 个 Python 脚本：`sync_bindings.py`(8 语言同步)、`gen_binding.py`(C/Python/Node 发射)、`gen_c_header.py`(C 头)、`gen_ssot_docs.py`(文档)、`enrich_registry_ffi.py`(元数据补全) |
| 测试 | `core/tests` 33 个集成测试文件、1500+ 单元测试、跨语言 golden 测试 |
| 文档 | `docs/` 58+ 篇（含索引 INDEX.md、架构/FFI/公式/mdBook 源等） |
| CI | 7 个 GitHub Actions workflow（主 CI、文档检查/部署、fuzz、perf-gate、Python wheels、release） |

---

## 2. 已完成的里程碑

| 阶段 | 内容 | 状态 |
|------|------|------|
| **P0** | 根目录清理：~50 → 31 个文件；调试脚本/报告/大文件归档 | ✅ 已完成 |
| **P1-1** | 构建整合：确认 Makefile 权威入口，14 个冗余根脚本归档 | ✅ 已完成 |
| **P1-3** | `ffi/ffi-common` 共享 crate（错误/注册表/类型/golden） | ✅ 已完成 |
| **P1-2** | 注册表驱动代码生成：C 头、C 绑定重生、8 语言同步器 `sync_bindings.py` | ✅ 已完成 |
| **P2** | core 分类 feature flags（9 类别 + `indicators-all`）+ golden 测试 + 文档索引 | ✅ 已完成 |
| **P3** | CI 防漂移（`sync_bindings.py --check`）、`.gitignore`、命名决策 | ✅ 已完成 |
| **续四** | 安全加固（cargo audit 必选 + cargo deny 接入 CI）、Dependabot、Fuzz CI 补全、benchmark 脚本合并、Go/.NET/Java/CLI/WASM 测试增加、P3 工程化清理 | ✅ 已完成 |
| **续五** | 按计划全部改进：安全加固（audit 必选+deny）、Dependabot、Fuzz 补全、benchmark 归档、Go/.NET/Java/CLI/WASM 测试、P3 清理、MSRV CI 步骤 | ✅ 已完成 |
| **续六** | Android/iOS 指标覆盖补齐（6 → 15），注册表驱动重生 generated.rs，`sync --check` 8 语言 drift=none | ✅ 已完成 |
| **续七** | 修复 core 单类别 feature 编译真 bug（libm 常驻+类别依赖边+simd_ops/mod.rs 门控）；新增 Android/iOS smoke 测试；删除冗余 python-wheels.yml | ✅ 已完成 |

---

## 3. 目标结构（重构后，已落地）

```
finkit/
├── Cargo.toml                  # workspace，members 含 ffi/ffi-common
├── Makefile                    # 权威构建入口
├── build-usage.{sh,ps1}        # 入口脚本（forwarder）
├── Dockerfile / docker-compose.yml
├── core/  src/ (indicators/ math/ streaming/ formula/ patterns/ transforms/ + traits.rs)
├── ffi/
│   ├── ffi-common/             # 共享错误/注册表/类型/golden
│   ├── c-binding/              # 注册表驱动生成 (78 指标)
│   ├── python-binding/         # 注册表驱动同步 (71 指标)
│   ├── node-binding/           # 注册表驱动同步 (76 指标)
│   ├── go-binding/             # 注册表驱动同步 (40 指标)
│   ├── dotnet-binding/         # 注册表驱动同步 (41 指标)
│   ├── ios-binding/            # 注册表驱动同步 (6 指标)
│   ├── java-binding/           # 注册表驱动同步 (42 指标)
│   └── android-binding/        # 注册表驱动同步 (6 指标)
├── cli/  wasm/  visualization/
├── scripts/                    # ~75 个脚本（含 archive/ 归档旧脚本）
├── tests/                      # 数据资产（data/fixtures/golden/...）
├── docs/                       # 58+ 文档 + indicator_registry.json + generated/
├── .github/                    # 唯一 CI 源（7 个 workflow）
└── .gitignore                  # 120+ 条目
```

---

## 4. 命名一致性

`finkit`（仓库目录名） vs `alpha_ta-*`（crate 内部名） vs `alpha-ta-*`（对外包名）。

**已决策（2026-07-18）**：
- 保留 `finkit` 仓库目录名（不改名，高风险）。
- 对外统一 `alpha-ta`（crate 已为 `alpha-ta-*`，无需额外改名）。
- 不强行重命名产物（`finkit.dll`、`finkit.win32-x64-msvc.node` 等）—— 记录在案，需逐产物确认。

---

## 5. 当前全面诊断（2026-07-18 续四）

本节基于对整个工作区的深度扫描，列出**所有**有待改进的问题（含已知旧问题和新发现的工程缺口），按 P0（阻塞/安全）→ P1（高价值结构性）→ P2（优化）→ P3（工程化）分级。

### 🔴 P0 — 安全/完整性风险

| # | 问题 | 说明 | 建议 |
|---|------|------|------|
| 0-1 | **CI 中 `cargo audit` 是 optional 的** | `ci.yml` 中 `cargo audit` 带 `continue-on-error: true`，CVE 不阻断 CI | 改为必选，或至少对已知 CVE 设置 advisory ignore（附原因） |
| 0-2 | **CI 不运行 `cargo deny`** | `deny.toml` 已配置但从未在 CI 中执行 | CI 中增加 `cargo deny check` 步骤（依赖/许可/来源审计） |
| 0-3 | **`deny.toml` 中 `db-path` 硬编码本地路径** | `db-path = "~/.cargo/advisory-db"`，CI 中可能不存在 | 删除该行（使用默认值，自动下载） |

### 🟠 P1 — 高价值结构性改进

| # | 问题 | 说明 | 建议 |
|---|------|------|------|
| 1-1 | **Android binding 严重不完整** | 只暴露 7 个指标（sma/ema/wma/rsi/mom/roc），其他 71 个不可用 | 扩展 `dispatch_ta()` 覆盖所有注册表指标；或改为通用 JNI 调度机制 |
| 1-2 | **iOS binding 同样不完整** | 只有 6 个指标，且 `generated.rs` 直接依赖 `moving_avg`/`indicators` 模块 | 与 Android 类似，需要扩展；考虑是否与 Android 共用 dispatch 逻辑 |
| 1-3 | **没有 Dependabot / Renovate** | 依赖更新全靠手动，无自动化 | 增加 `.github/dependabot.yml`，至少开启 GitHub Actions 和 Cargo 依赖的自动更新 |
| 1-4 | **Fuzz CI 只覆盖 3/6 个 target** | `fuzz.yml` 只跑了 `formula_jit`/`formula_simd`/`streaming_indicators`，缺少 `formula`/`indicators`/`fuzz_formula_parser` | 补充缺失的 3 个 fuzz target |
| 1-5 | **`bench_report.py` 和 `gen_benchmark_report.py` 功能重叠** | 两者都读取 Criterion JSON 输出并生成 `BENCHMARK_REPORT.md` | 合并为一个，另一个归档 |
| 1-6 | **`benchmark_comprehensive.py` 和 `benchmark_full_coverage.py` 功能重叠** | 两者都是 Python 级 AlphaTA vs TA-Lib 基准测试，数据生成逻辑几乎相同 | 合并或归档冗余 |

### 🟡 P2 — 测试/文档/代码质量

| # | 问题 | 说明 | 建议 |
|---|------|------|------|
| 2-1 | **Go/.NET/Java/iOS/Android 绑定没有任何测试** | 只有 Python 和 C 绑定有测试文件 | 为关键绑定（Go/.NET/Java）增加基本的 smoke test |
| 2-2 | **CLI 没有任何单元测试** | `cli/src/main.rs` 和 `cli/src/csv_io.rs` 无 `#[cfg(test)]` | 为 CLI 增加至少基础测试 |
| 2-3 | **WASM 绑定没有任何测试** | `wasm/src/` 无 `#[cfg(test)]` | 为 WASM 增加基础测试 |
| 2-4 | **`core/tests/common/streaming_test_templates.rs` 内容为空** | 只有一个 `}`，没有实质内容 | 补全或删除 |
| 2-5 | **`api-reference.md` 英文版非常简短不完整** | 只列出了几个函数签名，远不如中文版完整 | 从中文版同步，或删除英文版（仅保留中文） |
| 2-6 | **`docs/src/bindings/README.md` 内容空洞** | 只有 3 行重定向 | 填充实质内容或删除 |
| 2-7 | **`docs/src/reference/README.md` 只是重定向** | 无自己的内容，仅指向根目录的 api-reference 文件 | 填充或删除 |
| 2-8 | **`docs/INDEX.md` 缺少若干文档的链接** | 未链接到 `docs/features.md`、`docs/formula.md`、`docs/formula-debugger.md`、`docs/development.md` | 补全索引 |
| 2-9 | **core 类别 feature 关闭验证未做** | 已建好 `indicators-*` 脚手架，但未逐类别验证跨模块依赖 | 按需增量做（高风险，涉及 ~300 文件） |

### 🟢 P3 — 工程化/清理

| # | 问题 | 说明 | 建议 |
|---|------|------|------|
| 3-1 | **`convert_macros.ps1` 遗留根目录** | 一次性宏迁移脚本，无文档，不在 CI/Makefile 中 | 移至 `scripts/archive/` 或加注释保留 |
| 3-2 | **`python-wheels.yml` 与 `release.yml` 功能重叠** | 两者都构建 Python wheel | 确认是否独立工作流（手动触发用），如是则加注释说明 |
| 3-3 | **Android binding 硬编码版本号** | `lib.rs:108` 写死 `[1.0, 0.0, 0.0]`，应从 `Cargo.toml` 读取 | 用 `env!("CARGO_PKG_VERSION")` 替代 |
| 3-4 | **可视化 crate 只被 Python 绑定使用** | 只有 `python-binding` 导入了 `alpha_ta_visualization`，其他 7 个绑定未使用 | 记录在案（其他语言按需添加，非阻塞） |
| 3-5 | **`packaging/usage/dotnet/tests/obj/` 包含编译产物** | `.cs`/`.cache` 等文件应被 gitignore | 增加 `packaging/usage/dotnet/tests/obj/` 到 `.gitignore` |
| 3-6 | **`winapi: 0.3.9` 旧版依赖** | 应迁移到 `windows-sys`（已存在 `0.61.2` 版本） | 低优先级，仅当需要 Windows 新 API 时迁移 |
| 3-7 | **`thiserror` 存在 2 个版本** | 1.0.69 和 2.0.18 共存，应统一到 v2 | 逐步迁移 |
| 3-8 | **没有 MSRV 检查** | Cargo.toml 声明了 `rust-version = "1.75"`，但 CI 未测试旧版 Rust | 在 CI 中增加旧版本编译验证 |
| 3-9 | **CI 中 `cargo build -p finkit-android` 仍为旧名（已修）** | 上轮已修正为 `finkit-android`，确认生效 | 已修，确认 |
| 3-10 | **`docs/src/` 的 mdBook 构建未在 docs-check.yml 中验证** | `docs-check.yml` 只检查死链和版本一致性，不构建 mdBook | 增加 `mdbook build docs/` 验证步骤 |

---

## 6. 执行路线图

### 第一阶段：P0 安全加固（低风险，建议立即做）

| 动作 | 预计风险 | 依赖 |
|------|---------|------|
| CI 中 `cargo audit` 改为必选（或为已知 CVE 加 ignore 并附原因） | 低 | 需先确认现有 `cargo audit` 结果 |
| CI 中增加 `cargo deny check` 步骤 | 低 | 需修复 `deny.toml` 的 `db-path` 硬编码 |
| 修复 `deny.toml` 的 `db-path` | 低 | 无 |

### 第二阶段：P1 高价值结构性改进（中等风险，按需推进）

| 动作 | 预计风险 | 依赖 |
|------|---------|------|
| 扩展 Android/iOS binding 到更多指标 | 中 | 需熟悉 dispatch_ta + JNI/FFI 调用模式 |
| 增加 Dependabot 配置 | 低 | 无 |
| 补充 Fuzz CI 缺失的 3 个 target | 低 | 需确认 fuzz target 名称和参数 |
| 合并 `bench_report.py` / `gen_benchmark_report.py` | 低 | 需确认哪个是"权威"版本 |
| 合并 `benchmark_comprehensive.py` / `benchmark_full_coverage.py` | 低 | 需确认哪个是"权威"版本 |

### 第三阶段：P2 测试/文档/代码质量（低风险，增量进行）

| 动作 | 预计风险 | 依赖 |
|------|---------|------|
| 为 Go/.NET/Java 绑定增加基本 smoke test | 中 | 需搭建测试框架（可在 Rust 侧用 FFI 调用） |
| 为 CLI 增加单元测试 | 低 | 无 |
| 为 WASM 增加单元测试 | 低 | 无 |
| 处理 `streaming_test_templates.rs` | 低 | 确认是否已被其他测试覆盖 |
| 补全文档索引、空洞的文档文件 | 低 | 无 |
| core 类别 feature 关闭验证 | 高 | 涉及 ~300 文件跨模块依赖分析，需逐步做 |

### 第四阶段：P3 工程化清理（低风险，随手做）

| 动作 | 预计风险 | 依赖 |
|------|---------|------|
| `convert_macros.ps1` 归档 | 低 | 无 |
| 确认 `python-wheels.yml` 用途 + 注释 | 低 | 无 |
| Android binding 版本号硬编码修复 | 低 | 无 |
| `.gitignore` 补充 `packaging/usage/dotnet/tests/obj/` | 低 | 无 |
| 依赖版本统一（`thiserror` v2） | 中 | 需确认无 break change |
| CI 增加 MSRV 验证 | 低 | 无 |
| CI docs-check 增加 mdBook 构建验证 | 低 | 无 |

---

## 7. 命名红线（易踩坑记录）

- core crate lib 名 = `alpha_ta_core`（**无 `[lib]` override**）；visualization lib = `alpha_ta_visualization`。**没有** `finkit_core` / `finkit_visualization`。
- 所有 binding 包名用**连字符** `alpha-ta-*`（如 `finkit-android`），cargo `-p` 不接受 `alpha_ta-android`。
- `cdl_*`/某些图表类指标在部分语言走 dispatcher，不暴露为独立函数——`--check` 只对有 `ffi.bodies.<lang>` 的项判漂移。
- `scripts/sync_bindings.py --rewrite` 重写 lib.rs **必须保留被删 span 之间的空隙**（非注册表函数），否则留悬空引用。
- jni 0.21 破坏性变更：`JNIEnv` 不再 `Clone`；`jdoubleArray` 裸指针不满足 `AsJArrayRaw`。

---

## 8. 结论

`docs/indicator_registry.json` 是全部 8 种 FFI 绑定的单一事实源。新增/修改指标的标准化流程：

```
修改 registry 中 ffi.bodies.<lang> 的函数体
    ↓（或跑 --discover 从手写代码学习）
python3 scripts/sync_bindings.py --generate --rewrite
    ↓
cargo check --workspace
    ↓
git commit（CI 的 --check 覆盖全 8 语言防漂移）
```

**当前最紧迫的改进项**（按优先级排序）：

1. ~~🔴 安全：CI 中 `cargo audit` 改为必选 + 增加 `cargo deny check`~~ ✅ 已修
2. ~~🔴 完整性：Android/iOS binding 指标覆盖不足（仅 6-7 个）~~ ✅ 已扩展（6 → 15：sma/ema/wma/dema/tema/rsi/mom/roc/cmo/trix/midpoint/zscore/tsf/linear_reg/percent_rank；注册表驱动 + dispatch_ta 扩展）
3. ~~🟠 自动化：增加 Dependabot 配置~~ ✅ 已修
4. ~~🟠 测试：Go/.NET/Java 绑定缺少 smoke test；CLI/WASM 无测试~~ ✅ 已修
5. ~~🟠 脚本清理：合并冗余的 benchmark 脚本~~ ✅ 已修
6. ~~🟢 工程化：归档 `convert_macros.ps1`、修复版本号硬编码、补充 `.gitignore`~~ ✅ 已修

*本计划为动态文档，随执行持续更新。所有改动均以"编译/测试验证、不破坏现有构建"为安全网。*