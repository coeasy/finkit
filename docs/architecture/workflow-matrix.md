# 核心工作流与端到端覆盖矩阵

> 审查起点：`8b398999`（2026-10-18）；本地候选工作树验证见 [status-matrix.md](status-matrix.md)，不代表最终提交、远端 CI 或公开发行已验证。
> 产品边界：不含回测引擎、横截面选股或组合构建引擎。

本表按用户能够完成的任务拆分。源码存在、专项测试存在、跨层/安装后闭环、当前提交 CI、公开发行是不同证据，不能互相替代。

| 工作流 | 入口与主要实现 | 输入 → 输出 | 已有验证面（需当前 SHA 重跑） | 贯通判断与剩余工作 |
|---|---|---|---|---|
| Rust 批量指标 | `finkit::indicators`、`BatchIndicator`；`core/src/indicators/` | 序列或 OHLCV → 对齐结果序列 | golden、TA-Lib、API surface、batch/into convergence targets | Rust core 主链路已实现；不同指标的输入合同、NaN/warm-up 和 feature/build 支持仍按家族管理 |
| Rust/Python/Node 公式执行 | `FormulaEngine::{compile,eval,eval_with_dialect,eval_plan}`；Python `formula_eval` / Node `formulaEval` | 源码 + context → 主结果与命名 channel | formula backend、tree/plan differential、dialect/corpus、runtime convergence、binding consumer tests | Tree 默认路径覆盖最广；Plan 可选，存在登记的 Tree-only 入口。Rust/Python/Node 均可选择并查询 backend；其他绑定的 backend 合同另行确认 |
| Rust Streaming | `finkit::streaming`、`StreamingIndicator` | 有序 bar 更新 → 每次更新值/状态 | batch/streaming parity、repaint、long-run targets | 支持的指标主路径存在；目前不是所有 batch/streaming 都经同一 Runtime。需维持 per-indicator capability roster |
| Factor / Composite DAG | `FactorProvider`、`FactorGraph`、`ComputePlan`、`FactorPlan`、`UnifiedRuntime` | 有类型的数据引用与 factor DAG → 节点结果/缓存 artifact | graph contract、operation seam、full/range/DirtyRange tests | Rust 内执行链已实现；完整输入来源、artifact 生命周期和各 adapter 的一致性需按 Runtime contract 验证；不等同于交易组合构建 |
| 特征工程 | `core::features`、`transforms` | 行序列/特征列 → 转换特征、标签、筛选/导出结果 | feature unit tests、future-leakage/label invariants | 模块能力存在；各子模块的时间方向、未来信息使用及导出类型需在用户工作流示例中逐项对齐 |
| Factor Research | `finkit_factor_analysis::api::run_factor_study` / `run_factor_study_json`；prepare → analysis/validation → report | 面板因子、前瞻收益、配置/策略 → 研究报告 JSON | `research_invariants`、API contract、Python/Node request surfaces、CLI consumer test | Rust、Python、Node 和独立 CLI 均有研究入口；本轮补 Python/Node 调用同一版本化 JSON contract 的 consumer test，并为 CLI 补 CSV→报告端到端 test。研究报告不是回测结果 |
| 主 CLI | `finkit-cli`：`sma/ema/rsi/macd/...`、`formula`、`streaming`、transform/pattern commands | CSV / stdin / 参数 → plain/JSON/CSV 或文件 | CLI unit、`schema_cli` targets | 命令行指标/公式/流式主线存在。CSV 路径需要 consumer smoke；主 CLI 未接入 Factor Research |
| Factor Study CLI | `cli/src/bin/finkit-factor-study.rs` | JSON 请求或 CSV → JSON 研究响应 | schema/API fixtures、CSV→报告 consumer test | 独立二进制直连研究 crate；与主 CLI 是两个入口。release workflows 已纳入两个二进制，最终 runner 仍需验证 |
| Python / Node 绑定 | `ffi/python-binding`、`ffi/node-binding`；indicator API 与公式/Factor Research JSON 契约 | ndarray/JS arrays 或版本化 JSON 请求 → Python dict/JS object/JSON 响应 | binding parity、engine contract、factor-study request tests、安装包 consumer workflows | 两个 active binding 都暴露 Factor Study JSON API，指标 parity 比率只覆盖 78 项 indicator。Formula backend 查询/选择现已暴露；其余研究/流式操作仍按各自接口合同判断 |
| 其他绑定 / 移动端 / WASM | `ffi/{c,go,java,dotnet,ios,android}`、`wasm/` | 各语言数值数组/契约 → 本地结果 | binding-specific CI 与 candidate package jobs | surface 和发行层级不同；当前主接口/研究工作流不统一，不应将 CI candidate 表述为公开分发 |
| 发布安装闭环 | Cargo/GitHub Release、Python wheels、language packages、CLI binaries | tag + 精确 SHA → crate/wheels/CLI/native & binding artifacts → clean consumer | release workflows、package validators、release checklist | 当前公开 latest 为 `v0.1.15`，workspace 目标 `0.2.0`。只有 tag 对应的全部 required jobs 完成、资产和 clean consumer 验证后才算发布闭环 |

## 首要闭环缺口

1. **绑定消费验证扩面**：Python/Node backend 与 Factor Study JSON consumer tests 已补齐；剩余各 tier 的公式/streaming/研究能力须明确声明，active-tier clean install 工作流需在最终提交 SHA 上验证。
2. **两个 CLI binary 的发行验证**：release workflows 与 docs 覆盖主 CLI 和 `finkit-factor-study`；当前分支已补 Linux Release assets 和 CSV→报告测试，需在最终 release runner 确认产物。
3. **当前 SHA 证据闭环**：候选本地 workspace 与关键 binding 测试已通过；最终候选提交仍需重跑发布门禁、package/clean-consumer 验证，并按 tag 核对远程 CI 和发行资产。
4. **feature 合同**：类别 feature 当前不能独立 build。保留公开 Cargo feature 名称但明确标成未支持 scaffold；只将 `check_feature_matrix.py` 的 `SUPPORTED` 组合列为用户合同，直到真正修复并测试类别裁剪。

## 入口选择建议

- 对一次完整历史序列：Rust `FormulaEngine` 或 batch indicator API；若需要因子图复用，使用 `FactorPlan` / `UnifiedRuntime`。
- 对逐 bar 更新：`StreamingIndicator` 或有明确 capability 的 streaming API；forming-bar 修订应按指标 contract 使用。
- 对面板研究：`finkit-factor-study` JSON API 或 Rust `run_factor_study`；收益/执行延迟应在 request 中提供，不能把研究报告解释成回测结果。
- 对语言调用：先用该语言公开的 indicator/公式契约；查询 package/readme 确认对应 binding 暴露了该能力与平台产物。

## 关联合同

- [数据流](dataflow.md)
- [公式入口矩阵](formula-entry-matrix.md)
- [因子研究架构](../factor-research-architecture.md)
- [多语言绑定状态](../language-bindings.md)
- [当前实现状态](status-matrix.md)
