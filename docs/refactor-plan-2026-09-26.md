# Finkit 最优化重构方案

> 版本：v1.0
>
> 日期：2026-09-26
>
> 适用范围：`core`、公式执行路径、因子/研究链路、`ffi-common`、语言绑定、CLI、可视化和 WASM 发布链。
>
> 配套现状审计：[`docs/architecture-and-feature-audit-2026-09-26.md`](architecture-and-feature-audit-2026-09-26.md)
>
> 本方案采用“最小破坏、单核收敛、可逆迁移、门禁驱动”的优化路线，不进行没有验证收益的全仓推倒重写。

## 1. 已确定的优化原则

用户要求按最优化方式实现。结合当前代码和发布约束，本方案采用以下决策：

### 1.1 不推倒重写 Rust core

当前 `core` 已经包含成熟的数学核、指标、流式状态、公式 Tree 解释器、因子图和运行时契约。全部重写会同时破坏：

- 数值基线和历史 golden vectors；
- Python/C/Node 等已有 ABI；
- 公式方言兼容行为；
- streaming warm-up、NaN、ownership 和错误码语义；
- 现有测试和发布资产。

因此保留 Tree 作为**参考实现和迁移期安全基线**，只把重复的语义、计划和绑定逻辑收敛到共享中间层。

### 1.2 Plan 是最终生产主线，但不能直接翻转默认

目标架构仍然是：

```text
source + dialect + host context
        ↓
parser / normalizer / semantic metadata
        ↓
Canonical Formula IR
        ↓
capability analysis + lowering
        ├─ supported → Factor/Formula Plan → UnifiedExecutor
        └─ unsupported → explicit capability error
```

Plan 在完成 capability closure 和行为等价之前保持显式 opt-in。**禁止静默回退 Tree**，否则无法知道 Plan 的真实覆盖率，也无法保证性能承诺。

### 1.3 单一语义、多后端执行

所有 backend 都必须共享：

- 函数 canonical name；
- 参数和输入绑定；
- lookback/warm-up；
- NaN 和错误语义；
- output/variable/drawing metadata；
- timeout/memory/recursion 限制；
- string literal 和 host data 语义；
- 绝对参考向量。

Tree、Bytecode、Plan、JIT 只允许在执行机制上不同，不允许各自解释一套业务语义。

### 1.4 绑定优先复用共享 contract

语言绑定的主路径统一为：

```text
language adapter
    ↓
ffi-common contract / error / ownership / panic guard
    ↓
UnifiedOperationEngine / FormulaEngine / FactorAnalysis
    ↓
core
```

允许语言保留符合平台习惯的轻量 API，但不能再复制一套公式、错误、缓存或空结果语义。

### 1.5 明确“支持”“已验证”“已发布”三种状态

所有功能矩阵统一使用：

| 状态 | 含义 |
|---|---|
| `implemented` | 源码已有实现，并有至少一个真实调用路径。 |
| `contracted` | 有跨层接口契约和结构门禁。 |
| `tested` | 有行为测试、边界测试或 golden vectors。 |
| `validated` | 至少一个目标平台真实构建并运行 smoke test。 |
| `published` | 有可重复的发布产物、安装和消费验证。 |
| `unsupported` | 明确不属于产品范围，不能出现在“待补实现”里。 |
| `planned` | 尚未实现，但已经登记 owner、依赖和验收标准。 |

不能用“已注册”“crate 能编译”替代这些状态。

## 2. 目标架构

### 2.1 目标分层

