# Finkit 功能、架构与实现审计

> 审计日期：2026-09-26
>
> 本文回答三个问题：项目现在有什么、主要链路如何工作、哪些能力已经完成以及哪些地方仍不合理。结论基于当前源码、测试、CI、发布脚本和现有重构记录，不把“已注册”“能编译”“有测试”误认为“已实现且可生产使用”。
>
> 配套重构方案：[`docs/refactor-plan-2026-09-26.md`](refactor-plan-2026-09-26.md)

## 1. 审计结论摘要

### 1.1 总体判断

Finkit 已经形成以 Rust `core` 为单核的完整工程骨架，数值指标、公式解析、流式计算、因子分析、可视化、CLI 和多语言绑定均有实际代码，不是空壳项目。当前主要问题不是缺少某个顶层模块，而是**不同执行路径和不同绑定之间的收敛程度不一致**：

- Tree 解释执行路径覆盖最广，是当前事实上的参考实现；
- Bytecode、JIT、SIMD 是明确的替代或兼容路径；
- Plan 是面向生产吞吐的目标路径，但仍未达到与 Tree 的完整能力等价；
- `FormulaExecutionMode` 只对部分 `eval*` API 生效，部分公共入口会绕过用户选择的模式；
- 核心绑定链路能够编译并有发布产物，但不同语言的契约漂移检查深度不同；
- CLI 的 JSON 图表使用真实 visualization 管线，而 SVG/HTML 仍是独立的最小折线实现；
- Android、iOS、WASM 的能力边界是有意收窄的，但文档和统一契约必须更明确地区分“核心支持”和“平台子集”。

### 1.2 发布状态判定

| 维度 | 判定 | 说明 |
|---|---|---|
| Rust 数值核心 | **基本完成** | 指标、数学核、流式、模式识别、特征和数据契约均有实现和测试。 |
| Tree 公式路径 | **可用** | 当前默认路径，覆盖面最广，是迁移期间的参考实现。 |
| Bytecode/JIT/SIMD | **已实现但非统一生产入口** | 有独立 API；`eval_simd` 当前是兼容别名，不是独立语义后端。 |
| Plan 公式路径 | **未达到生产默认条件** | 现有记录显示默认翻转测得 12 个红 target、143 个失败测试、76 个缺失 kernel；另有字符串、宿主数据、缓存语义等结构性问题。 |
| 因子/特征分析 | **主体可用，产品边界需收紧** | 公式因子库、因子图、研究分析已连通；不应把回测/选股域外能力重新塞回核心。 |
| 流式计算 | **主体可用** | registry、meta、状态机有门禁，但应继续强化批量/流式语义一致性。 |
| Python / C/C++ / Node | **主链路可用** | 有编译、接口和发布验证；Python/Node 有较强 SSOT 检查。 |
| Java | **已修复 panic 边界后可用** | 最新审计已补齐所有 `extern "system"` 导出的 panic 防护。 |
| Go / .NET / iOS / Android | **可编译或可打包，但契约检查深度不一致** | 当前仍属于 deferred drift-check tier，不能声称与 Python/Node 同等完整。 |
| WASM | **核心导出存在，发行链不完整** | 有 `.wasm`，但没有完整的 JS/TS npm 应用包和安装消费验证。 |
| CLI 可视化 | **部分贯通** | JSON 使用真实 visualization，SVG/HTML 仍是最小 close-price polyline。 |
| Python wheel / native zip | **可重建并可冒烟验证** | manifest、archive verify、wheel smoke 均已形成闭环。 |
| OS 安装包 | **未完成** | 干净 checkout 下缺少提交的 WiX `Product.wxs`，不能列为当前发布能力。 |

**最终结论：核心数值库已经达到“可用候选发布”水平，但项目整体还不能声明“所有核心功能全部实现、所有主体链路完全贯通”。Plan 默认切换、跨绑定统一契约、CLI SVG/HTML、移动/WASM 分发仍是发布前的结构性工作。**

