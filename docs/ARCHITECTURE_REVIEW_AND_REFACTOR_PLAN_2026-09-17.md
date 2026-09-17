# Finkit 项目功能、架构与生产化重构方案

> 审计日期：2026-09-17
> 审计基线：`perf/outperform-talib-v3-20260904`，提交 `6f6171b`；审计开始时工作区干净，且已与远程目标分支同步。
> 本文是代码审计和重构决策文档，不把目录、类型或测试名称当作“已完成能力”。只有实现、链路和验证同时成立，才计入完成度。

## 1. 结论先行

Finkit 已经具备一个功能面很宽的量化计算内核，但当前状态仍是“多代架构并存的候选生产版”，不是已经收敛的统一因子计算平台。

主要结论如下：

1. **指标批量计算能力较完整。** Core 中已有 TA-Lib 风格指标、扩展指标、技术形态、统计、风险和特征工程；按源码统计，`core/src/indicators` 中有 354 个公开函数，`core/src/streaming` 中有 168 个 `Streaming*` 类型。数量不是完成证明，仍需逐函数确认数值语义、预热长度、NaN、输出形状、流式一致性和跨语言暴露情况。
2. **公式引擎已形成完整雏形，但兼容层仍是“共同子集 + 报告”，不是各终端的完整兼容运行时。** TDX、同花顺、东方财富目前共享 AlphaTA 风格的 canonical parser；`normalize_terminal_source` 主要处理 BOM/换行，没有完成终端专属语义转换。Pine AST 已覆盖声明、`if`、`for`、`while`、`plot`、`fill` 等节点，但这仍是 Pine 子集，不能等价宣称支持 TradingView 全部语言、绘图对象和重绘语义。
3. **主体链路没有完全贯通。** Core 内已有 Formula、Factor、Composite、Streaming、V3/V4 Runtime 多条可运行链路，但它们没有全部落到同一个计划、Kernel、状态、缓存和输出契约上。工作区还存在 `crates/finkit-factor` 与 `crates/finkit-runtime` 两个重复的早期骨架；其中 `finkit-runtime` 的 `FactorFactory::create()` 返回描述字符串而不是可执行 Factor，不能作为生产执行链。
4. **Factor 和 Composite 目前不能证明都满足高吞吐生产要求。** Core 的 borrowed、range、缓存和执行计划是正确方向，但 Factor 注册表使用 `Arc<dyn Fn>`，Composite 使用 `BTreeMap` 和独立缓存，仍有动态分发、重复物化和多套缓存身份的问题；没有统一的 compiled operator/typed state/kernel dispatch 作为唯一热路径。
5. **多语言已有大量绑定，但“语义同 API”尚未完成。** Python、C++、Go、Rust、Java、.NET 都存在绑定或源码入口，但目前是多套手写/半生成 façade。返回类型、错误模型、数组所有权、公式计划句柄、Streaming 状态和研究 API 仍存在语言间差异。Node、Swift、Android/iOS 专用入口不纳入本产品第一阶段正式公开语言范围。
6. **绘图当前是 Rust 自有渲染器，不是 Lightweight Charts 适配器。** 已有 SVG、Canvas、WebGL/WebGPU、PNG、JSON/HTML 输出，适合 native/headless/export；但 Web 前端要采用 Lightweight Charts，应让它成为 Web adapter，不能把 Lightweight Charts 代码塞进 Core 或继续让每个绑定生成一套独立 HTML。
7. **基础门禁已恢复全绿，但仍不能据此宣称全量生产化。** 审计初始验证为 `2882 passed, 5 failed, 1 ignored`；后续已修复 ADX 对齐、Runtime 检查点/状态恢复、注册表快照和版本/SSOT 漂移。当前 `finkit` Core 测试为 `2890 passed, 0 failed, 1 ignored`，版本、SSOT、文档链接检查已通过；公式完整语义、六语言 contract tests、跨语言 golden、性能 SLO 和持久化 typed state 仍未完成。

因此，下一步不是继续增加零散接口，而是先建立一个 canonical compute contract，把 Formula、Factor、Composite、Streaming 和兼容层收敛到同一编译计划与 Runtime，再由各语言和绘图适配器消费这个契约。

## 2. 当前主要功能地图

### 2.1 数值与数据基础层

- `core/src/traits.rs`：`Ohlcv`、`OhlcvBar`、`OhlcvArrayAdapter`、`BatchIndicator`、`StreamingIndicator` 等基础抽象。
- `core/src/runtime.rs`：`MarketFrame`、`SeriesView`、`NanPolicy`、`WarmupPolicy`、输入长度/有限值/对齐验证。
- `core/src/math`：移动平均、统计、回归、分位数、排序、信息量和 V3/V4 kernel 目录。
- `crates/finkit-array`、`finkit-series`、`finkit-math`：新加入工作区的基础包，但目前是轻量独立实现，尚未成为 Core 的唯一数据/Kernel 基础。

V1 数据边界已冻结：