```text
┌─────────────────────────────────────────────────────────────┐
│ Product APIs                                                │
│ Python / Node / C / Go / .NET / Java / iOS / Android / WASM │
│ CLI / Factor research / Visualization                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│ Shared Host Contract Layer                                  │
│ ffi-common: JSON, errors, ownership, panic, capability      │
│ per-thread runtime, input/output normalization               │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│ Canonical Semantic Layer                                    │
│ Formula IR / Operation / FactorGraph / Effect / Metadata    │
│ function registry / lookback / warmup / host requirements   │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│ Backend Layer                                               │
│ Tree reference | Bytecode | Plan/UnifiedExecutor | SIMD/JIT │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│ Numerical Kernel Layer                                      │
│ math / indicators / streaming / patterns / features         │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 各层职责边界

#### A. Numerical Kernel Layer

负责纯数值计算和状态更新：

- 输入长度和参数校验；
- warm-up 和 NaN 位置；
- batch 与 streaming 的数值规则；
- 标量和 SIMD 实现；
- 绝对参考测试。

不负责：公式解析、跨语言 JSON、缓存策略、平台异常转换。

#### B. Canonical Semantic Layer

新增或强化一个统一的语义描述层，至少包含：

```rust
struct FunctionSpec {
    canonical_name: String,
    aliases: Vec<String>,
    arity: Arity,
    input_requirements: InputRequirements,
    host_requirements: HostRequirements,
    lookback: LookbackSpec,
    warmup: WarmupSpec,
    effects: EffectSet,
    output_shape: OutputShape,
    capabilities: CapabilitySet,
    kernel: KernelId,
}
```

该层是以下信息的唯一来源：

- formula registry；
- plan kernel registry；
- streaming registry 的可映射部分；
- FFI capability matrix；
- 文档和兼容性报告。

禁止在 parser、Tree executor、Plan executor、Python binding、Android dispatcher 中分别复制函数名和参数规则。

#### C. Backend Layer

定义统一 backend trait 或等价的内部接口：

```text
compile(source, dialect, capabilities) -> CompiledProgram
execute(compiled, inputs, host, limits) -> ExecutionResult
```

结果必须包含：

- values/channels；
- variables；
- output names/modifiers；
- diagnostics；
- cache metrics；
- selected backend；
- unsupported capability（如有）。

`FormulaEngine` 负责协调，不再直接承载所有具体后端细节。

#### D. Shared Host Contract Layer

`ffi-common` 逐步拆为两个逻辑子层：

1. `contract`：纯类型、JSON schema、错误码、能力矩阵、版本；
2. `runtime`：thread-local engine、cache、scratch buffer、panic guard、ownership helper。

不要求一次拆成两个 crate；第一阶段先用 module boundary 和依赖规则约束，第二阶段再视编译依赖情况拆 crate。

## 3. 分阶段执行路线

### Phase 0：冻结现状和验收基线

**目标：** 在任何重构前锁住当前行为。

**任务：**

1. 将当前审计文档作为架构现状基线；
2. 生成 `docs/generated/capability-matrix.json`，记录每个函数/平台/backend 的状态；
3. 保存 Tree 输出作为绝对 reference vectors；
4. 将以下指标记录为 baseline：
   - Rust 全量测试：4714 passed / 0 failed / 17 ignored / 71 binaries；
   - 19 个 Python/结构门禁；
   - `cargo fmt --all -- --check`；
   - core lib、tests、benches 无 warning；
   - wheel/native archive smoke；
5. 对所有 deferred binding 标记真实状态，不再使用模糊的“全语言支持”。

**完成条件：** 任何后续阶段失败都能通过 reference vectors 和 baseline 分辨“行为回归”与“环境问题”。

### Phase 1：收敛语义 SSOT

**目标：** 解决 registry、formula、plan kernel、streaming meta、FFI capability 的多份事实源问题。

**任务：**

1. 清点并合并函数元数据，优先以已有 registry 为输入；
2. 为每个函数补齐：canonical name、aliases、arity、lookback、warmup、input/host requirement、effects、output shape、backend capabilities；
3. 生成以下物料：
   - core registry；
   - plan kernel table；
   - streaming mapping；
   - binding capability matrix；
   - compatibility report；
4. 继续保留无法统一的少量特例，但必须登记 `reason` 和 owner；
5. 把“注册但未被真实 corpus 调用”从“implemented”降为 `registered_only`。

**完成条件：**

- 新增函数只需在一个语义 SSOT 登记；
- 生成物 drift gate 能发现 registry/plan/streaming/binding 不一致；
- 452 个 formula catalogue 名称能够区分 `implemented`、`registered_only`、`unsupported`、`planned`。

### Phase 2：统一 FormulaEngine 入口

**目标：** 让 `FormulaExecutionMode` 变成真正的 backend 选择契约。

**任务：**

1. 设计 `BackendRequest`，包含：
   - requested mode；
   - fallback policy（默认禁止 fallback）；
   - dialect；
   - host requirements；
   - limits；
   - output policy；
2. 将以下绕过 mode 的入口改成统一 dispatch：
   - `eval_ast`；
   - `eval_lazy`；
   - `eval_parallel`；
   - validation/defaults；
   - template；
   - shared batch；
   - Pine multi-output/security；
3. 对确实只能走 Tree 的能力显式返回：
   - `BackendUnsupported { backend, capability, function }`，或
   - `TreeOnly` 的结果 metadata；
4. 禁止通过“偷偷走 Tree”伪装 Plan 已支持；
5. `eval_simd` 继续保留为兼容别名，但文档明确它不是独立语义 backend。

**完成条件：**

- 任意公共 `eval*` API 都能查询实际 selected backend；
- 选择 Plan 后 unsupported 会在 compile/lowering 阶段清晰失败；
- 选择 Tree 时不再误报 Plan metrics；
- 所有旧调用方行为有兼容测试。

### Phase 3：完成 Plan capability closure

**目标：** 让 Plan 从“部分生产候选”变成可按 capability 集合安全启用的生产路径。

#### 3.1 先删除域外功能

按产品边界清除回测/选股域外函数及其伪待办：

- `FOX_BACKTEST`、`FOX_BUY`、`FOX_SELL`、交易信号/收益/回撤类；
- `SELECTCOND`、`SMARTSELECT`、`TOPN`、`SORT`、`RANK` 等选股类。

删除顺序：registry → parser compatibility → tests → plan gap reports → docs。不能只删除 kernel 而留下“注册成功但运行失败”的入口。

#### 3.2 再解决结构性能力

1. **String literal**：让 compiled plan 携带 literal table，并在 context bind 阶段将字面量登记到 `string_table`，保持 Tree 的索引语义；
2. **HostContext**：继续使用借用型 context，扩展 block/index/em/finance/dynamic data，但禁止按值复制大序列；
3. **effects**：drawing、variable write、output emission、stateful effects 进入 IR，不再由 executor 私下猜测；
4. **cache contract**：明确 `cache_hit` / `cache_len` 是 backend-local 还是统一 API。建议统一为 `BackendCacheMetrics`，避免切换 backend 改变旧字段含义；
5. **limits**：统一 memory、timeout、recursion、loop budget 的可观测错误和粒度。

#### 3.3 补剩余普通 kernel

按依赖顺序分组：

1. pure arithmetic/statistics/reduction；
2. rolling/lookback/stateful numeric；
3. implicit OHLC/volume expansion；
4. host data；
5. string/block/domain adapter；
6. drawing and multi-output.

每补一个 kernel 必须同时完成：

- registry entry；
- lookback/warmup metadata；
- Tree-vs-Plan absolute vector；
- short input / NaN / invalid parameter tests；
- Python/Node contract smoke；
- error code mapping。

**完成条件：** 不能用“缺 kernel 数下降”作为唯一指标，必须同时满足：

- supported capability matrix 中的函数全部有 kernel；
- Plan differential 不能只比较两边 NaN/两边相同错误；
- 关键函数有绝对参考数据；
- Plan 模式下所有声明为 supported 的公共公式入口均可运行。

### Phase 4：统一绑定 contract 和错误语义

**目标：** 将跨语言差异限制在包装层，不让每个平台拥有一套业务语义。

#### 4.1 binding tier 重新分级

| Tier | 目标 | 平台 |
|---|---|---|
| Tier 1 | SSOT body drift + behavior + package smoke | Python、Node、C/C++ |
| Tier 2 | SSOT surface + error/ownership/panic + package smoke | Go、.NET、Java |
| Tier 3 | capability surface + focused behavior + package smoke | iOS、Android、WASM |

Tier 2 不应继续称为纯 deferred；它们至少要有自动化 surface 和 behavior contract。

#### 4.2 统一错误模型

所有绑定必须能区分：

- invalid input；
- unsupported function/capability；
- insufficient data；
- calculation error；
- host data missing；
- panic/internal error；
- ownership/free error。

Android `unknown -> empty Vec` 必须改掉，至少返回结构化 error code 或 JSON error。

#### 4.3 统一 runtime reuse

- 无状态单次 API 使用 thread-local shared runtime；
- 重复公式使用 `CompiledFormula` / `CompiledPlan`；
- 语言侧不得每次重新创建 FormulaEngine；
- cache metrics 进入统一诊断，不允许静默改变含义；
- Java chart/data handle、C result buffer、Go/.NET pointer、Python NumPy view 都必须有 ownership contract。

#### 4.4 绑定边界门禁

增加或扩展以下门禁：

- 所有 `extern "C"` / `extern "system"` 导出必须有 panic guard；
- void JNI 多输出必须用 `ffi_catch_void`；
- 所有 pointer-return export 必须有对应 free/ownership 文档；
- Android unknown/failed 不得返回合法空结果伪装成功；
- 每个平台生成 surface 与 capability matrix 必须一致。

### Phase 5：统一可视化链路

**目标：** 删除 CLI 的第二套图表实现。

**任务：**

1. `cli::run_chart` 只负责输入解析和 `ChartConfig` 构造；
2. 所有 SVG/HTML/JSON/PNG 输出都调用 `finkit-visualization::ChartRenderer`；
3. CLI 参数映射到：
   - OHLC/Kline；
   - indicator overlays；
   - Chan overlays；
   - title/theme/size；
   - renderer backend；
4. 统一 title/label/HTML/SVG escaping；
5. 为 CLI 增加 golden output 或结构断言：
   - SVG 必须有 candle/axis/title；
   - HTML 必须含合法 escaped title；
   - JSON 必须包含 scene/series metadata；
6. 禁止再次出现“close-price polyline quick preview”独立实现。

**完成条件：** CLI、Python、Node、WASM 对同一 `KlineData + ChartConfig` 产生同一 scene 语义；格式差异只存在于 renderer。

### Phase 6：WASM 和发行链产品化

**目标：** 让 WASM 成为可消费产品，而不只是 raw artifact。

**任务：**

1. 选择 wasm-pack 或等价构建工具生成 JS/TS glue；
2. 输出 npm 包、类型声明、版本和 ABI/contract manifest；
3. 增加 Node/browser smoke：公式、指标、streaming、chart、factor JSON；
4. 将 WASM capability matrix 与 iOS/Android 子集能力矩阵统一格式；
5. 只有真实发布链完成后，才把 WASM 状态从 `validated` 提升为 `published`。

OS 安装包单独作为发布项目：先提交并验证 WiX `Product.wxs` 和干净 checkout 构建，再允许在发布清单中标为可用。

## 4. 推荐实施顺序

```text
Phase 0 基线冻结
    ↓
