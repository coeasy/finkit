# PR #28 Architecture v3 重构与发布收敛方案

> 状态：执行中（PR #28）  
> 基线提交：`c2ec7b0b3e1a2fa344239217c81dfb3f0e02f712`  
> 分支：`perf/outperform-talib-v3-20260904`  
> 目标：在不降低 TA-Lib 语义兼容和 Architecture v3 性能门槛的前提下，将 Finkit 收敛到“单一编译链、单一热执行链、多前端适配器”的实现。

## 1. 项目定位

Finkit 是面向金融指标、公式和因子计算的高性能基础库，不是交易执行系统。核心职责包括：

1. **技术指标**：TA-Lib 兼容指标、国内市场扩展指标、K 线/图形模式、价格/成交量/统计/波动率等指标族。
2. **公式系统**：解析国内外交易终端风格公式，执行变量、函数、控制流、绘图副作用以及增量计算。
3. **因子系统**：因子注册、依赖解析、拓扑执行、复合打分、时间序列和截面处理。
4. **Runtime / Streaming**：批量、`eval_range`、`eval_last`、逐 bar 更新和复用状态。
5. **多语言绑定**：Rust 核心，Python 为当前主要高性能绑定，同时维护 Node/Java/C/C++/Go/.NET/WASM/mobile 等集成。
6. **质量与发布门禁**：语义 parity、golden/differential tests、性能门槛、内存契约、ABI/包安装验证。

## 2. 当前架构梳理

### 2.1 语义与编译层

当前已经具备 Architecture v3 的核心骨架：

- `core/src/compute.rs`
  - `ComputeNode` / `ComputePlan` 负责语义 DAG、依赖、effect、lookback、streaming capability。
  - 编译时完成拓扑排序、环检测和可观测副作用分析。
- `core/src/execution_plan.rs`
  - `HotExecutionPlan` 将语义 operation 在编译阶段转换成 `KernelId`。
  - 将逻辑节点映射为 `InputSlot` / `BufferSlot` / `StateSlot` / `ParameterRange`。
  - 计算 dependency lifetime，支持 scratch buffer 复用。
- `core/src/buffer_arena.rs`
  - `BufferArena` 与 `StateArena` 分离，分别负责临时向量和跨调用持久状态。

### 2.2 热执行层

- `core/src/unified_executor.rs`
  - `UnifiedExecutor` 持有 hot plan、dispatcher、buffer/state arenas。
  - 热路径仅接触数值 Kernel/slot，不再按字符串分发 operation。
  - 已提供 batch、range、last 等入口。

这已经满足“逻辑 DAG 与热执行分离”和“字符串只在编译期解析”的第一阶段要求。

### 2.3 指标层

指标目前同时存在三类实现：

1. `Array1` 返回型公共 API；
2. `*_into` caller-owned 输出 kernel；
3. 部分历史 SIMD/临时 scratch 实现。

PR #28 已开始把 AD/ADOSC/OBV/MFI/TRANGE 等公共 API 路由到 canonical kernel，但仍需继续执行“每个指标只有一个高性能算法实现，包装层只负责分配/绑定”的原则。

### 2.4 Formula

Formula 当前是迁移中的“双轨结构”：

- 编译阶段已经生成 `FormulaComputePlan` 并做 effect/lookback 分析；
- 常见简单公式存在 zero-copy / direct-kernel fast path；
- 通用公式仍主要由 `FormulaExecutor` 递归解释 AST；
- bytecode/JIT/cache 仍与新的 `HotExecutionPlan` 并存。

因此 Formula 已经统一了部分“计划语义”，但尚未完全统一“执行后端”。控制流、绘图、副作用必须保留解释器 fallback，纯计算子图则应逐步下沉到统一执行器。

### 2.5 Factor

Factor 已具备：

- owned / borrowed context；
- 依赖环检测；
- 动态递归计算；
- precompiled execution order；
- 请求内共享依赖缓存。

但 `FactorEngine` 的热路径仍以字符串 key、`BTreeMap<String, Vec<f64>>` 和 closure 为主。下一阶段需要把可静态解析的 built-in factor 子图 lower 到 `ComputePlan -> HotExecutionPlan`，自定义 closure 保持受控 fallback。

### 2.6 Python FFI

Python 已支持 borrowed NumPy 输入和 direct ndarray 输出，但仍存在两个架构债：

