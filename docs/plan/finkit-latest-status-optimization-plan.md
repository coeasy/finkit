# Finkit 最新代码核对、功能梳理与优化改进方案

- 执行更新：2026-09-02：PR #11 已合并 v0.1.2 发布元数据与 CI/文档修复，PR #12 已将旧 PR #1 中仍缺失的 Factor Engine、Runtime Contract、Function Registry 和终端公式兼容层移植到当前主线；本轮继续修复多语言绑定入口、Java JNI 结果对象、FFI panic 隔离、iOS 头文件契约和 workspace 编译门禁。正式 v0.1.2 Release 仍待处理。

- 核对日期：2026-09-02
- 仓库：coeasy/finkit
- 核对分支：main
- 核对基线：2c2ba4d6805bd9eb2176b14edb461a955aad1f67
- 源码版本：0.1.2
- 当前可见 GitHub Release：v0.1.0
- 方案目标：先完成 0.1.x 稳定化、发布闭环和 API 契约，再扩大功能面

> 本文区分“源码/API 已存在”“文档已声明”“测试已覆盖”和“可发布”。只有四者同时闭环，功能才视为可对外承诺。

## 0. 2026-09-02 最新执行记录

- 当前 `main`：`2c2ba4d`；PR #11/#12 已合并，PR #13 保持开放；本轮修复分支已继续追加绑定契约、JNI 结果类型和 CI workspace-check 修复。
- PR #1 的旧 v0.1.0 发布工作流、旧版本元数据和重复代码未带入主线。
- 新增核心公共模块：Factor Engine、Runtime/MarketFrame、Function Registry、FormulaTerminal compatibility。
- 本轮继续修复 Function Registry 别名冲突、Factor 空名称、Runtime Preserve/Error 借用路径、MarketFrame 别名查询分配，以及 CI Cargo.lock/Docs Check 漏检，并复核 Core Contracts 示例与实际签名；同时将 .NET csproj/Java pom 纳入版本检查与 SSOT 版本矩阵。
- 当前 GitHub Release 仍只有 `v0.1.0`；历史 CI/Python wheels 运行曾在 runner preflight 阶段失败，本轮已移除会令真实校验整体跳过的单点前置，下一次运行需重新观察实际编译/测试结果。

## 1. 结论摘要

### 1.1 最新代码已经从 v0.1.0 基础版推进到 v0.1.2

main 最近的提交集中在以下方向：

- 可复用 Formula Plan、持久化 Bytecode/JIT 缓存和增量上下文；
- 公式执行热路径减少分配，并增加 eval_into、缓存和 BufferPool 复用；
- Python ABI3 wheel 矩阵、跨 Python 版本兼容和版本一致性检查；
- CI、发布版本门禁、文档和基准测试补强；
- RSI AVX-512 种子归一化等回归修复。

因此，当前项目已经具备“高性能指标库 + 公式运行时 + 多语言绑定”的产品骨架，不应再按只有 v0.1.0 基础设施的状态规划。

### 1.2 当前最大问题不是缺少新功能，而是发布闭环尚未完成

核对到的发布阻断项：

1. workspace、Python、Node 和生成文档已经统一到 0.1.2，但 GitHub Releases 当前仍只有 v0.1.0；
2. main 最新 CI 和 Python wheels 工作流均失败；失败作业返回的 steps 为空，当前连接器无法取得实际日志，因此不能臆断具体根因；
3. README 已同步为 ci.yml、python-wheels.yml 和 docs-check.yml；perf-gate/fuzz 仍属于后续门禁；
4. 仍保留多条历史 feature/fix/release 分支，所有核对到的分支均未保护；
5. PR #1 已关闭；其仍有价值的核心模块已通过 PR #12 移植，当前没有开放 PR。

结论：核心模块补齐后，v0.1.2 继续以发布闭环、API 契约和可验证性为主，暂不扩大默认 feature。

### 1.3 公式运行时是当前最值得沉淀的差异化能力

