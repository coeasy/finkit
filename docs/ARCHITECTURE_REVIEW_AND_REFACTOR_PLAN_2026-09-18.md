# Finkit 统一公式与因子计算引擎：现状审计与最终重构方案

> 审计日期：2026-09-18  
> 当前开发分支：`feature/finkit-v1-unified-engine-20260917`  
> 目标：公式系统、TA-Lib 对标、因子/Composite 计算与 Lightweight Charts Web 适配器的生产化；不纳入订单、撮合、回测或交易风控 Runtime。

## 1. 结论

Finkit 已经拥有较宽的指标、公式、因子、流式和多语言代码基础，但当前仍属于“功能分散、契约正在收敛”的版本，不能把源码数量、函数注册表或绑定目录当作全部生产能力。

当前最重要的判断：

1. Core 指标计算链已经可运行，公式函数与 Operation Catalog 已接入统一执行入口。
2. TA-Lib 显式 profile 已有版本化的 JSON Operation Contract；本轮已将 161 个 profile 名称接入共享 catalog/dispatcher，其中完整 61 项 candlestick 目录已逐项真实调用验证。
3. Rust、Python、Go、Java、.NET、C、C++、Node 的目标应当是同一份 Rust Core + FFI Contract，而不是八套各自定义语义的 API。当前共享入口已经存在，但各语言高层类型、发布流程和跨语言 golden 仍需要继续统一。
4. Formula 的 TDX、同花顺、东方财富兼容语义、Pine 子集、绘图和控制流都是长期核心范围；它们必须进入同一个编译计划和能力矩阵，不能仅停留在 parser 或函数名登记层。
5. Factor 和 Composite 都必须支持高吞吐生产运行，但不得通过继续复制 Factory/Registry/Executor 接口解决。应统一为 typed IR、compiled plan、kernel dispatch、状态和缓存身份。
6. “全面超过 TA-Lib”只能通过逐函数数值对照、warm-up/NaN/边界/流式一致性、性能基准和跨语言结果一致性证明；目前不能宣称已经完成。

## 2. 产品边界与公开 API

### 2.1 正式范围

- TA-Lib 风格指标、扩展指标、统计、数学、周期、成交量、价格变换和完整 candlestick 目录。
- TDX、同花顺、东方财富风格公式：表达式、赋值、引用、窗口、条件、绘图、信号控制和常见市场数据函数。
- Pine 子集：声明、表达式、series 语义、条件、循环、状态变量、plot/fill/shape/line 等可映射绘图语义；不承诺完整 TradingView 私有对象和全部平台行为。
- Factor、Composite、Streaming、批量/range/incremental 计算。
- 多标的、多周期、横截面和带 point-in-time/as-of 约束的基本面输入。
- Lightweight Charts Web 前端适配器；native/headless 输出继续保留为导出和测试能力。

### 2.2 明确不纳入统一 Runtime

- 订单、撮合、账户、回测执行、交易风控、交易路由和数据供应商服务。
- 自动猜测周期、隐式重采样、跨标的串接状态和未声明的未来数据引用。

### 2.3 公开语言与统一接口原则

正式公开语言为：Rust、Python、Go、Java、.NET、C、C++、Node.js。

统一 API 分为两层：

```text
Canonical Rust Core
        |
Versioned FFI Contract (JSON/control plane + typed/buffer data plane)
        |
Rust / Python / Go / Java / .NET / C / C++ / Node adapters
```

所有语言必须共享以下概念和字段：

- `FormulaSource`、`FormulaDialect`、`CompiledFormula`；
- `OperationRequest`、`semantic_profile`、`input_order`、`inputs`、`params`；
- `FactorDefinition`、`CompositeDefinition`、`ExecutionPlan`；
- `ValueShape`：series、multi-series、cross-section、event、report；
- `WarmupPolicy`、`NanPolicy`、错误码、版本号和输出名称；
- `CacheKey` 中的 operation/formula、参数、symbol、timeframe、data revision、semantic profile。

