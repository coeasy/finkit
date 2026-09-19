# Finkit 最终架构收敛与生产化重构方案

> 基线：`feature/finkit-v1-unified-engine-20260917`
> 目标范围：公式系统、TA-Lib profile、因子计算器、Composite、跨语言运行时和 Lightweight Charts 数据适配。
> 明确排除：订单、回测、交易执行和风控。

## 1. 最终产品边界

Finkit 应定位为“统一数值语义的公式与因子计算引擎”，而不是交易系统。核心公开能力分为：

1. TA-Lib 0.8.0 Python profile，对应 wheel 内置 core 0.8.1；公开函数名、输入顺序、默认参数、输出名、warm-up、NaN 和错误语义固定。
2. 经典公式系统：TDX、同花顺、东方财富兼容语义，以及受控 Pine 子集；每个方言拥有独立 profile，不把同名函数强行合并。
3. Factor 与 Composite：支持 batch、range、incremental、streaming、checkpoint 和 bounded cache；Factor 与 Composite 均按生产吞吐设计。
4. 数据维度：单标的/单周期是基础；多标的、多周期、panel、横截面和 publication-time 基本面通过显式输入 contract 接入，不隐式重采样、不泄漏未来数据。
5. 绘图：Core 只产生版本化 `draw` 数据；Web 端优先由 Lightweight Charts adapter 渲染，计算层不依赖浏览器。
6. 语言：Rust、Python、Go、Java、.NET、C、C++、Node 共用同一 Rust canonical core、C ABI 和 JSON/typed-buffer contract。

## 2. 当前架构审查结论

### 2.1 合理且应保留

- Core 负责数值、Formula AST/IR、Factor/Composite plan 和状态；binding 负责转换、生命周期和错误映射。
- Operation catalog、dispatcher、cache identity 和 contract vectors 已形成统一入口，适合继续收敛。
- batch 与 streaming 已区分；有限 lookback、递归状态和未知函数不再被错误标记为 bounded streaming。
- TA-Lib profile 已与国内公式同名语义分离，新增 `talib_ext` 用于 ADR、KDJ、QSTICK、RVI 等冲突函数。

### 2.2 原有不合理点及最终处理

| 问题 | 风险 | 最终方案 |
|---|---|---|
| runtime 直接创建具体 Factor | crate 耦合、插件困难 | `FactorCatalog -> CompiledFactorPlan -> FactorProvider/Kernel` |
| Core registry 与 TA-Lib profile 共用同名语义 | 输入和 warm-up 被静默混用 | `semantic_profile` 显式分派；生产 profile 为 `talib_0_8_0` |
| 只登记函数名、不固定输出和 warm-up | 多语言结果不一致 | catalog 参数、输出 schema、golden 和 contract vector 四者绑定 |
| 公式 parser、运行时、绘图互相耦合 | 无法做跨语言和流式状态 | AST -> typed HIR -> execution IR / Draw IR 分层 |
| cache key 缺少 scope、revision、profile | 串标的、串周期、旧结果污染 | key 至少包含 profile、operation/formula id、参数 hash、scope、revision、range |
| FFI 各语言各自维护指标名单 | 漂移、版本难以升级 | Rust catalog 生成 discovery 和 binding metadata，binding 不复制数值逻辑 |
| “注册/可解析”被当成“可执行/等价” | 生产误用 | 分开记录 parse、register、execute、reference、cross-language 五种状态 |
| TA-Lib 复合函数从 standalone kernel 直接拼接 | 起始索引、seed 和浮点顺序可能不同 | profile adapter 按官方调用范围实现；KC 已按 TA-Lib 起始窗口重新 seed ATR |

## 3. 目标分层架构

```text
Public APIs / bindings
        |
        v
Versioned Contract Layer
  operation / formula / factor / composite / draw / error
        |
        v
Canonical Rust Core
  parser -> AST -> typed HIR -> execution plan -> kernel/state
        |
        +--> TA-Lib profile adapters
        +--> TDX/THS/EastMoney/Pine lowering
        +--> Factor/Composite compiled plans
        +--> bounded cache + checkpoint
        |
        v
C ABI / typed buffers / JSON envelope
        |
Python | Go | Java | .NET | C | C++ | Node
        |
Lightweight Charts adapter
```