当前 Formula 模块已形成较完整的执行链：

- AlphaTA/TDX 风格公式和 Pine Script v5 子集解析；
- AST、Bytecode、优化器、CSE、常量折叠、死代码消除和 lookback 分析；
- FormulaEngine、FormulaExecutor、LRU 公式缓存、Bytecode 缓存、VM scratch 和 BufferPool；
- eval、eval_range、eval_last、eval_into、partial、lazy、parallel、batch、incremental 等入口；
- FormulaPlan / Python CompiledFormula 的重复调用、append_bar、range 和 last 场景；
- debugger、templates、sandbox、drawing 和参数校验能力。

但“zero-copy”目前是有边界的性能路径，不是整个公式引擎的统一保证：

- Python eval 会把输入复制到可复用的 owned stream context；
- eval_zero_copy 只接受连续的 float64 一维 NumPy 数组；
- MA、EMA、RSI、BOLLMID 等直接常见路径可以借用输入；
- 复杂公式仍可能因为现有数组型内建函数 ABI 产生中间数组；
- Bytecode 的 LoadVariable 当前仍会把输入切片复制为新的 Array1；
- CompiledFormula 标记为 unsendable，线程/进程使用方式需要明确文档。

这部分应优先做“语义透明 + 可测量”，再继续追求极限性能。

## 2. 当前功能全景

### 2.1 Rust 核心和指标体系

| 能力 | 当前代码证据 | 当前判断 |
|---|---|---|
| Rust workspace | core、visualization、CLI、WASM、C/Python/Node/Go/.NET/Java/Android/iOS 等成员 | 已形成多 crate 架构 |
| no_std 与 feature gating | core/src/lib.rs、core/Cargo.toml | 已具备，可继续收敛最小依赖 |
| 批量指标 | overlap、momentum、volume、volatility、cycle、statistics、patterns、market、China/A-share 等模块；README 声明 150+ | 核心能力较完整，需统一计数口径 |
| Streaming 指标 | StreamingIndicator、builder、registry、ring buffer、rolling min/max 及多类具体指标；README 声明 98 | 源码能力存在，但生成文档只列 24 个直接导出的 public struct，统计口径需要修正 |
| K 线与模式 | 经典蜡烛图、chart patterns、SMC、Fibonacci、Ichimoku、Renko/Kagi/P&F 等 | 功能面广，需加强分类和稳定性等级 |
| 批量/并行 | batch、parallel、rayon、Polars 扩展 | 已有性能扩展，需避免默认 feature 过重 |

### 2.2 Formula Engine

建议将现有能力对外统一描述为以下流水线：

~~~text
OHLCV / 参数
    -> 输入校验与数据布局
    -> 方言解析与 AST
    -> 类型、依赖和 lookback 分析
    -> 优化器与 CSE
    -> FormulaPlan / Bytecode
    -> Batch、Range、Last 或 Incremental Runtime
    -> 结果、变量和诊断信息
~~~

当前已存在的公共能力包括：

- 公式解析：AlphaTA/TDX 默认方言、Pine 方言；
- 编译执行：AST、Bytecode VM、优化 Bytecode；
- 缓存：编译缓存、Bytecode 缓存、运行时 scratch；
- 运行模式：完整序列、半开区间、最后一根、增量、并行、批量、部分求值；
- 工具链：模板、调试、绘图、验证、参数定义和 sandbox；
- Python：formula_eval、formula_eval_dialect、formula_eval_bytecode、formula_eval_optimized、formula_eval_zero_copy、formula_eval_numpy_zero_copy、CompiledFormula。

需要注意两个命名风险：

- Cargo feature 中 formula-simd 当前是 Bytecode executor 的薄别名，不应对外表述为所有公式均已 SIMD 化；
- formula-jit 仍属于有限/实验性路径，应在能力矩阵中标明支持范围、回退逻辑和真实收益。