- **多标的：纳入正式链路。** 作为并行批处理维度，例如同时计算一组股票/期货/加密资产的 EMA、公式和因子；每个标的保留独立时间序列、缓存身份和状态，禁止跨标的隐式串行拼接。
- **多周期：纳入正式链路。** 作为明确的 timeframe/frame 维度，支持 1m/5m/1d 等数据源的显式选择、对齐和跨周期引用；不允许未声明的自动重采样或把未收盘的高周期值提前泄漏到低周期。
- **横截面：纳入正式链路。** 作为同一时间点的 symbol 列集合，服务 rank、z-score、winsorize、行业/市值中性化等因子操作；它与单标的时间序列计算是两种不同的执行方向。
- **基本面：纳入输入契约，不纳入数据供应商。** V1 支持带 publication/availability timestamp 的 point-in-time 字段和 as-of 查询，防止使用未来财报修订值；数据抓取、清洗、供应商适配和授权数据管道不属于计算内核范围。

此前代码已经零散存在 Chan 多周期、CrossSectional 因子枚举和 Pine `request.security` 描述，但没有统一的 panel/cross-section/point-in-time 容器，也没有全部接入同一 Planner、cache 和跨语言 API，因此只能计为部分实现。

### 2.2 指标、形态与市场结构

- 批量指标：overlap、momentum、trend、volatility、volume、cycle、statistics、price transform、market 等。
- Streaming 指标：大量 `Streaming*` 状态类型、Builder、checkpoint、泛型浮点支持和重绘相关类型。
- 形态与结构：candlestick/chart patterns、Chan、Chan MTF、sector、selectors、market structure。
- 数据与交易语义：calendar、session、OHLCV/amount/timestamp、warm-up、NaN policy。

### 2.3 Formula 系统

当前路径大致为：

```text
source
  -> parser / dialect parser
  -> AstNode
  -> analysis / params / compatibility
  -> bytecode / compute IR / hot plan
  -> optimizer / SIMD / JIT
  -> FormulaEngine / FormulaExecutor
  -> FormulaResult + DrawResult + debug information
```

已有能力包括：

- 自有 AlphaTA/Finkit 公式语法；
- TDX、同花顺、东方财富兼容入口；
- Pine parser/runtime 子集；
- 表达式、赋值、输出、参数声明、条件控制流；
- 部分循环和状态语义；
- `plot`、`draw text`、`stick line`、颜色和绘图命令 AST；
- bytecode、optimizer、JIT/SIMD 入口；
- full eval、partial/range eval、incremental、parallel、zero-copy、zero-alloc、debug、template、cache 等入口。

问题在于这些入口没有被同一个稳定的 `CompiledPlan` 和能力矩阵统一管理，且兼容性报告可能把“可解析”“已登记”“可执行”“数值等价”混在一个报告体系中。

### 2.4 Factor、Composite、量化研究分析

Core Factor 路径：

```text
FactorContext / BorrowedFactorContext
  -> FactorDefinition
  -> FactorRegistry
  -> FactorEngine
  -> FactorPlan / FactorCatalog / CompiledFactorPlan
  -> UnifiedRuntime
  -> output / range recompute / trace
```

Core Composite 路径：

```text
CompositeDefinition
  -> CompositeExpr / CompositeOp
  -> dependency walk + cycle check
  -> built-in/custom vector function
  -> evaluate / evaluate_borrowed / evaluate_cached
```

`factor-analysis` 进一步提供研究数据、因子挖掘、稳定性、验证、横截面组合分析、情景和报告模块。其生产目标是服务因子计算与量化分析，不负责订单、撮合、回测执行或实时交易风控。

同时存在一套早期独立 crate：

```text
crates/finkit-factor: 仅有 Ema/Sma/Rsi/Macd + 最小 Factor trait
crates/finkit-runtime: FactorFactory/Registry/Executor/Cache/Graph 骨架
```

它们和 Core 的 FactorSystem 不是同一套类型，也没有贯通到 Core Formula/Streaming/研究执行路径。

### 2.5 可视化

当前 `visualization` crate 包含：

- backend-neutral `DrawList`/primitive/scene；
- Kline、line、indicator、Chan、event marker；
- SVG、Canvas、JSON、PNG、HTML、WebGL2/WebGPU fallback；
- viewport、LOD、pan、zoom、crosshair、tooltip、replay、ring buffer；
- 当前还存在 Python、Node、WASM 等 Kline/HTML/JSON 包装；其中 Node/WASM 入口不纳入第一阶段正式公开语言契约，避免继续扩大未收敛的绑定矩阵。

它的优点是无浏览器依赖、可做 native/headless/export；缺点是 Web 交互和图表生态需要在 Rust 字符串 HTML/JS 中重复维护。Lightweight Charts 应作为前端适配层，Core 只输出标准化 series/marker/viewport/tooltip 数据和事件，不应让 Core 依赖浏览器图表实现。

### 2.6 语言与交付入口

当前工作区成员包含 Rust Core、research、visualization、C/C++、Python、Node、Go、.NET、iOS、Java、Android、CLI、WASM，以及 array/series/math/factor/runtime 等新 crate。正式产品公开语言范围冻结为 Rust、Python、C++、Go、Java、.NET；其他绑定暂列实验性或维护性入口，不得进入“全语言 API 一致”的发布门禁。

目标应是“语义同构、类型符合语言习惯”：

- 同一个 operation 名称、参数顺序/名称、默认值、预热/NaN 规则、输出字段、错误码和 schema version；
- Python 使用 NumPy/对象句柄，C++ 使用 RAII/视图，Go 使用显式生命周期包装，Java/.NET 使用托管对象包装，Rust 直接使用类型安全 API；
- 这些只是传输层差异，不能改变计算语义、生命周期、错误分类和能力矩阵。

## 3. 目前不合理或高风险的设计