### 3.1 数值执行链

`request -> normalize -> catalog lookup -> profile validation -> typed inputs -> compiled plan -> kernel/state -> named outputs -> cache -> envelope`。

每个结果必须携带 schema version、semantic profile、operation identity、output names、shape、warm-up/null policy 和 execution trace（如发生 range/streaming）。

### 3.2 Formula 执行链

`source + dialect -> parser -> canonical AST -> dialect lowering -> typed HIR -> lookback/control-flow analysis -> batch/stateful plan -> values + draw`。

TDX、同花顺、东方财富的 `REF/HHV/LLV/SUM/STD/CROSS` 等语义必须在 profile 中固定；Pine 只开放已验证子集，`request.security` 必须有显式 provider 和时间对齐策略。

### 3.3 Factor / Composite

- Factor：单因子依赖图、输入槽位、方向/类型、lookback、batch 与 stateful kernel。
- Composite：表达式图、依赖拓扑、cycle 检查、有限窗口证明、嵌套 stateful 节点。
- 两者均支持 `scope=symbol/timeframe/panel` 与 `data_revision`，不把跨截面计算伪装成时间序列计算。
- checkpoint 必须可序列化、带 plan signature 和 semantic profile，恢复时校验版本与输入槽位。
- streaming checkpoint 还必须带 `scope` 与 `data_revision`；Factor、Composite、Formula
  三类入口在恢复前统一拒绝 scope 或数据版本不匹配，避免跨标的、跨周期或跨数据快照串状态。

## 4. TA-Lib 0.8.0 当前收敛状态

真实 `talib.get_functions()` 为 201 个公开函数。当前仓库已完成：

- profile catalog：141 个 profile-only 名称；
- 合并 Core registry 后 dispatcher：201 个名称；
- TA-Lib profile 的可执行性由 Core TA-Lib contract 与 profile-only catalog
  联合推导，扩展项 `DONCHIAN`、`SUPERTREND`、`VWAP` 也纳入同一 canonical
  catalog；测试强制校验所有 catalog 名称都能到达 dispatcher，避免手写支持列表再次漂移；
- registry operation 与 profile-only operation 共用 TA-Lib 输出 schema table/helper；
  `DONCHIAN`、`KDJ`、`SUPERTREND` 等多输出契约通过 profile 层校验，避免把 Core
  顶层输出语义误当成 TA-Lib 兼容输出；schema table 还具备唯一性和非空输出门禁；
- TA-Lib profile 参数 metadata 也已改为声明式 parameter schema table，统一维护
  参数名、类型、默认值和约束，并通过唯一性、可执行性和完整约束门禁；
- profile-only 输入槽位（series/hlc/hlcv/ohlcv/dynamic）已统一为 input schema table，
  candlestick 动态前缀和默认 series 规则均有测试覆盖；
- checked-in numeric golden：201 个；
- shared numeric contract：201 个向量、160 行合成输入，直接从 checked-in TA-Lib
  0.8.0 golden 生成；Node、Python 已实际执行，Go、Java、.NET 已接入相同逐元素
  执行入口，C/C++ 已接入由同一生成器产生的 201 向量测试 fixture；各平台仍以 CI
  宿主结果作为最终发布证据；
- 审计差集：0；
- 21 个新增函数的独立 adapter、参数目录、输出字段和 warm-up：已接入；
- Core golden suite：已通过；
- `finkit-ffi-common`：已通过 84 个测试，包含全目录 dispatcher smoke、canonical
  catalog 覆盖门禁和三类
  streaming checkpoint provenance 校验。

这只证明当前 workspace 和已执行 binding 的目录/数值对照，不等于所有操作系统、编译器、CPU、Node 宿主和发布包均已完成验证；发布前仍需运行完整 binding/ABI 矩阵。