## 2. 功能地图

### 2.1 Workspace 层

Workspace 入口是 `Cargo.toml`，当前 14 个主要成员：

| Crate | 主要职责 | 依赖方向 |
|---|---|---|
| `finkit` | 数值核心、指标、公式、流式、特征、模式、计划执行 | 不依赖上层绑定 |
| `finkit-factor-analysis` | 因子研究、IC、收益、风险、报告、验证 | 依赖 `finkit` |
| `finkit-visualization` | 图表场景、K 线、指标叠加、SVG/HTML/Canvas/WebGL/PNG/JSON | 依赖核心数据结构 |
| `finkit-ffi-common` | 跨语言 JSON/操作/公式/因子/流式/研究契约、共享 runtime | 依赖 `finkit` 和分析层 |
| `finkit-ffi` | C/C++ ABI、指针和结果所有权 | 依赖 core/common |
| `finkit-python` | PyO3/NumPy、公式、因子、图表、研究 API | 依赖 core/common/visualization |
| `finkit-node` | N-API、公式、流式、图表、WebGL/WebGPU | 依赖 core/common/visualization |
| `finkit-go` | CGO 包装和 C ABI 消费 | 依赖 native C ABI |
| `finkit-dotnet` | P/Invoke、托管结果和 RID 打包 | 依赖 native C ABI |
| `finkit-java` | JNI 指标、公式、研究、图表 handle | 依赖 core/common/visualization |
| `finkit-ios` | staticlib/XCFramework、Swift 子集 API | 依赖 core/common |
| `finkit-android` | Android JNI 子集和 AAR | 依赖 core/common |
| `finkit-cli` | 命令行计算、公式、流式、特征、研究和图表 | 依赖 core/analysis/visualization |
| `finkit-wasm` | wasm-bindgen 指标、公式、流式、研究、图表 | 依赖 core/common/visualization |

原则上这套分层是合理的：数值能力在 `core`，跨语言契约在 `ffi-common`，绑定只做适配。但是当前各绑定仍有不少语言特有的重复入口，后续应让“共享语义契约”优先于“语言专属实现”。

### 2.2 Core 功能层

`core/src/lib.rs` 和各子模块形成以下功能分层：

```text
数据/运行时契约
  ├─ SeriesView / MarketFrame / TemporalSeries / NaN / warm-up / alignment
  ├─ HostContext / zero-copy / memory / timeout / recursion limits
  └─ error / diagnostics / compatibility contracts

数值计算
  ├─ math kernels / rolling / regression / reductions / SIMD
  ├─ batch indicators
  ├─ streaming indicators and state
  ├─ patterns / candlestick / chart patterns
  └─ features / labels / combinations / multi-period resonance

语义和组合
  ├─ formula parser / AST / optimizer / dialect mapper
  ├─ bytecode / JIT compatibility / formula SIMD
  ├─ compute DAG / execution plan / unified dispatch
  ├─ factor provider / factor graph / composite operations
  └─ drawing / templates / stateful formula
```

这个分层的优点是能力集中、没有平行 crate。主要不合理点是：

1. 语义层、参考执行层和生产计划层的边界仍然有重复入口；
2. `FormulaEngine` 同时承担解析、缓存、上下文写回、执行模式选择、计划编译和多种兼容 API，职责过重；
3. `ffi-common` 同时承担契约定义、JSON 转换和共享 runtime，后续应拆成“纯契约”和“宿主 runtime”两个逻辑层，避免绑定只想使用类型时引入过多运行时；
4. 部分平台绑定直接复制核心调用，而不是全部通过统一 operation contract，造成 coverage 和错误语义不一致。

## 3. 主体执行链路

### 3.1 批量指标链路