### P0：多套 Runtime 并存，职责和状态模型重复

当前至少有：

- `core/src/runtime.rs`：输入/NaN/warm-up contract；
- `core/src/unified_runtime.rs`：Factor range execution、dirty range、artifact hash；
- `core/src/runtime_engine.rs`：V4 ExecutionPlan、KernelAdapter、Scheduler、Session、StateArena；
- `core/src/execution_plan.rs` 与 `core/src/unified_executor.rs`：较早的 V3 计划/执行器；
- `crates/finkit-runtime`：另一套 Graph/Factory/Cache/Executor。

这会导致同一指标可能有不同的：计划类型、lookback 定义、状态句柄、缓存 key、执行 trace、错误模型和增量规则。应将它们收敛为：

```text
DataContract -> SemanticIR -> CompiledPlan -> KernelRegistry
             -> RuntimeScheduler -> StateStore/Cache -> Result/Trace
```

旧模块在重构期间只能作为内部兼容 façade，最终公开面只保留一套。

### P0：独立 Factor/Runtime crate 是不可执行的重复骨架

`crates/finkit-runtime::FactorFactory::create()` 返回 `String`，例如 `EMA(period=20)`，不是 `Box<dyn Factor>`、compiled operator 或执行结果；其 Registry 也只做工厂描述和名称查找。`crates/finkit-factor` 仅实现 4 个小指标，与 Core 的 354 个批量函数、168 个 Streaming 类型和 Formula registry 没有统一注册关系。

处理方式：不再继续增强这套旧骨架。要么删除为内部实验包，要么把它改成 Core canonical registry 的薄适配层；生产执行不允许根据字符串反射创建热路径对象。

### P0：Factor/Composite 热路径仍过度依赖动态函数和通用容器

`FactorDefinition` 使用 `Arc<dyn Fn>`，Composite 使用 `BTreeMap<String, Vec<f64>>` 和按 revision 的独立缓存。这对于扩展性有价值，但在每行/每节点高频执行中会带来动态调用、分配、复制和排序开销，也无法与 V4 kernel/state arena 统一。

目标是把动态注册限制在 compile 阶段：

```text
registered definition
  -> validated operator id + typed params
  -> compiled static dispatch / kernel adapter
  -> borrowed input + arena output
```

Factor 和 Composite 都必须使用同一个 `CompiledPlan`、同一个 buffer/state arena、同一个 cache identity 和同一个 execution trace。

### P0：状态检查点实现不满足自身契约

初始基线测试实际失败（已在 V1 分支修复并回归）：

- `runtime_engine::session::tests::node_state_survives_checkpoint_restore`：恢复后出现 `MovingAverageState` 类型不匹配；
- `runtime_engine::state_arena::tests::reused_slot_rejects_stale_handle`：复用 slot 的测试得到 `expected: u64` 类型错误；
- `runtime_engine::state_arena::tests::typed_access_and_checkpoint_restore_round_trip`：恢复 `Vec<u64>` 时类型错误。

当前 `StateArena` 基于 `Any + Clone` 的擦除状态。它适合进程内临时对象，但如果 checkpoint 需要跨版本、跨语言、持久化或跨进程，就不能只依赖 Rust `TypeId`/内存对象。应使用带稳定 `state_type_id`、`schema_version`、payload codec、generation 和 checksum 的 typed state record，并分别支持：进程内 fast checkpoint、可持久化 checkpoint、跨语言 checkpoint。

### P0：数值语义和文档/版本门禁尚未全绿

初始基线测试实际失败（已在 V1 分支修复并回归）：

- `math::kernels::compat::tests::canonical_adx_matches_legacy_public_api`：canonical ADX 与 legacy API 在第 27 行数值不一致；
- `streaming::registry::tests::test_docs_json_matches_registry`：生成注册表与 `docs/indicator_registry.json` 不一致。

当前版本门禁已通过：

- Workspace canonical version 为 `0.1.15`；
- 统一版本源为 workspace `0.1.15`；
- `check_versions.py`、`gen_ssot_docs.py --check`、`check_research_ssot.py` 和 `check_docs_links.py` 已通过。

另外，`cargo +1.98.1 check --workspace --all-features --locked` 不能作为当前版本的有效全功能门禁：该组合同时启用了互斥的 `std` 与 `no_std`，并进一步产生 77 个编译错误（还暴露出部分 feature 组合下的类型/依赖问题）。重构后必须改为一组明确且可支持的构建 profile，分别验证 `std`、`no_std`、公式、SIMD/JIT、并行、绑定等组合；不能用“all-features 通过”作为不成立的生产化结论。

在生产发布前，版本、注册表、函数目录、绑定声明和文档必须由一个生成流程产生，并在 clean checkout 上执行。

V1 分支跟进结果：StateArena 的具体类型恢复、Runtime session checkpoint、canonical ADX 对齐、指标注册表快照和版本/SSOT 同步已经通过专项及 Core 全量测试；这些基础问题已关闭。持久化/跨语言状态 codec 和更高层执行链仍属于未完成项。

### P1：兼容层命名已覆盖，但语义覆盖不足

`FormulaTerminal` 已声明 Finkit、TongDaXin、TongHuaShun、EastMoney、TradingView；但代码明确将中国终端映射到同一 AlphaTA parser，且 `normalize_terminal_source` 不做终端专属语义重写。兼容报告也区分 `Exact`、`Near`、`Approximate`、`Unsupported`。