JSON 适合作为控制面和跨语言一致性测试面；高吞吐数据面必须使用 typed buffer/zero-copy ABI，不能让 JSON 成为每个 bar 的热路径。

## 3. 当前主要架构

当前代码可以概括为：

```text
OHLCV / panel / fundamental inputs
              |
              +--> Formula parser / dialect parser --> AST / analysis
              |                                           |
              |                                           v
              |                                      FormulaEngine
              |
              +--> Function Registry --> Operation Catalog --> UnifiedOperationEngine
              |                                            |
              |                                            v
              |                                      Factor / Composite paths
              |
              +--> Streaming indicators / state / checkpoints

Unified result contract --> FFI common --> Rust/Python/Go/Java/.NET/C/C++/Node
                          --> Lightweight Charts adapter
```

已存在的核心模块包括 `core/src/formula`、`core/src/operation`、`core/src/registry`、`core/src/factors`、`core/src/runtime`、`core/src/streaming`、`core/src/patterns`、`ffi/ffi-common` 和各语言 binding。它们已经形成多个可工作的局部链路，但还不是所有场景都经过同一个 compiled plan 和同一个执行状态。

## 4. 不合理设计与重构判断

### 4.1 Registry、Factory、Provider 多代接口并存

早期 `crates/finkit-runtime` 的 Factory/Executor/Cache 骨架与 Core FactorSystem 不是同一套类型；让 Runtime 继续直接创建具体 Factor 会重新引入 crate 耦合和重复注册表。

最终方案：

- Core 负责 canonical Function/Factor/Composite 定义、编译和执行；
- Runtime 只接收已编译的 `ExecutionPlan`/`CompiledFactorPlan`；
- 插件通过注册 metadata、builder 或 provider 注入，不让热路径依赖字符串 Factory；
- 早期重复 crate 要么降级为迁移工具，要么删除，不能继续作为第二套生产 Runtime。

### 4.2 “已注册”与“可执行”混淆

一个名字出现在 Registry 不代表它拥有真实实现、正确输入、正确 warm-up 或数值等价语义。每个 operation 必须有：

```text
catalog metadata
 -> typed dispatcher
 -> formula/batch implementation
 -> streaming state (若声明支持)
 -> golden/reference test
 -> every-language contract test
```

Catalog 应明确标注 `registered`、`executable`、`streaming`、`reference_verified`、`cross_language_verified`，不能用一个 boolean 代表全部状态。

### 4.3 动态分发和重复物化仍可能进入热路径

`Arc<dyn Fn>`、`BTreeMap`、重复的 Vec materialization、逐节点字符串查找适合控制面，不适合作为高吞吐 Factor/Composite 的唯一热路径。

重构为：

1. 编译阶段把名称解析为 typed operator id。
2. 规划阶段完成依赖拓扑排序、输入绑定、输出槽位、lookback 和 state layout。
3. 执行阶段按 kernel id 和连续 buffer 运行，动态分发只发生在 plan 边界。
4. Composite 复用同一 dependency graph、cache key 和 buffer allocator。
5. Streaming 与 batch 使用同一套 golden 语义；只允许状态存储方式不同。

### 4.4 兼容语义没有完全分层

TDX、同花顺、东方财富和 Pine 不能只靠函数别名解决。必须分成：

- Lexer/parser dialect；
- AST/HIR canonicalization；
- series、未来函数、引用、窗口、短路、赋值和状态语义；
- 绘图/控制流 lowering；
- profile-specific numeric policy；
- compatibility report 和 unsupported diagnostic。

同名函数如 `REF`、`CROSS`、`SUM`、`MA`、`COUNT`、`BARSLAST`、`VALUEWHEN` 必须由 dialect/profile 显式定义输入、边界、预热、空值和未来数据策略。

### 4.5 绘图输出不应与计算内核耦合