```text
Python / C / Node / CLI / WASM / JNI
        ↓
参数与输入校验
        ↓
core::indicators / core::math
        ↓
Array1 / caller-owned buffer / TaResult / typed wrapper
        ↓
语言侧转换、释放和异常/错误映射
```

该链路主体已贯通。风险集中在不同绑定的错误返回语义：C/Go/.NET 走结果码或指针，Python 走异常，Android 当前未知指标可能退化为空数组，必须统一为显式错误。

### 3.2 公式链路

```text
source + dialect + input + host context
        ↓
parse / normalize / custom expansion
        ↓
AST + semantic metadata + dependency/effect analysis
        ├─ Tree executor（当前参考/默认）
        ├─ Bytecode compiler + VM
        ├─ FormulaHotPlan + UnifiedExecutor
        └─ frozen JIT / SIMD compatibility APIs
        ↓
values + variables + output_names + output_modifiers + diagnostics
```

问题不在于缺少路径，而在于路径治理：

- `eval`、`eval_with_dialect`、`eval_with_params` 等会根据 mode 选择 Plan；
- `eval_ast`、`eval_lazy`、`eval_parallel`、`eval_template`、部分 validation/default/batch API 仍直接走 Tree；
- Pine multi-output 和部分 stateful/drawing 入口也绕过 mode；
- 因此调用者无法只凭 `FormulaExecutionMode` 判断实际执行后端。

### 3.3 因子和研究链路

```text
formula / indicator / factor provider
        ↓
FactorGraph / Operation / cache
        ↓
FactorAnalysis panel / IC / returns / risk / report
        ↓
Python / C / Go / .NET / Java / iOS / Android / WASM JSON contracts
```

该链路已经有统一 JSON contract，但研究层和核心因子层的“批量时间序列”“横截面”“多标的”“多周期”能力边界需要在接口文档中进一步明确。不能把已有因子库误称为完整回测或选股系统；这些能力按产品边界应继续排除。

### 3.4 图表链路

完整 visualization 链路是：

```text
KlineData / ChartConfig / IndicatorOverlay / ChanOverlay
        ↓
ChartScene / ChartRenderer
        ├─ JSON
        ├─ SVG
        ├─ HTML / Canvas
        ├─ WebGL / WebGPU
        └─ PNG
```

Python、Node、WASM 基本使用这条链路。CLI 的 JSON 使用该链路，但 SVG/HTML 在 `cli/src/main.rs::run_chart` 中自行拼接 close-price polyline，形成第二套图形实现。这是明确的前后端断链和重复实现。

## 4. 不合理设计清单

### P0：发布前必须解决

#### P0-1 Plan 尚未达到生产等价

现有实测基线：Plan 默认翻转时 12 个红 target、143 个失败测试、76 个缺失 kernel。缺口包含：

- 回测/选股域外函数：应删除，不应补回核心；
- 字符串/板块函数：需要把 Tree 的 string table 语义带入 plan；
- 宿主数据：需扩展借用型 `HostContext`；
- 大量普通指标和统计函数：机械补 kernel，但必须逐个建立绝对参考测试；
- drawing、stateful、implicit OHLC 和错误/缓存可观测语义：不能仅靠新增 kernel 解决。

#### P0-2 FormulaExecutionMode 不是全局语义开关

当前 mode 只治理一部分 `eval*`。如果用户选择 Plan，某些 API 仍会走 Tree，导致：

- 性能不可预测；
- unsupported 公式的失败时机不一致；
- timeout/memory/recursion 限制行为可能不一致；
- binding 层无法稳定解释“选了 Plan 之后到底执行了什么”。

#### P0-3 CLI SVG/HTML 是重复且不完整的可视化实现

CLI 自己生成 close-price polyline，跳过 K 线、指标、Chan、交互、LOD 和统一 renderer；标题还未经过统一 HTML/SVG escaping。应删除这套实现，改为调用 visualization 的真实 renderer。

### P1：进入统一发布线前解决

#### P1-1 跨绑定 SSOT 覆盖不均衡

