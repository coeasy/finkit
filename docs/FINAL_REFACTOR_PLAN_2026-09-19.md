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

## 4. TA-Lib 0.8.0 当前收敛状态

真实 `talib.get_functions()` 为 201 个公开函数。当前仓库已完成：

- profile catalog：138 个 profile-only 名称；
- 合并 Core registry 后 dispatcher：201 个名称；
- checked-in numeric golden：201 个；
- 审计差集：0；
- 21 个新增函数的独立 adapter、参数目录、输出字段和 warm-up：已接入；
- Core golden suite：已通过；
- `finkit-ffi-common`：已通过 70 个测试，包含全目录 dispatcher smoke。

这只证明当前 workspace 和参考环境的目录/数值对照，不等于所有操作系统、编译器、CPU、Node 宿主和发布包均已完成验证；发布前仍需运行完整 binding/ABI 矩阵。

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
- 横截面 Factor 批处理入口也已接入统一 dispatcher，并保留时间戳/标的轴与
  row-major null 语义；本轮 `finkit-ffi-common` 测试为 73 个通过。

## 6. 后续实施顺序

1. 完成当前改动的 fmt、workspace、ABI 和 binding 回归并提交；Lightweight HTML
   页面生成已纳入 visualization 回归，但真实浏览器宿主仍需单独纳入 CI。
2. 将 TA-Lib profile 参数/输出 metadata 从手写 match 逐步生成化，但保留 profile adapter 的显式语义代码。
3. 为八语言补齐同一份 201 函数 typed-buffer/JSON conformance vector，尤其覆盖新增 21 个函数的多输出和 warm-up。
4. 将 Formula dialect registry、TA-Lib catalog、Factor catalog 和 Draw schema 统一纳入版本发布清单。
5. 为生产部署增加 benchmark workload、内存/分配、并发、checkpoint 恢复和错误可观测性门禁；性能结论按硬件和数据规模分别报告。
6. 继续扩展 TDX/同花顺/东方财富通用函数与 Pine 子集，但每次扩展必须先增加语义矩阵和参考向量，再进入 production catalog。

## 7. 本文档的验证原则

本文档只引用已在仓库或参考环境中实际运行的结果。任何后续数字、版本和“已支持”表述，都必须由脚本、测试报告或绑定矩阵重新生成；手工修改文档不得替代验证。