- canonical 源码中仍有部分 `PyResult<Vec<f64>>` 包装；
- release workflow 会运行迁移/生成脚本，把源码临时改写成 NumPy-direct 版本。

目标应是生成器直接产出最终 canonical hot binding，CI 脚本只做 `--check`，禁止“构建时修改工作树后再打包”。

## 3. 三轮审计结果

### 第一轮：核心链路与架构一致性

已确认：

- `ComputePlan -> HotExecutionPlan -> UnifiedExecutor` 主链存在且无字符串热分发；
- buffer/state arena 已分离；
- 编译期已具备依赖拓扑和 scratch 生命周期信息；
- Formula / Factor 仍存在未完全迁移的旧执行路径，这是当前最大的架构分叉；
- Python 构建期源码改写属于发布链路债务，应迁回 SSOT generator。

### 第二轮：正确性与死循环/孤儿逻辑

已确认：

- `ComputePlan`、Factor 依赖均具备环检测；
- Formula `FOR` / `WHILE` 具有最大迭代保护，避免无界循环；
- PR 当前 core CI 失败不是算法 parity 回退，而是旧 MACD golden fixture 的 warmup 契约过期；
- TA-Lib installed-wheel gate 的 parity failures 为 **0**；
- `UnifiedExecutor::execute_range` 的低层 range 是“已准备输入窗口内的物理区间”，前端必须先根据 lookback 扩展 dependency window；不能直接把全局逻辑区间切片后交给 rolling kernel。该契约必须保持清晰，否则会产生隐蔽增量计算错误。

### 第三轮：性能与发布门禁

基线 gate：

- indicator geomean Finkit speedup：约 `1.059x`，目标 `>= 1.15x`；
- 100K：约 `1.105x`，目标 `>= 1.15x`；
- 1M：约 `1.019x`，目标 `>= 1.20x`；
- Top20 minimum：约 `0.414x`，目标 `>= 1.05x`；
- parity failures：`0`。

持续低于 floor 的指标集中在 AD/ADOSC/BBANDS/MFI/MIDPOINT/MIDPRICE/DI/OBV/SAR/TRANGE/WILLR/WMA 等。说明当前问题主要是热路径常数开销、FFI 输出路径和共享中间状态，而不是公式正确性。

## 4. 目标架构

```text
Rust / Python / Node / WASM / C ABI
                 |
        Frontend adapters
                 |
     Semantic compile / registry
                 |
             ComputePlan
                 |
      CSE / DCE / effect barrier
      lookback / liveness analysis
                 |
          HotExecutionPlan
   KernelId + Input/Buffer/State slots
                 |
          UnifiedExecutor
        /        |         \
  BufferArena  StateArena  KernelDispatcher
                 |
    canonical *_into kernels only
                 |
       batch / range / last / stream
```

必须坚持以下约束：

1. **One kernel**：同一算法只保留一个 canonical `*_into` 核心。
2. **One plan**：batch/formula/factor/streaming 尽量共享同一种 execution plan。
3. **No string hot dispatch**：字符串只允许在 parser/compiler/registry 层出现。
4. **No hidden FFI materialization**：NumPy contiguous f64 输入直接借用；输出直接写 ndarray。
5. **Effect barrier**：assignment/output/draw/control-flow 等不可随意跨越优化。
6. **Parity first**：任何性能修改必须保持 TA-Lib warmup/NaN/seed/output length 契约。
7. **Hard performance gate**：不能通过降低阈值来“修复”性能失败。

## 5. 重构执行方案

### R0 - CI 与语义基线收敛（P0）

- [x] 定位 rustfmt 唯一失败：`core/src/math/mfi.rs` 测试数据格式。
- [x] 定位 MACD golden 冲突：旧 fixture 在 index 25-32 暴露 MACD line，而正式 TA-Lib 对齐 API 从 index 33 统一暴露三路结果。
- [ ] 修复 fixture 并确保 core test 回绿。
- [ ] 保持 TA-Lib parity failure = 0。

### R1 - 共享 extrema kernel（P0）

- [ ] 将 `rolling_minmax_visit` 从双 `VecDeque` 改为 TA-Lib 风格的“极值索引缓存 + 失效时重扫”。
- [ ] 不再为 MIDPOINT/MIDPRICE/WILLR 共用路径维护两个动态 deque。
- [ ] 增加 ties、单调序列和一般窗口的 naive differential unit test。
- [ ] 复测 Top20 minimum 与 MIDPOINT/MIDPRICE/WILLR。