这不是缺点本身，缺点是公开 API/文档容易让使用者误以为“支持该终端”就是“支持该终端常用全部公式”。必须把能力拆成：

1. lexical/syntax parse；
2. name/alias resolution；
3. type/shape check；
4. causal/lookahead/repaint classification；
5. numeric semantic parity；
6. runtime state parity；
7. drawing/control-flow parity。

每个终端、每个函数、每个语义特性都要有 capability matrix 和 golden corpus。对未实现能力必须在 compile 阶段报告结构化 unsupported，而不是运行时返回空值或“近似成功”。

### P1：控制流、未来数据、重绘、绘图副作用没有统一执行约束

Formula AST 和 Pine AST 已经能表达控制流和绘图，但这些能力会影响：

- 是否可做 incremental/range evaluation；
- 是否会读取未来数据；
- 是否能并行；
- 是否可缓存；
- 是否可重放；
- 是否能在因子分析和增量计算中避免 look-ahead bias；
- 绘图命令是结果数据还是副作用。

应在 IR 节点能力中显式记录 `causal`、`lookahead`、`repaint`、`stateful`、`draw_effect`、`bounded_cost`、`streaming`、`parallel` 和 `range_safe`，由 Planner 决定可用执行路径。

### P1：跨语言 API 不是由同一个契约驱动

目前各绑定大量手写包装，虽然有 `docs/ffi_registry.json`、`scripts/sync_bindings.py`、生成代码和 `ffi-common`，但版本门禁失败说明单一事实源尚未真正闭环。尤其要避免：

- Python 返回 `Vec`，C++/Go 使用视图或显式所有权，而错误/NaN/生命周期解释不同；
- 一种语言有 FormulaPlan/range/latest，另一种只有 full eval；
- 某绑定暴露了某个指标，但 registry/schema 未声明；
- 文档写“支持”但发布流程没有对应 artifact。

应建立 versioned `api-contract`：Rust 类型/IDL/JSON schema 三者只保留一个源，其他均生成；每个语言生成 capability test 和 golden output。

### P1：绘图层把 Web 适配器和渲染后端混在一起

当前 Rust 端生成复杂 HTML/JS 字符串，同时维护 SVG/Canvas/WebGL/WebGPU fallback。这对导出和无浏览器环境有价值，但不适合无限扩展 Web 交互。建议：

- Core/Visualization 只输出 `ChartSceneV1`：series、OHLC、markers、panels、styles、viewport、source index、tooltip payload、events；
- Web 使用 Lightweight Charts adapter，把 `ChartSceneV1` 映射为 series/markers/time scale；
- Rust SVG/Canvas/JSON/PNG 保留给 native/headless/export；
- WebGL/WebGPU 仅作为高密度/特殊场景可选 adapter，不作为每种语言复制的公开逻辑。

### P1：公开模块过多，feature gating 和文档状态复杂

Core `lib.rs` 暴露大量模块，并同时支持 std/no_std、formula、JIT、SIMD、rayon、polars、metrics、多个指标 category。能力很多，但 feature 组合会扩大验证矩阵；`#![allow(missing_docs)]` 和 `#![allow(missing_debug_implementations)]` 也降低了公开 API 演进的可见度。

应按产品层拆包或明确 profile：`finkit-core`（纯计算）、`finkit-formula`、`finkit-runtime`、`finkit-research`、`finkit-visualization`、各 binding；每个 profile 有明确支持矩阵，不再用“全功能默认编译”替代生产发布验证。

## 4. 核心功能与主体链路完成度

| 能力 | 当前判断 | 证据/原因 | 生产化要求 |
|---|---|---|---|
| 批量指标 | 已有较大覆盖，未完成最终核验 | 大量函数、TA-Lib golden、兼容测试存在；ADX parity 失败 | 每个函数的输入/输出/NaN/lookback/数值 golden 与跨语言测试全绿 |
| Streaming 指标 | 部分贯通 | 大量 Streaming 类型和 Builder；不同指标状态模型并存 | 与 batch 共享 kernel/语义，逐指标 batch-stream differential |
| TA-Lib 对标 | 部分实现 | 合同、golden、benchmark 存在；当前 ADX parity 失败，历史 Python benchmark 也显示需谨慎解释 | 固定 TA-Lib 版本、编译器、数据、warm-up、指标列表和误差预算 |
| TDX/同花顺/东方财富 | 共同子集已实现，完整兼容未实现 | 终端枚举和测试存在，但共享 parser、normalize 不做语义翻译 | 分终端 lexer/parser/semantic profile、语义 golden、unsupported matrix |
| Pine 子集 | parser/runtime 已有，非完整 Pine | AST 含控制流/plot/fill，存在 Pine corpus；不能代表完整 TradingView | 固定支持版本/子集，控制流配额、重绘/未来数据声明和 corpus gate |
| Formula 批量执行 | 基本贯通 | parser → AST → bytecode/IR → engine/executor → FFI | 与 Factor/Composite 使用同一 compiled plan、cache、trace |
| Formula incremental/range | 部分贯通 | Engine 有 range/latest/append/zero-copy；能力依赖 state/lookback | 由 Planner 证明 range-safe，checkpoint/state 恢复测试全绿 |
| Formula 绘图 | AST/DrawResult 已有 | drawing AST 和 visualization 均存在 | 将绘图变成结构化 scene effect；Web 由 Lightweight Charts adapter 消费 |
| Factor | Core 可运行，重复 crate 未贯通 | FactorEngine/FactorPlan/UnifiedRuntime 存在；独立 factor crate 只有最小骨架 | Factor 与 Formula 使用同一 IR/kernel/state/cache |
| Composite | 可执行，性能/缓存未统一 | dependency、cycle、borrowed、cached 测试存在 | compiled graph、CSE、arena、统一 cache identity、parallel plan |
| 量化研究分析 | 功能面已存在，发布契约未完全证明 | factor-analysis、ffi-common 和 JSON 入口存在 | 因子研究 request/report/error schema、跨语言 golden、artifact gate；不包含订单/回测/交易风控 |
| Visualization | Rust 后端较完整 | SVG/Canvas/WebGL/WebGPU/JSON/HTML 等存在 | scene schema + Lightweight Charts Web adapter + headless export |
| 多语言 | 绑定广，但语义一致未完成 | 8 个 reviewed façade，版本门禁失败 | 统一 schema/错误/生命周期/能力矩阵和各语言 contract tests |
| 版本/文档 SSOT | 未完成 | `check_versions.py` 和 `gen_ssot_docs.py --check` 失败 | clean checkout release gate 全绿 |