核心 indicator registry 为 235 项，FFI registry 只有 78 项带 binding bodies，剩余 157 项没有 binding。这个子集可以是产品选择，但必须在接口文档和每个绑定的能力清单中显式表达，不能让用户从“core registry”推断“所有语言都支持”。

#### P1-2 deferred 绑定缺少源级漂移检查

当前 Python/Node 是 active tier，C/Go/Java/.NET/iOS/Android 是 deferred。它们可以编译和打包，但 `sync_bindings.py --check --all` 不会比较 stored bodies。建议先把 C/Java/.NET/Go 提升为“契约稳定 tier”，移动端和 WASM 单独使用 surface/behavior contract。

#### P1-3 Android 错误语义不安全

`dispatch_ta` 的未知名称、计算失败和合法空结果可能都变成空数组。必须改为 `Result<Vec<f64>, BindingError>` 或统一 JSON error contract，禁止 silent empty。

#### P1-4 一次性公式 API重复创建 FormulaEngine

共享 JSON contract 使用 thread-local reusable engine，但 Python/Node/C/.NET 的部分 direct API 每次创建新引擎，失去 cache/plan/scratch reuse。应保留兼容入口，但内部统一调用 per-thread runtime 或显式 `CompiledFormula`。

### P2：产品化阶段解决

- iOS/Android/WASM 是否提供统一 Formula/Factor/Streaming contract；
- WASM 从 raw `.wasm` 升级为 JS/TS npm 包；
- Python 可选 accessor 和异常测试不再允许“功能缺失即 skip”；
- binding README 与 workspace 版本、发布状态同步；
- Java/C/Go/.NET/移动绑定的资源所有权、free 函数和异常边界统一测试。

## 5. 核心功能是否全部实现？

答案是：**按“数值核心”口径，大部分已实现；按“完整产品和所有执行路径”口径，尚未全部实现。**

### 已实现且有证据的部分

- 数学和技术指标：有 batch、slice、streaming、SIMD/标量实现；
- Tree 公式执行：当前默认，功能覆盖最广；
- Bytecode/JIT/SIMD API：可调用，有测试或兼容契约；
- 因子图、因子库、研究分析：有 Rust 和部分语言接口；
- C ABI、Python wheel、native archive：可构建、可验证、可冒烟；
- Java JNI panic boundary：已完成全导出审计；
- 流式 registry/meta：有结构门禁；
- 文档、链接、SSOT、孤儿逻辑、dead-code suppression：已有 19 道发布门禁。

### 未完成或不能声称全部完成的部分

- Plan 不能成为默认生产路径；
- 不是所有公共 `eval*` API 都服从 `FormulaExecutionMode`；
- 452 个 formula catalogue 名称不等于 452 个终端都能运行；
- C/Go/Java/.NET/iOS/Android 的源级 binding drift check 尚未全部启用；
- iOS/Android 是平台子集，不是 core 全量 parity；
- WASM 缺少完整 npm/JS/TS 发行链；
- CLI SVG/HTML 不是完整 visualization 能力；
- OS 安装包在干净 checkout 下仍不可构建。

## 6. 审计结论

项目的合理方向不是推倒重写，而是：

1. 保留 `core` 单核和 Tree 参考实现；
2. 把 semantic IR、capability metadata、Plan lowering、UnifiedExecutor 变成唯一生产主线；
3. 让所有公共 API 明确声明自己使用的 backend，而不是让 mode 半生效；
4. 让所有语言绑定复用同一 operation/JSON/error/ownership contract；
5. 把平台子集、未实现函数和发行限制全部变成可机器检查的 capability matrix；
6. 删除重复实现（尤其 CLI SVG/HTML、Android silent empty、绑定侧重复公式 runtime）；
7. 在 Plan、绑定、可视化三条主链达到可测量的完成条件后，再提升默认路径或扩大发布声明。