### 2.3 Feature Engineering、因子相关和相邻模块

当前代码已包含：

- FeatureMatrix、FeatureSet、FeatureEngine；
- rolling stats、normalization、labels、regime、market structure、microstructure、PCA、Fourier、wavelet、GARCH、selection、importance、time features 等源码模块；
- backtest、risk、sector、selectors、transforms、Polars 和 visualization 等相邻模块。

当前已补齐独立的 `core/src/factors.rs`、`core/src/runtime.rs`、`core/src/registry.rs` 和 `core/src/formula/compat.rs` 公共模块。因此：

- Finkit 已具备一个 Rust 侧 dependency-aware Factor Engine、对齐的 MarketFrame/runtime policy、函数元数据 registry 和终端公式兼容入口；
- 这些能力目前仍主要是 Rust core contract，尚未完成 C/Python/Node 等绑定的统一暴露和跨语言 golden fixture；
- 不能把它们描述成已经完成的跨语言 Factor SDK；下一步应先补 parity、错误契约和安装 smoke test。

建议保持核心边界：指标、公式和数据变换是核心；回测、风险、选股和可视化作为可选扩展，不能反向污染最小安装。

### 2.4 多语言和工具链

| 入口 | 当前能力 | 主要验收重点 |
|---|---|---|
| C ABI | finkit.h 提供版本、状态码、指标输出缓冲区、模式与可视化句柄 | 指针/长度、所有权、错误和 ABI 稳定性 |
| Python | ABI3、CPython 3.8–3.14 Linux 兼容目标，NumPy API，CompiledFormula | wheel 真机安装、GIL、unsendable、连续数组和错误类型 |
| Node | N-API、平台包和 TypeScript 声明 | loader 与平台包命名、四平台安装 |
| Go/.NET/Java/Android/iOS | workspace binding crate 和元数据 | 是否有可下载工件、API/错误/版本一致性 |
| WASM | wasm crate 与 npm 目标 | 浏览器/Node 产物、内存和输入格式 |
| CLI | finkit-cli crate，支持公式、streaming、features、sweep、templates 和指标命令 | 二进制名称、帮助文档、CSV/JSON 输出契约 |
| 基准与质量 | Criterion、TA-Lib 对比、golden、differential、property、fuzz、长稳测试 | 将结果纳入发布门禁，而不是只生成报告 |

## 3. 成熟度和改进点

| 领域 | 成熟度 | 已有基础 | 主要缺口 |
|---|---|---|---|
| 批量指标 | A：源码和测试均有 | 指标分类多、TA-Lib 对比和 golden 测试 | 计数、NaN/warm-up、性能结果和版本口径需统一 |
| Streaming | B：源码较完整 | O(1)/bar 设计、builder、checkpoint、repaint、registry | README 历史口径 98、registry 145 与生成文档直接扫描 24 曾不一致；现已拆分说明，仍需继续收敛为同一 SSOT 展示 |
| Formula | B：核心已成形 | AST/Bytecode/优化/缓存/增量/zero-copy 入口 | 需要把 ownership、copy、lookback、last 语义写成稳定契约 |
| SIMD/JIT | C：能力存在但宣传需收敛 | feature、AVX-512、JIT 入口和基准 | SIMD 当前覆盖有限，JIT 仍偏实验；缺少能力矩阵和性能门槛 |
| Feature Engineering | B：源码有、文档展示不全 | FeatureSet、FeatureMatrix 及多种变换 | generated/features.md 只列 3 个 direct public submodule；现已补充 internal re-export 说明 |
| Factor API | C：Rust core 已具备初版 | FactorContext/Registry/Engine、依赖缓存、cycle 检测和 composite | 尚未跨语言暴露；需补 golden fixture、错误契约和参数/窗口 metadata |
| Python 发布 | B：构建配置有 | ABI3、平台矩阵、Python 3.8–3.14 兼容 job | 最新 wheels 工作流失败；当前可见 release 仍是 v0.1.0 |
| 其他 FFI | B/C：代码与元数据有 | 多语言 binding crate、C header、Node 平台包 | 缺统一 golden fixture、安装 smoke test 和工件清单 |
| CI/CD | C：门禁正在补强 | fmt、clippy、test、doc、audit、版本检查 | 最新 main 仍未绿；README/workflow 名称已完成同步 |
| 文档/SSOT | C：文档量大 | generated docs、version matrix、indicator registry | 生成范围、计数口径、版本和工作流说明存在漂移 |
| 仓库治理 | C | 分支和 PR 历史完整 | main 未保护；历史分支和旧 PR 未清理 |