`tests/contracts/talib_numeric_contract_v1.json` 不作为手工维护的第二份
truth source。`scripts/gen_talib_numeric_contract.py --check` 会重新读取
`tests/golden/talib/*.json`、覆盖矩阵和合成输入，并拒绝生成结果漂移；数值比较同时
检查 warm-up 的 `null`、输出长度和逐元素绝对/相对误差。

## 5. 生产化门禁

### 必须通过

- Rust workspace 全量测试、fmt、clippy（按 CI 配置）；
- TA-Lib 201/201 golden、dispatcher 和 catalog 一致性；
- Formula 方言边界、绘图、控制流、Pine host-required 和 temporal 对齐测试；
- Factor/Composite batch-range-stream-checkpoint conformance；
- C ABI 以及 Python/Go/Java/.NET/C++/Node 的相同 JSON/typed-buffer vectors；
- NaN/null、空输入、长度不一致、参数越界、重复变量、错误码和版本拒绝测试；
- Lightweight Charts adapter 的替换、增量、markers、pane、viewport 和 warm-up 空白测试。

### 禁止宣称

- 未有参考结果的函数“数值等价”；
- 只通过 parser 的公式“可运行”；
- 只通过单机 benchmark 的“全面超过 TA-Lib”；
- 只通过某一 binding 的“八语言一致”；
- 没有真实 Node 宿主加载或平台矩阵的“Node 全平台生产化”。

### 本轮实际落地

- `finkit-visualization::ChartRenderer::render_html` 已接入版本化
  `LightweightChartsPayload` 和仓库内的 Lightweight Charts adapter，不再返回
  “HTML rendering is not yet implemented”。
- SMA/MA、EMA、BOLL、MACD、RSI、KDJ 和 SAR 的可见指标描述会在 payload 层调用
  Core 的同一套计算函数；非有限 warm-up 值统一输出为 `null`。
- HTML 页面使用固定的 Lightweight Charts 5.0.0 CDN 入口，adapter 仍支持当前和
  旧版 `addSeries` 形态；这验证了页面契约生成，不等于已经完成所有浏览器、网络
  策略和发布平台矩阵验证。
- 已验证：`finkit-visualization` 默认特性 406 个测试、`html` 特性 426 个测试，
  集成测试分别 26/27 个通过；前端 adapter Node 测试 2 个通过。
- Factor/Composite 的跨语言批处理 JSON contract 已补充显式 `scope` 与
  `data_revision`，完整执行路径现在通过 `UnifiedOperationEngine`；range/stream
  仍明确走各自的增量/checkpoint 专用执行器，不把不同能力误报为同一模式。
- Factor、Composite、Formula 的 stream contract 现在统一写入 checkpoint 的
  `scope`/`data_revision`，恢复前同时拒绝 scope mismatch 和 data revision mismatch；
  本轮 `finkit-ffi-common` 84 个测试全部通过。
- 横截面 Factor 批处理入口也已接入统一 dispatcher，并保留时间戳/标的轴与
  row-major null 语义；本轮 `finkit-ffi-common` 测试为 84 个通过。
- 基础 Formula JSON 入口也已改走 `UnifiedOperationEngine::Formula`，因此普通
  公式、Factor、Composite 和横截面 Factor 的公开批处理入口共享同一 Runtime
  dispatcher；Temporal/Panel/Streaming 仍保留其显式时间对齐与状态执行路径。
- Temporal Formula 的非 Pine 请求现在也复用统一 Formula dispatcher；Pine 请求
  保留显式 security resolver，以维持 provider 缺失、时间对齐和 host-required
  错误语义，不将 provider 访问伪装成普通变量。
- Pine 子集本轮新增 `ta.wma`、`ta.hma`、`ta.stdev`、`ta.variance`、
  `ta.correlation`、`ta.barssince` 和 `ta.tr` 的 catalog mapping；其中 `ta.tr`
  显式展开为 OHLC 输入后再 lower 到 Core `TRANGE`，其余函数通过统一 Formula
  runtime 执行。AST lowering 与 `FormulaEngine` 数值执行回归均已覆盖；未经过
  参考向量和语义矩阵验证的 Pine 函数不宣称为生产支持。