补充说明：Python 侧已有 accessor/strategy 相关测试以 `pytest.skip` 标记为尚未实现。这类入口应在公开 API 清单中明确标为 `planned` 或补齐实现与契约测试，不能仅因模块或测试文件存在就计入完成度。

**主体链路结论：**

- “单一指标 batch → 某个绑定返回值”多数情况下可以运行；
- “Formula → compiled plan → Runtime → state/cache → result/plot → 所有语言”没有完全闭环；
- “Factor/Composite → 同一高吞吐 Runtime → 所有语言”没有完全闭环；
- “兼容终端源码 → 终端语义等价执行 → 统一绘图/控制流/重放”没有完全闭环。

## 5. 最终目标架构

```text
                    Public API Contract / Schema
                               |
             +-----------------+------------------+
             |                                    |
       Language adapters                       CLI / HTTP
             |                                    |
             +-----------------+------------------+
                               v
                    Canonical Request / Error
                               |
                 Data Contract + MarketFrame
                               |
              +----------------+----------------+
              |                                 |
        Numeric API request                Formula source
              |                                 |
              v                                 v
        Operation resolver        Dialect lexer/parser + spans
              |                                 |
              +---------------+----------------+
                              v
                     Semantic IR / Plan
            (Factor, Composite, Formula, Draw effects)
                              |
                     Capability + safety analysis
                              |
                   Compiled ExecutionPlan
              +---------------+----------------+
              |               |                |
        Kernel dispatch   State arena       Cache/materialization
              |               |                |
              +---------------+----------------+
                              v
                 Result columns + DrawScene + Trace
                              |
                              +----------------------+-----------------------+
       |                      |                       |
  Rust/Python/C++/Go/     Lightweight Charts       Native export
  Java/.NET adapters         Web adapter          SVG/Canvas/PNG/JSON
```

### 5.1 Canonical data与结果契约

统一定义：

- `MarketFrame`: symbol、timestamps、OHLCV、amount、timezone/session、revision；
- `FrameKey`/`MarketPanel`: 明确的 symbol + timeframe 地址和零拷贝多帧集合；
- `CrossSectionView`: timestamp × symbol 的行主序矩阵，用于横截面因子；
- `FundamentalSeries`: 按可用时间排序、支持 as-of 的基本面序列；
- `SeriesRef`/`ColumnRef`: 原始列、派生列、数据类型、长度和有效性；
- `WarmupPolicy`、`NanPolicy`、`AlignmentPolicy`；
- `ValueShape`: scalar series、multi-series、event/marker、table/report；
- `ExecutionPolicy`: full、range、incremental、parallel、deterministic；
- `ResultEnvelope`: named columns、metadata、validity mask、trace、warnings、algorithm/schema version；
- `ErrorEnvelope`: stable code、category、message、source span、parameter、retryability。

任何语言的公开 API 都必须能映射到这些字段。语言专属 API 只能是便利方法，不能另造语义。

### 5.2 Canonical registry 与 operation contract

每个指标、公式函数、Factor、Composite builtin、绘图命令都注册为同一类 `OperationSpec`：

- canonical name、aliases、terminal/dialect；
- input names/types/shapes；
- parameter names/types/default/range；
- output names/types/shapes；
- lookback/convergence；
- batch/streaming/range/parallel capability；
- causal/lookahead/repaint/stateful/draw effect；
- numeric reference/accuracy budget；
- algorithm/schema version；
- language exposure status。

Registry、Rust dispatch、FFI 声明、Python stubs、C++ 头文件/RAII wrapper、Go bindings、Java/.NET metadata、文档和测试清单从该源生成。

当前实现状态：`core/src/operation.rs` 已落地 `OperationKind`、`ValueShape`、`OperationCapabilities`、稳定 `OperationId`、别名解析、冲突校验和从现有 `FunctionRegistry` 的原子投影，且已有 3 个单元测试。它现在是统一元数据契约，不代表 Formula/Factor/Composite/Draw 已全部接入同一执行 dispatcher；后续必须逐项补齐 dispatcher、golden 和六语言暴露后，才可将对应 operation 标记为 `implemented`。

### 5.3 Formula 和兼容层