## 4. 优化改进方案

### P0：恢复可发布性

#### P0-1. 先定位并修复 CI / wheels 的前置失败

现象：main 最新 CI 和 Python wheels run 失败，失败 job 的 steps 为空；通过当前 GitHub 连接器无法取得底层日志。

执行项：

1. 重新运行一次 CI 和 Python wheels，记录 run、attempt、job、runner image 和 failure annotation；
2. 保留轻量 workflow-health 信息作为诊断，但不得让任何实际构建/测试 job 依赖单点 preflight；当前 CI 已改为独立校验，并新增 workspace 全量 compile check；
3. 将版本检查独立成 version-consistency job，并在 test job 中保留一次调用；
4. 对 workflow action 版本、权限、concurrency、runner image 和 Python 路径做最小化排查；
5. 只有核心 CI 和 wheels 全绿后才允许创建 v0.1.2 Release。

验收标准：

- fmt、clippy、core test、doc、version consistency、audit 均有明确结果；
- Python wheel build、四平台校验和 CPython 3.8–3.14 compatibility job 均完成；
- 失败时能在 job summary 或日志中直接定位到具体 step。

#### P0-2. 完成 v0.1.2 版本、tag、release 和文档闭环

执行项：

1. 确认 v0.1.2 tag 指向经过 CI 的 main commit；
2. 创建真正的 GitHub Release v0.1.2，并上传 Rust crate、四个平台 wheel、SHA256SUMS 和 CLI 工件；
3. 校验 README、CHANGELOG、docs/installation、docs/python、version-matrix、Node 平台包和 Cargo.lock；
4. 删除或修正所有仍指向 v0.1.0 的安装、下载和发布说明；
5. 发布后用干净环境执行 Python、Node、Rust 和 CLI 安装 smoke test。

验收标准：

- 版本检查脚本通过；
- Release 页面、下载工件、安装文档和包管理器版本完全一致；
- release asset 数量、平台 tag 和 ABI3 tag 与矩阵一致；
- 不把“源码版本为 0.1.2”误写成“已发布 v0.1.2”。

#### P0-3. 统一 README 与真实工作流

二选一并执行到底：

- 补齐 README 中承诺的 release、perf gate、fuzz、docs-check 工作流；或
- 按仓库真实存在的工作流改写 README，删除不存在的工作流名称。

建议保留独立工作流：

- ci.yml：格式、静态检查、核心测试、文档、依赖审计；
- python-wheels.yml：构建、兼容性、工件校验、发布；
- docs-check.yml：生成文档和 SSOT 差异检查；
- perf-gate.yml：选定基准相对基线的回归阈值；
- fuzz.yml：定时和手工触发，避免每次提交消耗过多资源。

#### P0-4. 收敛分支和 PR

执行项：

- 已关闭仍描述 v0.1.0 的 PR #1，并通过 PR #12 保留其未合并的核心模块；
- 标记历史分支为 archived，合并确认后删除已无用途分支；
- 对 main 开启 branch protection：必需 CI、版本检查、docs check 和至少一名 review；
- 发布分支只从经过检查的 main/tag 产生，禁止旧 release 分支重新覆盖发布线。

### P1：建立稳定 API 和 SSOT 契约

#### P1-1. 一个版本源、一个文档生成入口