- FFI 的 Factor、Composite、Formula 和 direct operation 入口现在复用线程本地的
  `UnifiedOperationEngine`，每个线程保留有界 compiled-plan/result cache，避免跨语言
  JSON 调用每次重新创建 Runtime。Factor/Composite 批处理与三类 streaming 请求现在
  必须显式提供非空 `scope` 和 `data_revision`；`data_revision=0` 是合法的初始快照，
  缺少元数据或使用空 scope 直接拒绝，避免无来源数据进入共享缓存或 checkpoint。
- Operation catalog 现在对同名但 profile 语义不同的 operation 发布
  `profile_output_contracts`；顶层字段保留 Core registry 语义，TA-Lib profile
  使用独立输出 schema，并通过 201 项 dispatcher smoke 逐项校验实际返回字段，
  防止 discovery 与执行结果在 KDJ、AROON、DONCHIAN 及扩展多输出指标上漂移。
- 多语言目录门禁现在直接读取同一份
  `tests/contracts/talib_coverage_matrix_v1.json`：Node、Python、Go 和 .NET
  对照 201 个名称集合；C、C++ 和 Java 校验当前 profile 的 201 个 catalog
  contract entries；全部入口都拒绝已淘汰的 `talib_0_7_1`。这防止旧 native
  artifact 被误当作最新源码验证；完整目录当前还包含 Core-only operation，
  因此不把总 operation 数误写成 201。
- 201 个 TA-Lib 数值向量现在进入 Node 的默认 `npm test`、Python 发布 wheel gate
  和 C/C++ CTest；Go、Java、.NET 测试也读取同一份 JSON contract。C/C++ 的测试
  头文件由 `scripts/gen_talib_numeric_contract.py` 生成并由 `--check` 校验，避免
  为 native 测试维护第二份数值真源。
- 对照官方 CandleSettings 语义修正了 10 个此前只用固定比例近似的 candlestick
  adapter（Harami、Morning/Evening Star、Piercing、Three Stars in the South、
  Homing Pigeon、Matching Low、Mat Hold、Tasuki Gap、Unique Three River）；完整
  201 向量审计从 10 个差异收敛为 0 个差异。
- 内置 operation catalog 现在由进程级 `OnceLock` 缓存，并作为 TA-Lib dispatcher
  的运行时执行契约：请求中的参数数量不得超出 profile schema，返回字段、输出数量、
  shape 和序列长度必须与 schema 对齐；不一致统一返回结构化
  `internal_contract_error`，避免仅依赖测试发现生产环境的 catalog/执行器漂移。
- 主 CI 已加入 `finkit-ffi-common` 统一契约测试，多语言工作流的触发路径已覆盖
  共享 FFI 契约、C/Go/Java binding 和 `tests/contracts` fixtures。
- `tests/contracts/engine_contract_v1.json` 现在同时包含 direct Operation、Formula、
  Factor 和 Composite 的共享请求/结果向量，并新增多个原始序列、跨定义依赖、
  常量广播和多输出的 `composite_multi_input` 向量。该向量已由 Rust
  `finkit-ffi-common`、Node、Python 和 C ABI 实际执行并通过；Go、Java、.NET
  已接入同一 fixture 的测试入口，但本次工作机缺少 Go native 链接产物/Maven/.NET
  SDK，因此未将它们标记为本机运行通过。C++ 复用同一 C ABI，不维护第二套数值实现。

## 6. 后续实施顺序

1. 已完成当前 Runtime typed execution chain 的 fmt、workspace check 和专属回归并提交；
   Lightweight HTML 页面生成已纳入 visualization 回归，但真实浏览器宿主仍需单独纳入 CI。
2. TA-Lib profile 参数、输出和 profile-only 输入 metadata 已收敛为声明式 schema
   tables；继续保留 profile adapter 的显式数值语义代码，并将表项纳入发布清单。
