# 05 · GitHub 上传准备（Checklist）

> 本文基于对仓库现状的核查结果编写。目标仓库：`https://github.com/coeasy/finkit.git`。

## 0. 当前状态（已核查）

- ✅ `.git` 已初始化，当前分支 `main`，**尚无任何提交**（全部文件为 untracked）。
- ✅ remote 已配置：`origin -> https://github.com/coeasy/finkit.git`。
- ✅ **仓库身份已统一**：根 `Cargo.toml` 的 `repository/homepage`、README 徽章、各 FFI 绑定 URL（Cargo/pyproject/package.json/pom.xml/csproj/go.mod）已全部改为 `coeasy/finkit`。
- ⚠️ **`.github/workflows/ci.yml` 为补齐项** —— 本套整理流程新建最小 `ci.yml`（fmt+clippy+test+doc+audit）；README 声明的 release/perf-gate/fuzz/docs 等高级工作流可后续补充。
- ⚠️ **`Cargo.lock` 已不再忽略并纳入提交**（含多个可执行目标的 workspace 应提交锁文件）。
- ⚠️ README/文档引用的**缺失文档**已补建（见 §5），保持链接有效。

---

## 1. 仓库命名与元数据统一

- ✅ **已决策：统一为 `coeasy/finkit`**（与远端 remote 一致）。
- ✅ 全部 `github.com/alphata-rs/{alpha-ta,alphata,alpha_ta}` 引用已批量改为 `github.com/coeasy/finkit`，范围覆盖：根 `Cargo.toml`、`README.md`、`PROJECT_SUMMARY.md`、`ffi/*`（Cargo/pyproject/package.json/pom.xml/csproj/go.mod、各绑定 README）、`docs/{api-reference,development,installation}.md`。
- 保留不变的命名：crate 名 `alpha-ta-*`、Python 包 `alpha_ta`、npm 包 `@alphata/core`、Java groupId `com.alphata`——这些是"包/产物"身份，与"托管仓库路径"解耦。

## 2. 许可证一致性（README 是双许可证声明）

- 当前根目录有 `LICENSE` 与 `LICENSE-APACHE`，**但没有 `LICENSE-MIT`**。
- README License 段写成双许可证（Apache-2.0 OR MIT），并链接到 `LICENSE-MIT`（该链接现在会 404）。
- **建议**：补一个 `LICENSE-MIT`（放 MIT 全文），并保留 `LICENSE-APACHE`；将 `LICENSE` 文件明确为二选一说明或指向二者。这样与 `Cargo.toml` 的 `license = "MIT OR Apache-2.0"` 完全对应。

## 3. 补全缺失的 CI/CD（`.github/`）

README/PROJECT_SUMMARY 描述了这些工作流，但 **`.github/workflows/` 不存在**。上传前建议添加：

| 工作流 | 用途 | 关键步骤 |
|--------|------|---------|
| `ci.yml` | 主门禁 | fmt --check、clippy（default/no_std/all-features）、`cargo test --workspace`、`cargo doc -D warnings`、`gen_ssot_docs.py --check`、`cargo audit`、WASM build、FFI 冒烟（3 平台矩阵） |
| `release.yml` | 多语言发布 | 构建并发布 7 语言包（Python/Node/Java/Go/.NET/WASM/C） |
| `perf-gate.yml` | 性能回归 | 对比基准，变化 >10% 告警/失败 |
| `fuzz.yml` | 每周模糊 | 3 targets × 300s |
| `docs-check.yml` / `docs-deploy.yml` | 文档 | mdBook（根 `docs/`）校验 + GitHub Pages 部署 |

> 若希望"开箱即用"，可先补一个最小的 `ci.yml`（fmt + clippy + test）即可满足基本发布需求，其余按需补齐。

## 4. 提交 `Cargo.lock`（重要）