采用“每个 dialect 独立前端、统一后端”的模式：

```text
TDX lexer/parser       ->
THS lexer/parser       -> Canonical Formula IR
EastMoney lexer/parser ->       |
Pine subset parser     ->       +-> semantic profile -> planner/runtime
```

每个 dialect 需要：

1. 词法/语法版本；
2. 名称、常量、数组索引、引用和参数规则；
3. 函数别名与参数重排；
4. `REFX`、`BACKSET`、未来数据、重绘、`SECURITY` 等非因果语义；
5. 终端绘图/控制流映射；
6. 明确 unsupported/approximate 结果；
7. 端到端 golden corpus。

Pine 只承诺明确的固定子集。控制流必须有静态/运行时预算：最大循环次数、最大指令数、最大状态、最大输出对象数量和最大递归深度；网络 `request.*`、账户/订单语义和交易执行不属于本产品范围，必须在编译阶段明确拒绝或标记为 unsupported，不得伪装成普通数值函数。

### 5.4 Factor、Composite 与高吞吐执行

Factor 和 Composite 统一视为 `SemanticOperator`，区别只在元数据和输入输出：

- compile 阶段解析动态名称和参数；
- hot path 使用 numeric `OperationId`、typed params、preallocated column slots；
- CSE 合并相同子表达式；
- borrowed input 默认不复制；
- arena 复用中间列和 state；
- recursive kernel 使用 checkpoint/replay；
- finite lookback kernel 使用 DirtyRange；
- 可并行节点由 Planner 调度，禁止对 stateful/non-deterministic/draw-effect 节点盲目并行；
- trace 记录实际 kernel、行数、分配、cache hit/miss、state restore、fallback 原因。

统一缓存：

```text
CacheKey = operation_id + canonical_params + input_artifact_hash
           + data_revision + range + semantic_profile + algorithm_version
```

不同层不得再各自定义互不兼容的 string key、`(u64,u64)` key 或仅按 symbol/name 缓存。

### 5.5 Runtime、State 和 checkpoint

保留 V4 计划的优点，但只保留一套实现：

- immutable validated plan；
- deterministic topological order；
- `DependencyHorizon::{Fixed, Recursive, Dynamic}`；
- typed kernel adapter；
- state slot + generation；
- in-process checkpoint；
- versioned serialized checkpoint；
- dirty range scheduler；
- replay and reset contract。

State record 至少包含：`plan_hash`、`operation_id`、`state_type_id`、`state_schema_version`、`generation`、payload length、checksum、algorithm version。恢复时先验证计划、版本、类型和校验，再将 payload 解码为具体状态；不允许靠 `Any` 类型猜测持久化身份。

### 5.6 多语言实现策略

优先以 Rust canonical core + 稳定 C ABI/JSON schema 为基础：

- 数值高吞吐路径：typed pointer/length、borrowed view、explicit output buffer；
- Formula/Factor compiled plan：opaque handle，公开 compile/evaluate/evaluate_range/evaluate_last/reset/checkpoint；
- 低频研究/调试/兼容报告：versioned JSON；
- Python：NumPy zero-copy view + owned result；
- C++：RAII wrapper + `std::span`/显式 owned result；
- Go：显式 ownership/free 和稳定错误码；
- Java/.NET：托管 wrapper + deterministic dispose/finalizer fallback。

所有绑定必须通过同一套 cross-language golden vectors 和 error/lifecycle tests。

### 5.7 V1 目标公开 API 契约

六种正式语言必须围绕同一组语义入口实现，名称可以符合语言习惯，但不能改变参数含义、默认值、预热规则、NaN 规则、输出字段或错误分类。目标公开面收敛为以下五类：

```text
Catalog
  list_operations / get_operation_spec

Data dimensions
  frame(symbol, timeframe)
  panel.insert(frame_key, market_frame)
  cross_section(timestamps, symbols, values)
  fundamental(name, publication_timestamps, values)

Numeric indicator
  compute(operation, inputs, params, execution_options)

Formula
  compile(source, dialect, compile_options)
  plan.evaluate(frame, execution_options)
  plan.evaluate_range(frame, range, execution_options)
  plan.evaluate_last(frame, execution_options)

Factor / Composite
  compile_factor(definition, compile_options)
  compile_composite(definition, compile_options)
  plan.evaluate(frame, execution_options)

Streaming / persistence / drawing
  stream.push(frame_or_bar)
  stream.checkpoint() / stream.restore(checkpoint)
  result.columns / result.metadata / result.warnings / result.draw_scene
```

实现约束：

- Rust 是 canonical implementation 和类型契约来源；C ABI 是跨语言边界，C++ 在其上提供 RAII、span/view 和异常安全的习惯化封装。
- Python、C++、Go、Java、.NET 的公共入口必须覆盖 batch、compiled plan、range/latest、streaming、checkpoint、结果读取和结构化错误；不能出现某语言只能 full evaluate 的降级版本。
- `ResultEnvelope`、`ErrorEnvelope`、`OperationSpec`、`ChartSceneV1` 使用统一 schema；语言绑定只负责内存视图、对象生命周期和语言习惯包装。
- 数值结果默认支持命名列、validity mask、warm-up metadata、algorithm/schema version 和 warnings；不得以不同语言的空值、异常或 NaN 约定替代统一语义。
- 编译句柄、流式句柄和 checkpoint 都必须显式拥有 owner、线程安全属性、释放方式和版本验证规则；跨语言不得暴露 Rust `TypeId` 或裸内部指针作为稳定契约。
- 旧 `FactorFactory`、独立 `finkit-runtime`、重复的 Formula/Composite façade 只保留为迁移适配层，不能继续增加新的公开能力；完成迁移后删除或降为内部模块。