3. 将本轮已接入的 `engine_contract_v1.json` 与 `talib_numeric_contract_v1.json`
   扩展到 C/C++、Go、Java、.NET 的实际宿主运行证明；C/C++ 已完成测试接入，Go、
   Java、.NET 仍需在其 CI/toolchain 矩阵中完成逐元素运行证据。
4. 将 Formula dialect registry、TA-Lib catalog、Factor catalog 和 Draw schema 统一纳入版本发布清单；继续把多输入 Composite 接入同一 typed plan，而不是重新引入平行 Factory。
5. 为生产部署增加 benchmark workload、内存/分配、并发、checkpoint 恢复和错误可观测性门禁；性能结论按硬件和数据规模分别报告。
6. 继续扩展 TDX/同花顺/东方财富通用函数与 Pine 子集，但每次扩展必须先增加语义矩阵和参考向量，再进入 production catalog。

## 7. 本文档的验证原则

本文档只引用已在仓库或参考环境中实际运行的结果。任何后续数字、版本和“已支持”表述，都必须由脚本、测试报告或绑定矩阵重新生成；手工修改文档不得替代验证。

### 本轮共享向量验证记录

- Rust：`cargo +1.98.1 test -p finkit-ffi-common --offline`，80 passed，1 ignored doc。
- Node：共享 engine contract 测试，13 passed。
- Node：重建当前 Rust native module 后 `npm test`，13 passed（含 TA-Lib 目录集合门禁）。
- Node：重建当前 Rust native module 后 `npm test`，14 passed（含 201/201 数值向量）。
- Python：从 workspace 根目录运行
  `python -m pytest ffi/python-binding/tests/test_engine_contract_v1.py -q`，2 passed。
- Python：重新构建并安装当前 wheel 后
  `python -m pytest ffi/python-binding/tests/test_talib_numeric_contract.py -q`，1 passed。
- 生成校验：`python scripts/gen_talib_numeric_contract.py --check` 通过；Node 独立
  审计报告为 `201 vectors / 0 failures`。
- Java：重新构建当前 `finkit_java.dll`，编译并运行
  `com.finkit.ContractConformance`，exit code 0（含 TA-Lib 目录门禁）。
- Go：未运行，工作机 `CGO_ENABLED=0` 且没有 `gcc`；C/C++：未运行，工作机没有 CMake/C++ 编译器；.NET：未运行，工作机没有 `dotnet`。这些是未验证项，不视为通过。

### 多输入 Composite 契约收敛记录（2026-09-19）

- `composite_multi_input` 验证了同一请求中 `high`/`low` 多原始序列、`sum`/`mid`/`spread`/`signal` 跨定义依赖、`const:2` 广播、三路输出以及 `multi_series`/无 primary 结果语义。
- 完整执行仍通过 `UnifiedOperationEngine -> OperationRequest::Composite -> CompositeEngine`；本轮没有把旧 `crates/finkit-runtime` 单依赖 Executor 扩展成第二套生产 Runtime。
- `CompositeEngine` 现在公开只读 `CompositeCacheStats`，记录结果快照缓存的 hits、misses、entries、capacity；注册自定义函数、变更容量和清理缓存都会清零旧统计，避免把失效前的命中率带入新缓存周期。
- `UnifiedOperationEngine::composite_cache_stats()` 将该统计暴露在统一 Runtime façade，避免调用方只看到 Factor/Formula 通用缓存而看不到 Composite 的独立计划/结果缓存。
- dirty-range 现在也进入统一 façade：`execute_factor_range_targets()` 和
  `execute_composite_range()` 负责计划解析、依赖执行和 `RuntimeExecutionTrace`；
  FFI Factor/Composite range 分支不再直接绕过 Runtime 调用领域引擎，原有
  `input_dirty/affected/recompute` contract 向量保持不变。
- Factor 多目标 full batch 现在通过 `execute_factor_targets()` 一次执行共享 DAG，
  并按目标集合建立 batch result cache；重复依赖只计算一次，重复请求直接命中
  batch 缓存；canonical target 集合按稳定顺序归一化，反序请求也复用同一缓存，
  不再由 FFI 按 target 循环调用单目标入口。