Phase 1 语义 SSOT
    ↓
Phase 2 FormulaEngine 全入口统一
    ↓
Phase 3 Plan capability closure
    ├──────────────┐
    ↓              ↓
Phase 4 绑定契约   Phase 5 可视化统一
    └──────┬───────┘
           ↓
Phase 6 WASM/发行产品化
```

### 为什么不先补所有 kernel？

因为当前缺口中包含域外回测/选股函数、字符串/宿主数据和缓存/上下文结构问题。如果不先完成产品边界和语义 SSOT，直接补 kernel 会：

- 把不应该存在的功能重新实现；
- 让 Plan 产生更多孤儿 kernel；
- 使 Tree/Plan 语义进一步分叉；
- 把 binding 和文档的错误承诺扩大。

### 为什么不先把所有绑定都提升 active tier？

因为没有统一 capability/error/ownership contract 时，提升 gate 只会制造大量噪声。先统一 contract，再按 Tier 2/Tier 3 分阶段提升，投入产出更高。

## 5. 工作项分解和依赖

| 编号 | 工作项 | 优先级 | 依赖 | 产出 |
|---|---|---:|---|---|
| A0 | capability/status matrix | P0 | 无 | machine-readable capability matrix |
| A1 | Tree absolute vectors | P0 | A0 | reference fixtures |
| A2 | canonical FunctionSpec | P0 | A0 | semantic SSOT |
| B1 | FormulaEngine backend dispatcher | P0 | A2 | all `eval*` unified |
| B2 | backend selection diagnostics | P0 | B1 | selected backend/capability error |
| C1 | delete out-of-scope formula names | P0 | A2 | smaller supported domain |
| C2 | string literal/HostContext/effects | P0 | B1 | structural Plan support |
| C3 | pure/rolling/host kernels | P0 | C1,C2 | Plan capability closure |
| C4 | Plan absolute/differential gate | P0 | C3 | safe promotion gate |
| D1 | shared error and ownership model | P1 | A2 | cross-language contract |
| D2 | promote Go/.NET/Java source checks | P1 | D1 | Tier 2 drift gates |
| D3 | Android structured error | P1 | D1 | no silent empty result |
| E1 | CLI renderer unification | P1 | visualization API | one chart path |
| E2 | CLI chart golden tests | P1 | E1 | output regression gate |
| F1 | WASM JS/TS package | P2 | D1 | npm artifact |
| F2 | OS installer clean-checkout build | P2 | release packaging | installer artifact |

## 6. 验收门禁

### 6.1 每个 PR 必须通过

- `cargo fmt --all -- --check`；
- `cargo check --workspace --all-targets --locked`，必要时按 `-j 2` 分阶段重跑；
- 当前 19 个 Python/结构门禁；
- 不得新增无理由 `allow(dead_code)`；
- 不得新增孤儿脚本、死 workflow、缺失脚本引用；
- 不得引入未登记的 formula/plan kernel/FFI function。

### 6.2 Plan 推进门禁

每次增加 Plan 能力必须有：

1. Tree absolute fixture；
2. Tree vs Bytecode vs Plan differential；
3. 至少一个有限值断言，防两边都 NaN 假绿；
4. short input / all NaN / invalid parameter；
5. limits（memory/timeout/recursion/loop）；
6. variables/output_names/output_modifiers；
7. host data/string/drawing/stateful（适用时）；
8. binding smoke。

### 6.3 默认切换条件

只有全部满足才允许把默认从 Tree 改为 Plan：

- 支持域外已清除并有不支持矩阵；
- 计划支持矩阵中所有 `supported` 函数均有 kernel；
- 全部公共 `eval*` 入口遵守 mode；
- Plan differential 和 absolute fixtures 全绿；
- 关键绑定以 Plan/contract 运行并通过 smoke；
- cache/error/limits/context side effects 契约稳定；
- 经过一次可逆 default-flip 全量测试，结果为 0 failed；
- 失败不允许通过 silent Tree fallback 隐藏。

## 7. 风险与控制

| 风险 | 控制措施 |
|---|---|
| Plan 补 kernel 导致 Tree/Plan 数值分叉 | 先写 absolute reference，再写 kernel；禁止只看 differential。 |
| 删除域外公式破坏旧用户 | 先在 compatibility report 标记 unsupported，提供明确错误和迁移说明；不静默改变为别的函数。 |
| FormulaEngine 重构破坏 ABI | 保留旧公开方法签名，内部转发到新 dispatcher；分阶段删除内部重复实现。 |
| 绑定错误语义变化 | 先在 ffi-common 定义版本化 error contract，再更新各语言适配。 |
| Java/Android/JNI panic 或资源泄漏 | 全导出 panic guard 扫描、handle free 测试、长时间循环和异常注入。 |
| CLI 可视化输出变化 | 先保存现有输出 fixture，再迁移到统一 renderer，提供兼容格式选项。 |
| WASM 工具链或网络不可用 | 将 raw `.wasm` 作为中间产物，发布 npm 包作为独立阶段，不把环境失败伪装成代码完成。 |
| 文档和代码再次漂移 | 所有可验证声明进入 capability matrix、生成文档或 CI gate。 |

## 8. 建议的第一批实施任务

优先实施以下 5 项，收益最高且依赖最少：

1. **生成 capability matrix**：先把“实现/契约/测试/验证/发布/不支持”状态机器化；
2. **统一 FormulaEngine backend dispatcher**：先治理绕过 mode 的公共入口，不先大规模补 kernel；
3. **完成 Plan 缺口分类清理**：删除域外函数，单独登记 string/host/drawing/stateful，不把所有缺口混成一个数字；
4. **修复 CLI SVG/HTML**：直接复用 `ChartRenderer`，删除重复 polyline 和未 escaping 的 title；
5. **统一 binding error contract**：优先修 Android silent empty，再提升 Go/.NET/Java 的 source-level contract gate。

这 5 项完成后，项目才具备继续扩大 Plan 覆盖和升级绑定发布层级的稳定基础。

## 9. 最终目标

最终状态不是“所有路径都复制一遍实现”，而是：

```text
一个语义 SSOT
一个数值 kernel 层
一个可解释的 reference backend
一个生产 Plan backend
一个共享跨语言 contract
多个薄适配层
一套可验证 capability matrix
```

达到这一状态后：

- 新增函数只需一次语义登记和一次 kernel 实现；
- Tree、Plan、streaming、binding、docs 的差异能够自动检测；
- 不支持能力会明确失败，不会静默回退或返回空结果；
- 性能优化不会改变业务语义；
- 发布条件可以由门禁证明，而不是依赖人工宣称。