计算内核输出 typed series、markers、plots、panels、viewport hints 和 metadata；Web 适配器将其转换成 Lightweight Charts 数据结构。绘图对象不能反向改变计算结果，也不能把浏览器状态写入因子状态。

## 5. 目标架构

```text
Data adapters
  -> CanonicalFrame / MarketPanel / CrossSection / PointInTime fundamentals
  -> explicit temporal alignment (Exact / AsOfClosed)

Formula source or operation request
  -> dialect parser
  -> canonical AST
  -> typed HIR / semantic profile
  -> dependency graph + lookback/state analysis
  -> optimized ExecutionPlan
  -> batch / range / streaming / panel executor
  -> kernel dispatch + unified cache
  -> ResultEnvelope / DrawEnvelope / diagnostics
  -> FFI adapters / Lightweight Charts adapter
```

### 5.1 Plan 与 Runtime

`ExecutionPlan` 是唯一生产执行入口，至少包含：

- plan schema/version、semantic profile、source hash；
- typed inputs、output slots、operation ids；
- lookback、warm-up、NaN/null policy；
- state layout、checkpoint schema、determinism flags；
- dependency graph、kernel selection、parallelization hints；
- cache namespace 和 data revision requirements。

Runtime 只负责调度、生命周期、缓存、状态、取消、诊断和结果封装，不负责凭字符串猜测 Factor 类型。

### 5.2 数据维度

- 单序列：指标和公式的默认形状。
- 多标的：`symbol` 是缓存、状态和结果的显式维度，不能拼接成一条时间序列。
- 多周期：必须携带 source/target timestamp，并显式选择 Exact 或 AsOfClosed。
- 横截面：在同一 timestamp 的 symbol 列集合上执行 rank、z-score、winsorize、中性化等操作。
- 基本面：只接受带 publication/availability timestamp 的 point-in-time 输入；数据供应商不属于引擎。

### 5.3 Cache

统一 CacheKey：

```text
semantic_profile + operation/formula id + parameter hash
+ symbol + timeframe + input revision + range + state revision
```

批量结果、增量结果和 checkpoint 必须使用同一身份规则。缓存命中不能绕过输入时间对齐、版本校验和 NaN policy。

### 5.4 公式与 TA-Lib profile

TA-Lib profile 使用显式版本化名称（当前代码为 `talib_0_7_1`，对应 TA-Lib C core 0.7.1），不把 Core warm-up 语义伪装成 TA-Lib 语义。当前 parity corpus 已升级到 TA-Lib Python 0.8.0；每个公开 TA-Lib operation 仍必须有参数默认值、MA type、输出名、warm-up、NaN 和参考结果说明。当前 catalog 已补齐 profile-only 条目的可执行参数元数据，APO、BBANDS、MAVP、MACDEXT、PPO、STOCH、STOCHF、STOCHRSI 的 `matype` 已由共享 dispatcher 实际消费；剩余指标仍必须逐项完成同等数值验证，不能只在目录中声明。