- Composite bounded/stateful stream 现在通过
  `UnifiedOperationEngine::prepare_composite_stream()` 编译计划，并从已注册的
  Runtime 克隆 stream engine；FFI 不再以 `CompositeEngine::new()` 重建仅含内置函数的
  第二条初始化路径，注册的自定义函数和 stateful spec 不会在流式入口丢失。
- Factor bounded stream 现在通过
  `UnifiedOperationEngine::prepare_factor_stream()` 解析目标、复用编译计划缓存并克隆
  已注册的 `FactorEngine`；FFI 不再重建独立的 builtin registry，Factor streaming 与
  full/range/batch execution 使用同一注册和 canonical target 语义。
- Factor 普通、横截面和 catalog metadata 读取也改为使用 Runtime-owned
  `FactorCatalog`；FFI 不再为计划/描述信息单独构造 builtin catalog，避免执行与公开
  catalog 在注册表、别名和元数据上的漂移。
- Formula stateful stream 现在通过
  `UnifiedOperationEngine::prepare_formula_stream()` 完成 dialect admission 和 stream
  编译；三类 stream contract（Formula/Factor/Composite）均由同一 `shared_runtime`
  facade 创建，checkpoint 仍由各自的 Core state object 持有。
- `engine_contract_v1.json` 新增 `factor_multi_target`，覆盖 `momentum_5` 与依赖它的
  `reversal_5`；Rust FFI common、Python、Node 和 C ABI 当前源码已实际执行该向量，
  Go/Java/.NET 测试入口也已接入同一 fixture。
- 本轮实际验证：`finkit-ffi-common` 80 tests passed、C ABI 31 tests passed、Python contract 2 passed、Node 13 passed。Go 测试在未完成 native 产物链接时无法编译，Java 因 Maven 不可用，.NET 因 SDK 不可用；这些均保持未验证状态。

### 当前基线与门禁状态（2026-09-19）

- 当前基线分支：`feature/finkit-v1-unified-engine-20260917`。
- 历史性能基线提交为 `6ad4aa8`，文档基线随后由 `f7eb988` 推送；固定窗口内核改动提交为 `f69bca8`，随后 `MIDPOINT14` 线性块扫描提交为 `cc123a6`；当前稳定性能提交为 `146b39a`，最新 SIMD 路径提交为 `881f7d3`，Runtime typed chain 提交为 `18bff08`，均已推送到远端同名分支。
- Rust 格式检查、`finkit` library 测试（当前提交 2947 passed、1 ignored）、`finkit-ffi-common` 测试（80 passed、1 ignored doc）以及 Python ABI3 release check 已实际通过。
- 历史基线的 TA-Lib 0.8.0 对照门禁覆盖 96 个指标和 24 个公式，`parity_failures=[]`、`errors=[]`；三档规模指标几何平均约 `1.61x`，但性能门禁未通过（top-20 最低约 `0.64x`，`MIDPRICE14`、`VAR20`、`WILLR14` 持续低于 `0.95x`）。该数字仅用于保留基线，不代表本轮结果；因此不能宣称“全面超过 TA-Lib”。
- 已实际通过的 binding 验证包括 Node 全部 14 项默认测试，以及 Python 当前 wheel 的 TA-Lib 数值合同；Go 因本机 `CGO_ENABLED=0` 且缺少 `gcc` 未运行，Java 因缺少 Maven 未运行，C/C++ 因缺少 CMake/编译器未运行，.NET 因缺少 `dotnet` 未运行。这些语言仍需由对应 CI 宿主提供真实运行证据。
- GitHub Actions 页面目前没有显示该最新提交的可核验运行结果；在出现对应 workflow run 前，不能把 GitHub CI 说成已通过。工作流文件已配置 `feature/finkit-v1-unified-engine-*` 分支触发规则。

本节是实施状态记录，不是完成声明。下一阶段仍以多语言实际宿主验证、公式方言覆盖、Lightweight Charts 浏览器宿主验证、剩余性能瓶颈和生产发布门禁为主线。