### R2 - canonical indicator kernel 完成（P0/P1）

优先顺序：

1. SAR；
2. MIDPOINT/MIDPRICE/WILLR；
3. WMA/BBANDS；
4. PLUS_DI/MINUS_DI/MFI；
5. AD/ADOSC/OBV/TRANGE。

每个指标完成条件：

- 公共 `Array1` API 只负责一次输出分配；
- Python fast path 直接写 NumPy；
- Formula/batch 调同一 kernel；
- 禁止同算法保留第二套 SIMD/scratch 实现；
- differential/parity/golden 全绿。

### R3 - Formula 统一执行（P1）

- 将纯表达式/纯函数子图 lower 到 `ComputePlan`；
- 编译期把 function name + literal params 解析为 `KernelId + ParameterArena`；
- 简单公式不再每次执行重复解析字符串函数名；
- `FormulaExecutor` 仅保留 assignment、draw、control-flow、dynamic call 等 effectful fallback；
- `eval_range/eval_last` 使用统一 lookback-window planner；
- bytecode/JIT 不能再成为第三套语义实现，必须作为同一 plan 的可选 backend。

### R4 - Factor / Streaming 统一执行（P1）

- built-in factors lower 到 numeric slots；
- raw input name 在 compile 时绑定 `InputSlot`；
- 中间 factor 结果进入 arena，避免 `BTreeMap<String, Vec<f64>>` 热查询；
- streaming 复用 `StateArena`，batch 与 append/eval_last 的 seed/warmup 保持一致；
- 自定义 factor closure 作为明确的 external kernel/fallback，不污染 canonical built-in 路径。

### R5 - FFI / 生成器收口（P0/P1）

- `indicator_registry.json` + generator 成为 binding SSOT；
- canonical generated Rust binding 直接返回/写入 `PyArray1`；
- 删除 release 阶段“先 patch 源码再编译”的依赖；
- migration script 转为 idempotent checker 或最终删除；
- installed-wheel benchmark 必须测真实发布产物，不允许只测 Rust core。

## 6. 本轮 PR #28 立即落地项

本轮先执行低风险、可验证、直接影响发布条件的变更：

1. 修复 `mfi.rs` rustfmt；
2. 更新 MACD golden fixture，使 warmup 与 TA-Lib 对齐；
3. 重构共享 rolling extrema kernel，移除 `VecDeque` 热路径；
4. 增加 extrema differential test；
5. 保留现有性能阈值不变，重新由 installed-wheel gate 给出真实结果；
6. PR #28 作为 Architecture v3 重构主 PR 标记并持续收敛，不另建平行执行引擎。

## 7. 验收标准

### 正确性

- `cargo test -p finkit --locked` 全绿；
- TA-Lib parity failures = 0；
- warmup/NaN/seed/output length 与兼容契约一致；
- batch / formula / streaming 对同一 kernel 的结果一致。

### 架构

- 热执行层无字符串 operation dispatch；
- canonical 指标无重复算法实现；
- Formula 纯计算路径逐步进入 HotExecutionPlan；
- Factor built-in 路径逐步进入 numeric slot；
- Python 发布构建不再依赖修改 canonical 源码。

### 性能

最终发布门槛继续采用 Architecture v3 既定标准：

- 总体 geomean `>= 1.15x`；
- 100K `>= 1.15x`；
- 1M `>= 1.20x`；
- Top20 每项 `>= 1.05x`；
- 不允许指标长期 `< 0.95x`；
- parity failure 必须为 0。

## 8. 风险控制

- 不通过降低 benchmark threshold、删除失败 case 或放宽数值容差来换取绿色 CI；
- 不改变已公开 API 签名；
- 不在一个提交中同时改变算法语义与 FFI ownership；
- 每个共享 kernel 优化必须保留 naive/reference differential test；
- effectful Formula 在证明可安全 lower 前继续走受控 fallback；
- 遇到性能回退时优先回退单个 kernel，不回退整个 Architecture v3 主链。

## 9. 后续完成判定

PR #28 只有同时满足以下条件才可视为“达到发布条件”：

1. CI 全绿；
2. installed-wheel parity = 0 failures；
3. Architecture v3 性能硬门槛全绿；
4. 无新的重复 kernel / 构建期源码 patch 债务；
5. Formula/Factor/Streaming 的未迁移项在本文件中明确关闭或有对应实现提交；
6. 文档、代码、测试和 release gate 对同一语义事实保持一致。