版本依据：TA-Lib C core `0.7.1` 的官方 release 记录为 2026-07-03，Python 包 `0.8.0` 在 PyPI 的 release 记录为 2026-09-13；本仓库的 profile 和 golden reference 分别对应这两个版本。[TA-Lib core releases](https://github.com/TA-Lib/ta-lib/releases) · [TA-Lib Python 0.8.0](https://pypi.org/project/TA-Lib/0.8.0/)

Formula profile 也必须版本化：

```text
core_formula_v1
tdx_v1
ths_v1
eastmoney_v1
pine_subset_v1
talib_0_7_1
```

不兼容旧接口时，新增 profile 直接升级主版本并删除旧 profile，不在热路径保留隐式兼容分支。

## 6. 分阶段实施计划

### Phase 0：契约冻结与审计

- 固定 operation/formula/result/error/schema 版本。
- 生成唯一 catalog，禁止语言 binding 手写另一份函数名。
- 对每个能力区分 parse/register/execute/reference/cross-language 状态。
- 建立 TA-Lib、TDX、同花顺、东方财富和 Pine 的语义差异矩阵。

### Phase 1：真实执行闭环

- 完成 `request -> catalog -> typed dispatcher -> compute -> result -> cache`。
- 所有 multi-output 使用稳定输出名。
- 完成完整 TA-Lib 常用目录和 candlestick 目录的 operation/golden 测试。
- 错误统一为结构化 code/message/details。

### Phase 2：Formula 编译计划

- parser 输出 canonical AST，按 dialect 进行显式 lowering。
- 增加 typed HIR、lookback/state/control-flow analysis。
- 将 plot、draw、fill、alert、signal 等 lowering 为 Draw/Control IR。
- Pine 子集先冻结可执行边界，再扩展 `request.security` 等跨周期能力。

### Phase 3：高吞吐 Factor/Composite

- 统一 dependency graph、plan compiler、kernel registry、state layout、cache。
- Factor 与 Composite 同时支持 batch、range、incremental、streaming。
- 以 allocation、吞吐、延迟、确定性和 checkpoint 恢复为生产门禁。

### Phase 4：八语言统一发布

- Rust 作为 canonical typed API。
- C ABI 作为 C/C++ 稳定底层 ABI，C++ 提供 RAII/header wrapper，不复制数值逻辑。
- Python、Go、Java、.NET、Node 只做类型转换、生命周期和错误映射。
- 每个语言运行相同 JSON/typed-buffer golden；验证输出名称、null、错误码和版本完全一致。

### Phase 5：Lightweight Charts 与生产门禁

- 计算层输出 OHLC、volume、line、markers、panels、layers、viewport 和增量更新 envelope。
- Web adapter 负责 series、pane、tooltip、crosshair、resize 和 update；不承担公式计算。
- 增加浏览器集成测试、数据点顺序测试、null/warm-up 测试和大数据量更新基准；其中 adapter 的 Node fake-chart contract test 先作为无浏览器依赖的确定性门禁。

### 本轮已落地的架构收敛增量

- Formula 的 `FormulaEngine`、Operation engine 和 FFI formula 入口统一走 `eval_with_dialect` / `eval_multi_with_dialect`；TDX、同花顺、东方财富先经过统一 transport normalization，再进入各自显式 dialect profile，Pine 走 Pine parser/lowering。
- CLI 不再自行维护 AlphaTA/Pine 的分支匹配，直接调用核心 dialect 执行入口；终端 schema 现在明确返回 `alpha_ta`、`tdx`、`ths`、`eastmoney`、`pine`，避免把不同兼容契约伪装成同一方言。
- Operation panel cache 已改为带访问时钟的 LRU 淘汰；Composite cache 纳入 `scope`、`data_revision` 和 graph signature，增加 scoped evaluation，防止不同标的/周期在相同 revision 下串缓存。
- Unified Operation Engine 的 Factor 默认路径已改为 `FactorCatalog -> CompiledFactorPlan -> borrowed execution`，并缓存编译计划；这只证明主路径已接入 compiled plan，不代表所有 Factor/Composite、streaming 和跨语言高吞吐门禁已经完成。
- Composite 默认路径已增加 `CompiledCompositePlan`：定义校验、引用/cycle 检查和 graph signature 在计划阶段完成，Operation Engine 按 graph signature 复用计划；结果缓存仍额外受 scope/data revision 约束。
- Python 的公开 `formula_eval_dialect` 已与其他绑定统一调用 Core 的 `eval_with_dialect`；不能再让 Python 自己把国内 dialect 静默降级为 AlphaTA。
- Node 绑定已补齐 `operationCatalogJson`，与 C/C++、Go、Java、.NET、Python 共用同一 operation catalog 和 `operationExecuteJson` contract；Node 的宿主级加载仍需在真实 Node addon 环境中验证。
- Composite 已补齐 `composite.contract.v1` JSON contract：输入为 named series + graph definitions + outputs，结果统一返回 `shape/primary/values/schema_version`；C/C++、Go、Java、.NET、Python、Node 都有对应入口，避免 Composite 只在单一语言高层 API 中存在。
- Factor 已补齐 `factor.contract.v1` JSON contract：所有正式绑定都可以执行稳定的内置因子并获得 compiled-plan 的 `semantic_identity/range_lookback`；Rust typed API 仍保留自定义闭包因子，跨语言 contract 不把不可序列化闭包伪装成可移植定义。
- Factor 已补齐 `factor.catalog.v1` JSON discovery contract：C/C++、Go、Java、.NET、Python、Node 与 Rust FFI common 共用同一份内置因子目录，公开名称、类型、方向、依赖、版本及 streaming/incremental 能力，执行入口与发现入口不再断开。
- Factor 与 Composite 的 v1 请求现在强制要求 `schema_version`，并拒绝重复 Factor target；新增 `tests/contracts/engine_contract_v1.json` 将 Formula/Factor/Composite 的请求与期望输出固定为同一份跨语言 conformance vector，避免各 binding 分叉维护示例和数值语义。
- Formula compatibility report 已提升为 `formula.compatibility.v1` 共享 JSON contract：Rust、Python、Go、Java、.NET、C、C++、Node 均通过同一报告结构输出 parser、batch/streaming、control flow、drawing、cross-timeframe、lookahead、host data；各绑定只负责转发、生命周期和错误映射。能力矩阵只报告已验证的执行边界，不把 parser 识别或函数登记误报为完整兼容。
- Lightweight Charts adapter 已修复增量 payload 中动态新增 line 不创建 series 的问题；`visualization/frontend/lightweight-charts-adapter.test.mjs` 已覆盖 null/warm-up 空白点、markers、viewport、增量更新、完整替换和 schema 拒绝。
- TA-Lib `MINMAX` 与 `MINMAXINDEX` 已加入 registry、core multi-output dispatcher、TA-Lib FFI profile 和 operation catalog；输出名固定为 `MIN/MAX` 与 `MININDEX/MAXINDEX`，并有 JSON execution tests。
- TA-Lib profile catalog 已集中维护 161 个名称，所有绑定从同一目录发现；profile-only 条目现在公开输入形状、输出名、默认参数和约束，避免跨语言各自维护名称/参数表。
- 新增 161 个 TA-Lib profile 名称的 JSON dispatcher smoke test：逐项经过统一请求、分派和结果 envelope，确认返回结构及等长输出；这属于执行链覆盖验证，不等同于 161 项数值等价验证。
- TA-Lib 的 `APO`、`BBANDS`、`MAVP`、`STOCH`、`STOCHF`、`STOCHRSI` 已采用官方参数顺序并真实消费 `matype`；新增 Python TA-Lib 0.8.0 对照的非默认 MA type、输出暖机和末值断言。dispatcher 只接受显式 `talib_0_7_1` profile，不再接受无版本的 `talib` 别名。
- `MAVP` profile 的 batch/非 SMA 路径已统一使用 TA-Lib 的 `maxperiod - 1` 暖机规则；新增 `tests/golden/talib/profile_matype_variants.json` 作为可复现的参数变体参考，而不是只在测试代码中硬编码末值。
- 修复 DZH `MOD(...)` 函数调用与中缀 `MOD` 运算符的 grammar 冲突，国内公式集成测试重新通过。
- TA-Lib parity corpus 已生成并纳入版本控制：44 个声明指标、3 组固定 OHLCV fixture，另有 `profile_matype_variants.json` 覆盖官方 MA type 变体，参考版本固定为 Python `0.8.0`；golden 缺失现在是失败，不再静默 skip。PLUS_DM/MINUS_DM 已接入带 period 的 Wilder 平滑，PPO 已接入 TA-Lib `matype`（默认 SMA）语义；STDDEV/VAR 使用文档化的相对浮点容差。AD 的公开路径保留 TA-Lib 标量运算顺序，避免 AVX2 累计 ULP 偏差；CLI OHLCV CSV 读取也支持 fixture 的 `#` 元数据行。

## 7. 当前实际验证状态

截至 2026-09-18，本工作树已实际验证：

- `cargo +1.98.1 test --workspace --offline --quiet`：全 workspace 测试通过；其中核心库为 `2920 passed, 0 failed, 1 ignored`，DZH compatibility 为 `43 passed, 0 failed`，CLI schema 为 `3 passed, 0 failed`，其余 workspace test targets 也无失败。
- 定向验证：`finkit` operation tests `19 passed`、Composite tests `8 passed`、`finkit-ffi-common` library tests `28 passed`、C ABI library tests `23 passed`。
- 最新定向验证：`finkit-ffi-common` library tests `38 passed`，包含 161 个 TA-Lib profile 名称的 dispatcher smoke、参数目录、非默认 `matype` 数值测试、无版本 profile 拒绝测试和 Formula/Factor/Composite 共用 conformance vector；C ABI tests `25 passed`，并确认 catalog 参数通过 ABI 导出。
- `cargo +1.98.1 check -p finkit-python -p finkit-node -p finkit-go -p finkit-java -p finkit-dotnet -p finkit-ffi --offline`：通过。
- 61 个 candlestick operation 在 `talib_0_7_1` profile 下逐项真实分派并返回等长结果。
- `cargo +1.98.1 fmt --all` 已执行。
- `node --test visualization/frontend/lightweight-charts-adapter.test.mjs`：`2 passed, 0 failed`；这是 adapter contract test，不等同于真实浏览器版本兼容或完整交互集成。
- TA-Lib golden：`24 passed, 0 failed`，44 个指标均有固定 reference 文件；当前集合不是 TA-Lib 全目录证明。

本轮没有宣称完成：

- TA-Lib 全目录数值等价、性能全面超过 TA-Lib；
- TDX/同花顺/东方财富全部市场函数和 Pine 全语言兼容；
- Factor/Composite 所有路径都已达到生产吞吐 SLO；
- CMake/CTest 下的 C++ 原生编译与安装（当前验证环境没有 CMake/CTest）；
- 八语言真实宿主运行时 golden、发布包和 ABI 稳定性；workspace 中 Node binding 的 Rust 测试出现 Node-API 宿主符号加载告警，且 `ffi/node-binding npm test` 在当前环境因缺少 `finkit-win32-x64-msvc` native addon 包而未进入用例，不能替代真实 Node 宿主 smoke test；
- Lightweight Charts 完整浏览器交互集成；
- 订单、回测或风控能力（明确不在本项目范围）。

## 8. 生产验收门禁

只有同时满足以下条件，能力才可标记为 production-ready：

1. catalog 有唯一条目和版本化 profile。
2. batch、range、streaming（若声明）结果通过同一参考数据集。
3. warm-up、NaN/null、空输入、长度不一致和非法参数都有明确测试。
4. 多标的、多周期、横截面和基本面不会发生状态串扰或未来数据泄漏。
5. Factor 与 Composite 有吞吐、延迟、内存分配和 checkpoint 恢复基准。
6. Rust、Python、Go、Java、.NET、C、C++、Node 的公开结果、错误和版本字段一致。
7. C/C++、Node、Python 等原生发布产物在 CI 的实际目标平台完成构建和 smoke test。
8. 兼容系统对 unsupported 语义返回可定位诊断，不静默近似。

## 9. 仍需确认的两项发布决策

当前实现可以按默认方案继续推进：C++ 以 C ABI + CMake/RAII header wrapper 发布，TA-Lib profile 以显式版本字符串发布。若你们希望 C++ 直接拥有独立的二进制 ABI，或要把某个具体 Pine/公式平台版本锁为官方兼容基线，请在下一轮确认；否则按上述默认方案执行。
