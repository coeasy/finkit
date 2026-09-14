# Finkit 全面升级完成度矩阵

本文把“TA-Lib + 公式/公示系统 + 组合指标 + 缠论 + 高性能图表 + 交易日历”
拆成可验证的交付项，避免用单个示例或单个绑定的通过结果代表全部能力。

| 能力 | 当前交付 | 主要证据 | 状态 |
| --- | --- | --- | --- |
| TA-Lib/常用指标 | Rust 核心指标、流式指标、Node/Python/WASM 绑定；Python float64 生成绑定直接返回 NumPy 数组，避免 Rust Vec → Python list → NumPy 二次物化 | `core/src/indicators`、`ffi/python-binding/src/generated.rs`、各绑定测试 | 已实现 |
| 绑定生成一致性 | 核心注册表与 FFI 绑定元数据分离；Python 绑定可发现/生成/漂移检查，C 头文件签名可复核 | `docs/indicator_registry.json`、`docs/ffi_registry.json`、`scripts/sync_bindings.py` | 已实现 |
| 公式/公示系统 | 解析、校验、多输出、绘图命令、bytecode/JIT/SIMD/zero-copy 路径 | `core/src/formula`、`formulaEval*`、公式测试 | 已实现 |
| 自定义组合指标 | 依赖图、命名中间结果、常量、算子、阈值/区间/裁剪、滚动统计、内置指标、环检测；原始序列零拷贝借用；Python `compute_composite`、Node `computeComposite`、WASM `computeComposite` | `core/src/composite.rs`、Python/Node/WASM 绑定 | 已实现 |
| 缠论核心 | 去包含、分型、笔、线段、中枢、趋势、背驰候选、B1/B2/B3、S1/S2/S3、未确认结构 | `core/src/chan.rs` | 已实现，真实市场 fixture 仍需持续校准 |
| 缠论变种/阈值 | strict/loose、5/6/7 笔、动态/递归中枢、信号强度和突破阈值 | `ChanConfig`、Python/Node/WASM/Chart API | 已实现 |
| 多周期缠论 | 自动周期、显式因子、时间戳、session 起点、交易日历过滤、源区间映射 | `core/src/chan_mtf.rs`、日历测试 | 已实现 |
| 图表交互 | TDX 风格浮窗、十字线、滚轮缩放、拖拽、键盘、事件/结构命中 | Canvas/WebGL HTML、浏览器运行检查 | 已实现 |
| GPU 大数据 | WebGPU Compute、WebGL2 instancing、LOD、Float32 Base64、增量更新、容量预留、hover backing store 复用、增量边界维护 | `visualization/src/render/webgl.rs`、100k 基准、GPU 控制器行为测试 | 已实现；WebGPU 真机待矩阵验收 |
| 实时长流 | `appendBar`、`updateBar`、`reserve`、固定容量 `setRingBuffer`、覆盖最旧物理槽位 | `scripts/test_webgl_runtime.mjs` 普通/WebGPU 模拟行为测试、GPU HTML controller 协议 | 已实现；建议补真实 WebGPU 长流压测 |
| WebGPU 健壮性 | `device.lost` → WebGL2 → Canvas 2D | WebGPU 初始化、Compute/Render 模拟及设备丢失恢复测试 | 已实现；设备丢失需真实 GPU 注入验证 |
| 交易所日历 | A 股、国内期货、港股、美股、加密货币 preset；时区、DST、午休、夜盘、半日市、CSV/JSON 覆盖 | `core/src/calendar.rs`、Rust/Python/Node/WASM API | 已实现；年度官方文件由宿主配置注入 |
| 数据源边界 | 不包含行情连接、账户、订单、撮合和交易执行 | GPU/日历/图表模块设计文档 | 已固定 |

## 回归门槛

- 核心库：2712 个测试通过，1 个忽略。
- 可视化 HTML：419 个测试通过；无 HTML 特性：399 个测试通过。
- WASM：4 个测试通过。
- Node：7 个测试通过。
- Python：72 个测试通过。
- Python wheel 的 `sma`/`macd` 返回 NumPy `ndarray`；1,000,000 根 SMA 隔离 sanity 约 1.98 ms（非跨机器 TA-Lib 结论）。
- 100,000 根 K 线 GPU HTML 负载生成、1,000,000 根降采样基准通过。
- GPU 控制器普通模式和 WebGPU 模拟模式均通过，包含指标/缠论浮窗与 device lost 恢复。
- GPU controller 的普通模式与 WebGPU 模拟模式通过；真实浏览器/硬件 adapter 仍按下方外部验收项执行。

## 明确的外部验收项

当前开发环境没有可运行的 `navigator.gpu` 设备，因此以下项目不能仅凭本地
WebGL2 结果宣称完成：

1. Chrome/Edge 不同 WebGPU adapter 的 Compute Shader 真机执行；
2. WebGPU device lost 的硬件注入与恢复时延；
3. 50 万、100 万、500 万根数据在真实 GPU 上的显存、帧率和长帧分布；
4. A 股、期货、港股年度官方日历文件的逐年结构级对照。

这些属于部署环境和外部年度数据验收，不改变 Finkit 不负责数据源和交易执行的
核心边界。