## 6. 分阶段重构执行方案

### Phase 0：冻结契约和清理基线（必须先做）

1. 选定最终 workspace/package/API/schema version，不再混用 0.1.5 与 0.1.15。
2. 生成并校验所有 registry、version matrix、binding metadata、文档。
3. 修复 ADX parity、StateArena checkpoint、Runtime session restore、registry snapshot。
4. 建立 `cargo fmt/check/test/clippy`、版本、SSOT、链接、FFI memory、跨语言 golden 的统一 release gate。
5. 对每个公开 operation 标注 `implemented / partial / planned / unsupported`。
6. 将 `FrameKey`、`MarketPanel`、`CrossSectionView`、`FundamentalSeries` 纳入 canonical data contract，并为六种语言生成一致的数据维度 API。

**Gate：** 核心测试全绿、版本/SSOT 全绿、失败项不能以修改断言或降低误差预算解决。

### Phase 1：收敛 Core contract 与 registry

1. 定义 `OperationSpec`、`Request`、`ResultEnvelope`、`ErrorEnvelope`、`Capability`。
2. 将指标、Formula builtin、Streaming、Factor、Composite、Draw command 接入同一 registry。
3. 把 `MarketFrame`、series alignment、warm-up、NaN、revision 固化为唯一 contract。
4. 生成多语言声明、文档、测试矩阵；删除手写重复 metadata。

**Gate：** 任一 operation 可从 registry 查到同一参数、输出、能力和版本；Rust/Python/C++/Go/Java/.NET contract tests 一致。

### Phase 2：统一 Semantic IR 与 Planner

1. Formula、Factor、Composite 都 lower 到同一 Semantic IR。
2. 保留 source span、dialect、semantic profile、effect、lookahead、repaint、stateful 等信息。
3. 编译为同一 `ExecutionPlan`，支持 CSE、lookback/horizon、range safety、parallel safety 分析。
4. 旧 `FactorPlan`、`CompositeEngine`、旧 `FormulaEngine` 变成 façade，内部调用新 Planner；完成迁移后移除重复热路径。

**Gate：** 同一表达式通过 Formula、Factor、Composite 入口得到一致结果、trace 和 cache identity。

### Phase 3：统一 Kernel、State、Cache

1. 将移动平均、统计、波动率、RSI、ADX、ATR、MACD、成交量等高频族迁移到统一 kernel registry。
2. 实现 scalar/SIMD/parallel 后端选择，但由同一 operation contract 验证输出。
3. 用 typed state codec 替换 Any-only checkpoint；加入跨进程序列化测试。
4. 统一 full/range/incremental/checkpoint/cache 路径。
5. 建立分配、吞吐、延迟、内存、并行扩展和回退原因的基准指标。

**Gate：** batch/stream/range/checkpoint-restore 数值一致；Factor/Composite 在生产数据规模上通过明确 SLO；cache 命中不改变结果。

### Phase 4：兼容系统生产化

1. TDX、同花顺、东方财富各自建立 lexer/parser/semantic profile 和常见函数 catalog。
2. Pine 固定目标子集版本，逐项实现声明、series、namespace、控制流、绘图、`request.security` 边界和重绘限制；strategy/order/alert/external execution 不属于产品范围。
3. 建立来自真实公开公式的 corpus，区分 parse、compile、numeric、streaming、drawing、incremental 六类测试。
4. 兼容报告默认显示“不支持/近似/非因果/需要宿主能力”，禁止静默降级。

**Gate：** 每个宣称支持的函数都有实现和 golden；未支持特性有稳定错误码；不同语言的兼容报告字段一致。

### Phase 5：多语言和研究 API 一次性收敛

1. 从 schema 生成 Rust crate API、C ABI/C++ headers and wrappers、Python stubs、Go bindings、Java/.NET declarations。
2. 暴露统一的 formula/factor/composite request、compiled handle、streaming handle、checkpoint、result/error API。
3. 量化研究 API 使用 versioned JSON request/report/error；低频操作不复制各语言业务逻辑，不扩展到订单、回测或交易风控。
4. 每个 binding 增加 clean environment smoke test、memory ownership test、NaN/warm-up test、golden test。

**Gate：** 同一 request 在各语言返回语义等价结果；包版本、schema 版本、算法版本和 release asset 一致。

### Phase 6：绘图与 Web 适配

1. 定义 `ChartSceneV1`，从 Formula DrawResult、Chan、Factor/Composite series 和 event marker 统一生成。
2. 实现 Lightweight Charts Web adapter：OHLC、line、histogram、markers、pane、time scale、tooltip、viewport 和增量更新。
3. 保留 Rust SVG/Canvas/PNG/JSON/headless backend；WebGL/WebGPU 作为可选高密度 adapter。
4. 禁止各语言绑定各自生成不同版本的图表业务逻辑；绑定只输出 scene 或调用 Web adapter。

**Gate：** scene JSON 可被 Web/native/headless 三类 renderer 消费；源索引、时间、marker、tooltip 和增量更新一致。

### Phase 7：生产发布与运维