- `.gitignore` 第 2 行忽略了 `Cargo.lock`。对**含可执行/绑定产物、追求可复现构建的仓库**，惯例是提交 `Cargo.lock`。
- 仓库包含 `cli` 二进制 + 多个 FFI dylib/cdylib 产物，**建议不再忽略 `Cargo.lock`**，并删除 `.gitignore` 中的 `Cargo.lock` 行，上传时提交之。
- 若坚持忽略，则 CI 需在干净环境 `cargo update`，可复现性变差。

## 5. 修复 README / docs 失效链接

README 引用了以下文档，但**当前不存在**（会造成死链与观感问题）：
- `docs/QUICK_START.md`、`docs/WIKI.md`、`docs/BUILD_GUIDE.md`、`docs/ONE_CLICK_BUILD.md`、`docs/benchmark-results.md`、`docs/MIGRATION.md`、`docs/RELEASE_NOTES_v2.0.md`、`docs/optimization-continuation-2026-06-20.md`（在 `.trae/` 下，gitignore 后必不存在）。

处理方式（任选）：就地创建这些文档，或把 README 表格里的链接改指向实际存在的文档（如 `docs/INDEX.md`、`docs/installation.md`、`docs/BENCHMARK_REPORT.md`、本套 `docs/code-wiki/`）。

## 6. 清理"内部/开发机"痕迹（开源前务必）

这些内容面向内部 AI 辅助开发，上传公共仓库前最好收拾干净：

- `docs/AGENTS.md` / 根 `WORKFLOW.md` 记录了 `E:\agent_learn\AlphaTA` 的绝对路径与 AzaLoop MCP 工作流，属于内部工具约定，建议**改写为通用贡献指南**或移除。
- 大量 `OPTIMIZATION_PLAN.md`、`REFACTORING_PLAN*.md`、`UPGRADE_PLAN_2026.md`、`PLANNING.md`、`PROGRESS.md`、`PRD.md` 为过程性文档；公开前可筛选保留（Wiki/API/基准等），把一次性的规划文档移入 `docs/archive/`（已 gitignore 该目录）或删除，避免仓库显得杂乱/暴露开发过程。
- 灰色脚本（`scripts/debug_*.py`、`_diag_braces.py`、`_a5_scan.py` 等大量 debug/临时脚本）：建议清理到 `scripts/archive/`（已 gitignore）或按需保留少量可维护的脚本。
- **扫描并移除硬编码凭据**：`resources_table`/token/密钥在提交前用工具全仓库扫描（`git log`、grep 私有 token、AK/SK）。
- `.gitignore` 已排除 `.trae/`、`.aza/`、`.agentvault/`（AI 本地状态），无需担心提交，但**首次 `git add` 前**建议先检查 `git status`，确保不会误加。

## 7. 收尾提交步骤

```bash
cd p:\github_public\finkit

# 0) 统一仓库名/元数据（§1），补 LICENSE-MIT（§2），建 .github/workflows（§3），
#    不再忽略 Cargo.lock（§4），修复 README 死链（§5），清理内部痕迹（§6）

# 1) 预览将被提交的内容，确认无大文件/凭据
git status
git add .
# 或分目录精提交：git add Cargo.toml core/ cli/ ffi/ docs/ ...

# 2) 检查暂存内容大小/敏感信息（可选）
git status

# 3) 首次提交
git commit -m "Initial commit: AlphaTA financial technical analysis library"

# 4) 推送并设上游
git branch -M main
git push -u origin main
```

## 8. 上线前的额外加分项

- **仓库根放 `NOTICE` 或"第三方声明"**（若依赖存在再分发注意项）。
- 确认 `deny.toml` 覆盖的依赖许可证检查在 CI 中跑通（`cargo deny check`）。
- 在 GitHub 仓库页配置：Description、Topics（如 `technical-analysis`、`rust`、`ta-lib`、`finance`）、License（选 Apache-2.0/MIT）、README 展示。
- 为 `main`/`master` 配置 **Branch Protection**（PR 合入 + CI 必过）。
- 视需要给 `crates.io`/PyPI/npm/Maven/NuGet 的发布工作流配好 secrets（`CARGO_REGISTRY_TOKEN` 等）。

---

本套 Code Wiki 完。入口：[00-索引与导览.md](./00-索引与导览.md)