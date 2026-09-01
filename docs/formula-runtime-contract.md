# Formula Runtime 契约

- 适用版本：Finkit 0.1.x
- 当前实现基线：main 上的 FormulaEngine、FormulaExecutor 和 Python CompiledFormula
- 目标：明确计算结果、数据所有权、warm-up、增量上下文和并发边界，避免把局部优化误解为全局 zero-copy 保证

## 1. 执行入口

| 入口 | 语义 | 所有权与性能边界 |
|---|---|---|
| FormulaEngine.eval | 对 FormulaContext 执行完整公式 | 由调用方管理上下文；复杂公式可产生中间数组 |
| FormulaEngine.eval_range | 计算半开区间 [start, end) | 自动扩展公式所需 lookback，再裁剪返回结果；结果长度为 end - start |
| FormulaEngine.eval_last | 返回最后一根结果 | 当前通过最后一个区间求值，属于正确性优先的局部计算，不等同于专用 O(1) 状态机 |
| FormulaEngine.eval_zero_copy_inputs | 从连续切片借用 OHLCV 输入 | 同步调用期间借用输入；直接 MA/EMA/RSI/BOLLMID 路径可避免输入 Array1 物化 |
| Python CompiledFormula.eval | 复用编译计划和引擎执行 | 输入复制到 owned stream context，便于后续 append_bar 和 eval_last |
| Python CompiledFormula.eval_zero_copy | NumPy 借用路径 | 必须是非空、等长、连续的一维 float64 数组；复杂公式仍可能分配中间数组 |
| Python CompiledFormula.append_bar | 向 retained context 追加一根 OHLCV | Vec push 为摊销 O(1)；追加后需调用 eval_last 才会重新计算结果 |
| Python CompiledFormula.reset | 清空 retained context | 保留 compiled plan 和 engine cache；下一次带数组的 eval 建立新 context |

## 2. 输入契约

- open、high、low、close、volume 必须存在、非空且等长。
- amount 为可选输入；如果提供，必须与 OHLCV 等长。
- eval_zero_copy 和 NumPy zero-copy 入口要求连续的 float64 一维数组；不连续视图应先调用 numpy.ascontiguousarray。
- 输入数组在 borrowed 同步求值完成前必须保持存活；运行时不得把 borrowed context 保存到下一次调用。
- append_bar 当前只追加 OHLCV；如果公式依赖 amount，应在扩展该 API 前明确缺失 amount 的 NaN/拒绝策略。

## 3. 输出契约

- Python 结果字典使用 __result__ 作为主结果键。
- 公式产生的用户变量可以作为附加结果返回；内部 CSE 临时变量以 _CSE 开头时不作为用户变量导出。
- 返回数组长度与执行模式一致：完整求值为输入长度，range 为 end - start，last 为一个标量。
- warm-up 期间沿用指标实现约定的 NaN 语义；不得在不同绑定中悄悄改成 0 或删除前置位置。

## 4. Range、Last 和 Append 的一致性

必须满足以下等价关系：

1. eval_range(source, 0, n) 与 eval(source) 的 __result__ 数值一致；
2. eval_range(source, a, b) 与 eval(source)[a:b] 数值一致，包含 NaN 位置；
3. eval 后 append_bar，再执行 eval_last，结果与把新 bar 拼接到完整历史后重新 eval 的最后值一致；
4. reserve_bars 只影响容量，不影响结果；
5. reset 后不得继续使用旧 stream context，必须显式重新 eval 或 eval_range。

## 5. Zero-copy 的准确表述

对外应使用以下分层表述：

- borrowed input：输入 OHLCV 在同步执行期间可被借用；
- direct zero-copy kernel：已支持的简单公式可以直接使用输入切片；
- pooled execution：运行时复用 scratch/buffer，减少重复分配；
- complex formula：数组型内建函数可能产生中间数组；
- bytecode variable load：当前通用执行路径仍可能把变量切片复制为 owned Array1。

因此，zero-copy 不是“所有公式、所有变量、所有输出都不分配”。任何更强承诺必须由分配计数和数值等价测试支持。

## 6. 并发和生命周期

- Python CompiledFormula 是 unsendable；不要在线程之间共享同一个实例。
- 并行场景应为每个线程/任务创建独立计划或在 Rust 层使用明确的线程安全封装。
- borrowed 输入只在一次同步调用期间有效；不得跨调用保存裸指针。
- append_bar 会把 borrowed FormulaSeries 转为 owned，以保证后续追加的生命周期安全。

## 7. 必需测试

- full 与 range 结果一致；
- eval + append_bar + eval_last 与完整重算一致；
- reserve_bars 不改变结果；
- reset 后旧 context 不可读取；
- 连续与非连续 NumPy 输入行为稳定；
- owned、borrowed direct kernel、Bytecode 和 optimized 路径在支持范围内一致；
- 复杂公式的中间分配可解释；
- 跨 Python、C、Node、CLI 的 golden fixture 保持输出命名、warm-up 和错误类别一致。

## 8. 版本策略

本契约属于 0.1.x 稳定化工作。若要改变输出键、warm-up、NaN、range 边界、append 缺失字段或线程模型，必须：

1. 更新本文件和 API 文档；
2. 增加兼容性/回归测试；
3. 更新版本矩阵和 CHANGELOG；
4. 在发布门禁通过后再合并。
