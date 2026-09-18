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

- profile catalog：138 个 profile-only 名称；
- 合并 Core registry 后 dispatcher：201 个名称；
- checked-in numeric golden：201 个；
- shared numeric contract：201 个向量、160 行合成输入，直接从 checked-in TA-Lib
  0.8.0 golden 生成，并由 Node 与 Python binding 实际执行；
- 审计差集：0；
- 21 个新增函数的独立 adapter、参数目录、输出字段和 warm-up：已接入；
- Core golden suite：已通过；
- `finkit-ffi-common`：已通过 80 个测试，包含全目录 dispatcher smoke 和三类
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
  本轮 `finkit-ffi-common` 80 个测试全部通过。
- 横截面 Factor 批处理入口也已接入统一 dispatcher，并保留时间戳/标的轴与
  row-major null 语义；本轮 `finkit-ffi-common` 测试为 80 个通过。
- 基础 Formula JSON 入口也已改走 `UnifiedOperationEngine::Formula`，因此普通
  公式、Factor、Composite 和横截面 Factor 的公开批处理入口共享同一 Runtime
  dispatcher；Temporal/Panel/Streaming 仍保留其显式时间对齐与状态执行路径。
- Temporal Formula 的非 Pine 请求现在也复用统一 Formula dispatcher；Pine 请求
  保留显式 security resolver，以维持 provider 缺失、时间对齐和 host-required
  错误语义，不将 provider 访问伪装成普通变量。
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
- 201 个 TA-Lib 数值向量现在进入 Node 的默认 `npm test`，并进入 Python 发布
  wheel gate；两者都使用同一份 JSON 请求、输入、参数、输出名、null 和容差，
  不再只验证“能发现 201 个名字”。
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
  Factor 和 Composite 的共享请求/结果向量。该向量已由 Rust
  `finkit-ffi-common`、Node、Python 和 Java JNI host 实际执行并通过；Go、C/C++、
  .NET 已接入同一 fixture 的测试入口，但本次工作机没有 C 编译器/CMake、dotnet
  或 CGO 工具链，因此未将它们标记为本机运行通过。

## 6. 后续实施顺序

1. 完成当前改动的 fmt、workspace、ABI 和 binding 回归并提交；Lightweight HTML
   页面生成已纳入 visualization 回归，但真实浏览器宿主仍需单独纳入 CI。
2. 将剩余 TA-Lib profile 参数/输出 metadata 从手写 match 逐步生成化，但保留 profile adapter 的显式语义代码。
3. 将本轮已接入的 `engine_contract_v1.json` 与 `talib_numeric_contract_v1.json`
   扩展到 C/C++、Go、Java、.NET 的实际宿主运行证明；当前 Node 与 Python 已完成
   201/201 数值执行，其他语言仍需在其 CI/toolchain 矩阵中完成逐元素数值门禁。
4. 将 Formula dialect registry、TA-Lib catalog、Factor catalog 和 Draw schema 统一纳入版本发布清单。
5. 为生产部署增加 benchmark workload、内存/分配、并发、checkpoint 恢复和错误可观测性门禁；性能结论按硬件和数据规模分别报告。
6. 继续扩展 TDX/同花顺/东方财富通用函数与 Pine 子集，但每次扩展必须先增加语义矩阵和参考向量，再进入 production catalog。

## 7. 本文档的验证原则

本文档只引用已在仓库或参考环境中实际运行的结果。任何后续数字、版本和“已支持”表述，都必须由脚本、测试报告或绑定矩阵重新生成；手工修改文档不得替代验证。

### 本轮共享向量验证记录

- Rust：`cargo +1.98.1 test -p finkit-ffi-common contract_conformance --offline`，10 passed。
- Node：`npm test`，12 passed。
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