当前已有 scripts/check_versions.py 和旧的 scripts/check-versions.sh 两套版本检查路径。建议：

- 以 scripts/check_versions.py 作为唯一实现；
- shell 文件只做兼容包装，或在确认无调用方后删除；
- scripts/gen_ssot_docs.py 统一生成版本矩阵、指标、streaming、feature、公式函数和兼容性文档；
- CI 由 docs-check job 对所有生成文件执行 git diff --exit-code，而不是只检查 indicator_registry.json；
- 将“公共导出”“内部实现”“注册可用”“文档展示”分成四个字段，避免用一个数字代表所有功能。

#### P1-2. 统一指标、Streaming 和 Feature 的计数口径

至少输出四类计数：

1. batch public API 数量；
2. streaming concrete indicator 数量；
3. registry 中可构造/可执行数量；
4. 文档直接展示的 struct/module 数量。

生成文档应明确：

- 98 是哪一层的数量；
- streaming-indicators.md 中的 24 是直接扫描到的 public struct，还是完整可用指标；
- features.md 中的 3 是 public module，还是生成器目前识别到的模块；
- README 的 150+、98、60+ 是否能由同一 SSOT 生成。

#### P1-3. 把 Formula Runtime 契约写成可测试规范

为 FormulaPlan 和 FormulaEngine 固化以下字段：

| 契约 | 必须明确 |
|---|---|
| 输入 | OHLCV 必选项、amount 可选、长度一致、空数组和非连续数组处理 |
| 输出 | __result__、变量名、变量顺序、CSE 临时变量是否隐藏 |
| warm-up | 每个指标的 lookback、前置 NaN/None 规则、range 是否自动扩展依赖窗口 |
| range | [start, end) 语义、越界行为、结果长度和上下文副作用 |
| last | 是完整重算最后一项、依赖缓存，还是增量状态读取 |
| append | 是否允许跳过 bar、是否支持 amount、容量扩展、reset 和 checkpoint |
| ownership | owned eval、borrowed zero-copy、复杂公式中间数组的边界 |
| 并发 | CompiledFormula unsendable 的限制；Rust Engine 是否可跨线程；Python 多进程建议 |
| 错误 | 语法、参数、数据布局、运行时、unsupported kernel 的稳定错误类型 |

必须增加的回归测试：

- full eval 与 eval_range 拼接结果一致；
- eval 后 append_bar + eval_last 与重新计算全历史最后一项一致；
- reserve_bars 不改变结果；
- 连续与非连续 NumPy 输入的成功/失败行为稳定；
- 直接 zero-copy 和普通 owned 路径数值一致；
- 复杂公式允许分配，但不能出现未文档化的隐式输入拷贝；
- Bytecode、optimized、JIT、SIMD 路径在支持范围内结果一致。

#### P1-4. 区分 bounded ring buffer 与 append-growing history

当前 RingBuffer 是固定容量、面向流式状态的结构，不等同于 FormulaContext 的可增长完整历史。

建议明确两种数据策略：

- bounded streaming state：只保留计算所需窗口，强调 O(1)/bar 和内存上界；
- append-growing formula history：保留完整序列，支持 range、last、append 和可复用上下文。

每个 API 必须声明使用哪一种策略，避免用户以为 streaming buffer 能提供完整历史回溯。

#### P1-5. 重新定义 SIMD/JIT 的对外承诺

建立 formula capability matrix：

| 路径 | 当前状态 | 对外承诺 |
|---|---|---|
| AST | 稳定回退 | 支持全部已注册语义 |
| Bytecode | 稳定主路径 | 支持全部已编译语义 |
| optimized bytecode | 稳定增强 | 在等价性测试通过时启用 |
| zero-copy | 受限优化 | 只对连续 float64 和已支持 kernel 承诺 |
| SIMD | 局部优化 | 只列出实际覆盖的指标/操作 |
| JIT | 实验性/受限 | 明确编译条件、回退和收益，不作为默认稳定保证 |