### VAR20/RSI 稳定优化后的公开 wheel 复核（`146b39a`，2026-09-19）

- `VAR20` 新增 O(1) rolling mean/M2 稳定快速路径；固定间隔重播种，并对高绝对基线输入回退到精确 TA-Lib 兼容状态机。测试覆盖普通数据的容差等价和高基线数据的 bitwise 精确回退。
- `RSI` 平均值转换改为等价的单除法形式，减少每个输出点的除法次数；其公开兼容结果仍通过 TA-Lib 数值容差验证。
- 基于当前提交重建并安装 Windows ABI3 wheel，在 Python 3.12.13、NumPy 2.3.3、TA-Lib Python 0.8.0 / core 0.8.1 环境实测：96 个指标、24 个公式全部 `parity=True`，`errors=[]`、`parity_failures=[]`。
- 指标几何平均加速比为 `1.7286x`，100K 为 `1.8387x`，1M 为 `1.5738x`；三档规模没有指标持续低于 `0.95x`。
- 性能 release gate 仍未通过，top-20 最低为 `0.9967x`，低于门槛 `1.05x`。因此当前只能确认数值合同、主体执行链和本轮稳定优化通过验证，不能宣称整体性能已经全面超过 TA-Lib。
- 本轮稳定修改已提交并推送：`146b39a perf: stabilize fixed variance and rsi kernels`。下一步应继续针对 top-20 最慢项做受控优化，每轮都必须保留全量 parity、跨规模基准和 Rust/FFI 回归证据。

### AVX-512 公开路径补齐复核（`881f7d3`，2026-09-19）

- 复核发现公开 wheel 在当前机器实际走 AVX-512 RSI 路径；此前单除法优化只覆盖 AVX2/scalar，因此补齐 AVX-512 RSI，并同步补齐 WASM SIMD 语义。AVX-512 RSI 与通用 SIMD 回归均通过。
- MOM10 AVX-512 固定周期循环增加 4×8 展开；结果保持与既有输出一致，但公开边界 benchmark 的收益受 CPU 频率和测量波动影响，不把单次改善视为稳定门禁通过。
- 基于 `881f7d3` 重建的 Windows ABI3 wheel 复测：96 个指标、24 个公式全部 `parity=True`，`errors=[]`、`parity_failures=[]`；指标几何平均 `1.7433x`，100K `1.8398x`，1M `1.5878x`，持续低于 `0.95x` 的指标为空。
- 性能 release gate 仍未通过：top20 最低 `1.0362x`，门槛为 `1.05x`。本轮完整 Rust/FFI 回归仍为 `finkit 2947 passed / 1 ignored`、`finkit-ffi-common 80 passed / 1 ignored doc`。不得将该结果表述为“全面超过 TA-Lib”。

### 当前提交宿主重建复核（`7582c8f`，2026-09-19）

- Node binding 使用 Rust `1.98.1` 从当前源码重建 Windows native module，随后运行 `npm test`：14 passed，包含共享多目标 Factor 向量和 201 项 TA-Lib numeric contract。
- Python 使用当前源码构建并安装 `finkit-0.1.15-cp38-abi3-win_amd64.whl`：
  `test_engine_contract_v1.py` 为 2 passed，`test_talib_numeric_contract.py` 为 1 passed。
- 本机 Go/Java/.NET 仍分别受 CGO/native 链接产物、Maven、.NET SDK 限制；这些语言的共享 fixture 接入已提交，但没有将未运行的宿主测试记为通过。

### Pine security 统一 Runtime 复核（2026-09-19）

- Pine `request.security` 的 provider 对齐和数据防泄漏策略仍由宿主 resolver 负责；这是跨时间周期数据契约，不下沉为隐式重采样。
- 解析、Pine-to-AlphaTA 映射、求值、多输出封装和绘图结果现在统一由
  `OperationRequest::FormulaWithPineSecurity` ->
  `UnifiedOperationEngine::execute()` 完成；底层 resolver 方法只作为 Core 内部实现。