- reproducible build、SBOM、checksum、签名和 ABI/FFI 兼容声明；
- benchmark 绑定 commit、编译器、CPU、数据集、TA-Lib 版本和 warm-up；
- metrics/tracing 记录 operation、plan、cache、state、fallback、错误；
- fuzz Formula/Pine parser、bytecode、checkpoint、FFI pointer/lifecycle；
- 发布前执行 workspace、所有 feature profile、目标平台和外部项目 smoke tests。

## 7. 必须补充的测试矩阵

### 数值正确性

- TA-Lib 固定版本 golden：SMA、EMA、RSI、MACD、BBANDS、ATR、ADX、全部已声明对标函数；
- 终端公式 golden：TDX、同花顺、东方财富、Pine；
- batch vs streaming vs range vs checkpoint-restore；
- NaN、Inf、空输入、短输入、period 边界、重复时间戳、错序时间戳；
- f64/f32、scalar/SIMD、单线程/rayon。

### 计划与 Runtime

- dependency order、cycle、CSE、lookback、recursive/dynamic horizon；
- cache key 隔离和 revision invalidation；
- checkpoint serialize/deserialize、版本拒绝、类型拒绝、stale handle；
- deterministic trace、取消/超时/内存限制、状态 reset。

### 兼容与安全

- source span 错误、unsupported structured error；
- 控制流上限、未来数据检测、repaint 分类；
- Pine/公式绘图命令不影响数值结果；
- FFI 空指针、长度、所有权、异常/恐慌、重复 free；
- C ABI、opaque handle 和跨语言 JSON schema。

### 性能

- single indicator、shared dependency、Factor graph、Composite graph、Formula graph；
- 1K/10K/100K/1M/10M bars；
- full vs range vs append；
- cold compile vs warm compiled plan；
- cache hit/miss、allocation count、RSS、CPU scaling、p95/p99 latency。

## 8. 已确认边界与仍待确认问题

以下边界已由产品方确认，后续重构按此执行，不再将其作为阻塞问题：

- 正式公开语言固定为 Rust、Python、C++、Go、Java、.NET；Node、Swift、Kotlin、WASM 等不进入第一阶段正式 API 一致性承诺。
- 产品核心固定为公式系统、TA-Lib 对标、经典公式兼容、Factor/Composite 因子计算器和量化计算能力。
- Web 绘图优先采用 Lightweight Charts；Rust renderer 保留为 native/headless/export 后端，不能反向成为 Web 业务层。
- V1 正式支持多标的、多周期和横截面计算；基本面先支持 point-in-time 输入契约与 as-of 语义，不建设数据抓取和供应商适配。
- 不建设订单、撮合、交易执行、回测、滑点/交易成本模拟或实时交易风控 Runtime。因子分析中的 look-ahead/repaint 检测仍保留，因为它属于计算语义正确性。
- 目标是全面超越 TA-Lib，但“超越”必须用可复现的函数覆盖率、数值一致性、吞吐、延迟、内存和跨语言一致性基准证明，不能作为未经验证的宣传结论。

仍需确认、且会影响公开 API 或兼容语义的问题如下：

1. **版本策略：** 是继续以 workspace `0.1.15` 为起点，还是直接切换新的产品版本/协议版本？算法版本、schema 版本、binding package 版本是否允许独立递增？
2. **TA-Lib 对标范围：** 是否要求覆盖 TA-Lib 全部公开函数，还是以常见指标、candlestick、math/statistics 全 catalog 为第一阶段范围？NaN、warm-up 和误差预算是否以指定 TA-Lib 版本为绝对基准？
3. **终端兼容等级：** TDX、同花顺、东方财富是按各自常见公开函数追求数值等价，还是先冻结 common subset？同名不同义函数是否必须由源码显式指定终端 profile？
4. **Pine 边界：** 目标 Pine 语言版本和首批子集是什么？对 repaint、lookahead、`request.security` 是默认拒绝、显式 opt-in，还是允许 approximate 并强制输出警告？
5. **Factor/Composite SLO：** 目标吞吐（bars/sec）、p95/p99 延迟、内存上限、并发任务数、最大图节点数、最大历史长度和可接受编译延迟是多少？
6. **数据模型补充确认：** V1 已按多标的、多周期、横截面进入正式链路，基本面采用 point-in-time 输入契约；仍需确认第一版是否同时纳入 open interest、fundamental vendor adapters，以及 cross-sectional 的行业/市值中性化等高级字段。
7. **发布形态：** 六种语言是否都要求同步发布公共包，还是先统一源码/CI/golden 验证，再按语言分阶段发布？这会决定 ABI、构建矩阵和版本门禁的严格程度。

## 9. 推荐的决策顺序

在收到上述确认前，不继续扩大指标数量或重复迁移旧接口。推荐顺序是：

```text
确认公开边界
  -> 冻结 schema/version
  -> 修复 5 个失败测试与版本/SSOT 门禁
  -> 收敛 Registry/IR/Runtime/State/Cache
  -> 统一 Formula/Factor/Composite
  -> 终端兼容和 Pine corpus
  -> 生成多语言 API
  -> Lightweight Charts adapter
  -> 性能、发布、安全和运维 gate
```

在 Phase 0 和 Phase 1 完成前，不应对外宣称“全部核心功能已实现”“所有语言 API 完全一致”“TA-Lib/各终端完全兼容”或“Factor/Composite 已达到生产高吞吐”。