### P2：跨语言一致性和性能治理

#### P2-1. 一套跨语言 golden fixture

使用同一份输入、公式、参数、预期输出和错误样例，覆盖：

- Rust core；
- C ABI；
- Python；
- Node；
- Go/.NET/Java（具备可运行构建环境时）；
- WASM；
- CLI JSON/CSV。

输出契约至少包括：

- 数值数组；
- warm-up 区间；
- NaN/Infinity；
- 错误码和错误消息类别；
- 版本和能力标识。

#### P2-2. 绑定发布验收

每个平台都做真实安装，而不仅是编译：

- Python：四个平台 wheel 安装，CPython 3.8、3.11、3.13、3.14 冒烟；
- Node：安装主包和平台 optional dependency，验证 native loader；
- C：编译最小消费者程序，检查版本和释放接口；
- CLI：验证二进制名称、--help、CSV/JSON 输出；
- WASM：Node 和浏览器兼容 smoke test；
- 其他 binding：至少验证 artifact 可下载、版本和一个指标调用。

#### P2-3. 性能优化遵循“先等价、后提速”

固定小型 perf gate：

- SMA、EMA、RSI、MACD、BBANDS、ATR；
- 公式 full、range、last、append；
- zero-copy 与 owned；
- 单条和批量；
- 10K、1M、10M 数据规模；
- 分配次数、峰值内存、吞吐、p95 延迟。

建议将性能结果拆为：

- correctness gate：任何数值漂移先阻断；
- allocation gate：只约束明确标记为 zero-allocation 的路径；
- throughput gate：相对基线允许合理波动，例如先设 10% 回归告警，再逐步收紧。

### P3：明确长期产品边界

建议产品主线固定为：

1. 指标计算；
2. 流式指标；
3. 公式解析和执行；
4. Feature/因子构建基础；
5. 多语言调用和 CLI。

backtest、risk、sector、selectors、visualization 和 Polars 保持可选模块。只有在 Factor API 的概念、生命周期和跨语言需求已经明确后，才新增独立 Factor DAG，而不是继续把相邻研究能力直接放进 core 默认 feature。

## 5. 分阶段落地顺序

### 阶段 0：发布止血

- 修复或定位 CI / wheels 前置失败；
- 让 main 全部必需检查变绿；
- 统一 0.1.2 tag、Release、wheel、文档和包版本；
- 更新 README 的工作流描述；
- 处理旧 PR 和分支。

完成标志：可以从干净环境安装并运行 core、Python、Node 和 CLI 的最小示例。

### 阶段 1：契约和 SSOT

- 合并版本检查脚本；
- 全量生成文档并加入 docs-check；
- 统一 batch、streaming、feature 的计数；
- 发布 Formula Runtime API 契约；
- 完成 full/range/last/append/zero-copy 等价性测试。

完成标志：文档生成无差异，README 数量均来自 SSOT，所有公式执行模式有明确测试。

### 阶段 2：跨语言 parity

- 落地 golden fixture；
- C/Python/Node/CLI 优先完成；
- 其余 binding 至少完成版本和最小指标调用；
- 固化错误码、warm-up 和输出命名。

完成标志：同一输入在各入口的结果、错误类别和版本信息一致。

### 阶段 3：性能收敛

- 先优化 Formula Plan 的重复执行；
- 重点验证 Bytecode LoadVariable、复杂公式中间数组和 BufferPool；
- 对直接 kernel 增加 allocation/throughput gate；
- 对 SIMD/JIT 逐项扩覆盖，并保留稳定回退；
- 继续维护 TA-Lib 和竞争库对比，但用固定环境和可复现实验数据。

完成标志：无正确性回归，关键路径性能达到基线目标，性能报告可由 CI 复现。

### 阶段 4：下一版本规划

只有在 v0.1.2 稳定后再评估：