- `ffi-common` 不再为 temporal Pine security 路径临时创建独立 `FormulaEngine`；因此普通 Formula、带 provider 的 Pine Formula 和其他高层操作共享同一 Core Runtime façade。
- Core 回归已覆盖自定义 security resolver 的 `MultiSeries`/primary 结果语义；FFI temporal contract 继续覆盖实际 provider 对齐、缺失 provider 和非法输入场景。

### Runtime typed execution chain 复核（`18bff08`，2026-09-19）

- `crates/finkit-runtime` 已从描述字符串骨架收敛为可执行链：`FactorProvider -> FactorRegistry -> Scheduler -> Executor -> FactorCache`。Factory/Provider 返回 `Box<dyn Factor>`，不再返回 `EMA(period=20)` 这类不可执行字符串。
- Scheduler 现在验证重复节点、缺失依赖和环，并输出确定性的拓扑顺序；Executor 支持无依赖输入和单依赖串接，多个依赖会返回结构化错误，不会静默使用错误输入。
- Cache 增加 `get_or_compute`、命中/未命中统计和稳定参数身份；同一 DAG 二次执行已由测试证明复用两个节点的缓存结果。
- `FactorResult` 与 Runtime 输出类型已合并为“主序列 + 命名多输出”；MACD 现在真实产出 `macd`、`signal`、`histogram`，并按时间戳对齐快慢 EMA，修复旧实现的 warm-up 错位和未使用 `signal` 参数问题。
- `QuantSeries` 增加值/时间戳对齐和严格递增校验；RSI 改为 Wilder 递推语义。专属回归为：`finkit-factor` 3 passed、`finkit-runtime` 5 passed、`finkit-series` 1 passed；workspace check 通过。
- 该批次尚未宣称 Formula/Composite 已全部迁移：当前 Runtime Executor 对单节点/单依赖链已可执行，多输入 Composite、统一 Formula typed plan、各语言实际宿主运行证明和 Lightweight Charts 浏览器 CI 仍是后续工作。

### 固定窗口内核复核（前一轮，2026-09-19）

- `WILLR14` 与 `MIDPRICE14` 的固定周期路径改为无分配的 Van Herk/Gil-Werman 前缀/后缀块扫描；`MIDPOINT14` 使用固定窗口展开访问；前一轮的 `VAR20` 仍使用 TA-Lib 的移位累计与周期重播种语义，并改为 caller-owned raw-pointer 输出循环，随后已由 `146b39a` 的稳定 rolling mean/M2 路径替代普通输入热路径。
- 补充了长度为 `14/15/27/28/29` 以及长序列的跨块回归测试，验证块边界不会改变 warm-up、窗口覆盖或结果。
- 重新运行的 TA-Lib 0.8.0 / core 0.8.1 对照：96 个指标、24 个公式，`parity_failures=[]`、`errors=[]`；指标几何平均加速比约 `1.69x`，100K 约 `1.78x`，1M 约 `1.56x`。
- 完整架构门禁仍失败：top-20 最小加速比约 `0.775x`，`VAR20` 在三档规模均低于 `0.95x`。这是真实测量结果，不调整阈值、不视为生产性能门禁通过；后续需单独优化 VAR20 和 top-20 中的慢项。

### MIDPOINT14 后续公开边界复核（2026-09-19）

- `cc123a6` 将 `MIDPOINT14` 改为固定周期前缀/后缀块扫描；最新 ABI3 wheel 已重建、安装并从 Python 公开入口实测，96 个指标和 24 个公式均 `parity=True`，`errors=[]`、`parity_failures=[]`。
- 该轮指标几何平均加速比约 `1.69x`，100K 约 `1.77x`，1M 约 `1.55x`；top-20 最低约 `0.936x`，仍未达到 `1.05x` 门禁，且 `VAR20` 仍持续低于 `0.95x`。RSI14 专用内核实验在公开 wheel 上出现 1M 回归，已撤回，没有进入提交。
- 因此当前结论仍是：数值合同与主体执行链通过已验证范围，但性能 release gate 尚未通过，不能宣称整体全面超过 TA-Lib。