- 独立 Factor DAG；
- 更完整 Pine/TDX 兼容；
- 真正的公式 SIMD/JIT 覆盖；
- checkpoint/restore 跨语言；
- 更广平台和包管理器发布；
- breaking API 的 v0.2 版本。

## 6. 发布和 CI 验收清单

### 核心质量门禁

- cargo fmt --all -- --check
- cargo clippy -p finkit
- cargo clippy --workspace --all-targets
- cargo test -p finkit
- cargo test -p finkit --doc
- cargo doc -p finkit --no-deps
- scripts/check_versions.py
- scripts/gen_ssot_docs.py --check
- dependency audit
- golden、differential、property、fuzz 和 long-run 测试

### Formula 门禁

- parser/optimizer/bytecode 结果一致；
- full/range/last/append 结果一致；
- direct zero-copy 与 owned 结果一致；
- 非连续 NumPy 输入按契约报错；
- 复杂公式的中间分配可解释、可测量；
- JIT/SIMD 不支持时稳定回退；
- 线程模型和 Python GIL 行为有测试或明确限制。

### 发布门禁

- workspace、Cargo.lock、Python、Node、所有平台包和文档版本一致；
- tag、Release 和 main 基线一致；
- 目标平台 wheel 数量、文件名、ABI3 和 manylinux/macOS/Windows tag 正确；
- Python 兼容矩阵通过；
- C/Node/CLI 安装 smoke test 通过；
- SHA256SUMS 可复核；
- README、CHANGELOG 和安装文档指向真实可下载版本；
- main 已开启必需检查和 review 保护。

## 7. 优先级 Backlog

| 编号 | 优先级 | 工作项 | 验收结果 |
|---|---|---|---|
| FINKIT-REL-01 | P0 | 定位 CI 和 wheels 的空 steps 失败 | 失败可定位到具体 step，main 恢复全绿 |
| FINKIT-REL-02 | P0 | 完成 v0.1.2 tag/Release/assets | Release 和安装矩阵真实可用 |
| FINKIT-REL-03 | P0 | README 与实际 workflow 对齐 | 不再引用不存在的工作流 |
| FINKIT-GOV-01 | P0 | 处理旧 PR、分支和 main protection | 主线受保护，发布路径单一 |
| FINKIT-SSOT-01 | P1 | 合并版本检查和生成文档入口 | 一个 canonical checker，一个 docs gate |
| FINKIT-SSOT-02 | P1 | 修正指标/streaming/feature 计数 | README 与 generated docs 同源 |
| FINKIT-FORM-01 | P1 | 固化 Formula Runtime 契约 | full/range/last/append/zero-copy 有规范和测试 |
| FINKIT-FORM-02 | P1 | 补充 zero-copy copy boundary 诊断 | 用户能知道何时借用、何时复制 |
| FINKIT-FORM-03 | P1 | 区分 RingBuffer 和完整历史上下文 | 内存策略和生命周期不再混淆 |
| FINKIT-PAR-01 | P2 | 跨语言 golden fixture | Rust/C/Python/Node/CLI 结果一致 |
| FINKIT-PERF-01 | P2 | 性能和分配门禁 | 关键 benchmark 可复现且回归可阻断 |
| FINKIT-API-01 | P3 | 评估独立 Factor DAG | 形成单独 RFC 后再决定是否进入 v0.2 |

## 8. 近期执行顺序

1. 先恢复 CI 和 Python wheels；
2. 再发布 v0.1.2 并做干净环境安装；
3. 同步 README、CHANGELOG、版本矩阵和 workflow 文档；
4. 将版本检查和 SSOT 文档检查收敛成正式门禁；
5. 补 Formula 全路径等价性和 zero-copy 语义测试；
6. 最后推进跨语言 parity 和性能门禁；
7. v0.1.x 稳定前，暂停扩大默认 feature 和新增大型相邻模块。

该顺序的核心原则是：先让已经存在的能力可验证、可安装、可解释、可回归，再决定下一轮功能扩展。
