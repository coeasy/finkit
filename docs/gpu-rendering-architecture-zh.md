# Finkit GPU 图表渲染架构

## 范围边界

Finkit 可以明确定位为“计算与可视化内核”，不设计行情数据源、交易所连接、账户、订单、撮合或资金系统。上层应用负责把任意供应商的行情标准化为 OHLCV、Unix 时间戳、指标输入和事件；Finkit 负责：

- TA-Lib 兼容指标与流式计算；
- 公示系统/公式语言解析、编译、缓存和 SIMD/JIT 执行；
- 缠论结构、多周期、阈值和候选买卖点；
- SVG、Canvas、WebGL2、WebGPU 共享的语义场景；
- 回放、视口、LOD、浮窗和可复现日历配置。

这样可以让数据源由应用自由替换，同时保证图表、指标和回测使用同一份计算契约。

## 当前 GPU 快速路径

`KlineChart::to_webgl_html_string()` 默认提供 WebGL2/Canvas HTML；`to_webgpu_html_string()` 作为显式 opt-in 提供 WebGPU/ WebGL2/Canvas HTML。Rust/Python/Node/WASM 入口保持一致：

1. 默认使用 WebGL2，复用 packed OHLCV buffer 和 instanced draw call；
2. 显式选择 WebGPU 时使用 WGSL storage buffer 和 instanced draw；WebGPU 不可用时回退 WebGL2；
3. K 线实体、影线和成交量避免生成逐根 DOM/SVG 节点；
4. 背景、指标、缠论、事件和文字保留透明 Canvas overlay，复用已有语义场景；
5. 浮动数据窗直接使用同一批时间戳和源索引，并展示涨跌、振幅、可见指标值、包络桶及事件/缠论命中信息；
6. 浏览器内支持滚轮缩放、拖拽平移、十字线和视口同步；
7. WebGPU/WebGL2 均不可用时自动降级到 Canvas 2D。

GPU 快速路径只改变绘制后端，不改变指标结果、缠论结构、命中区域或数据协议。

## WebGPU 设计与后续演进

当前 WebGPU 快速路径复用同一份 `GpuBar` packed buffer、panel、LOD 和 tooltip schema，包含：

- storage/uniform buffer 分离；
- WGSL 实例化蜡烛、影线、成交量和指标线；
- 每根 GPU bar 使用 `OHLCV + source_start + source_end` 七个 `f32` 通道，包络级别仍能映射回源索引；
- 自包含 HTML 使用小端 Float32 Base64 载荷并在浏览器端一次解码，避免百万级数据被 JSON 浮点文本放大；
- 浏览器端按可见跨度和像素宽度选择幂次桶的 high/low/open/close/volume envelope；WebGPU 通过 storage buffer 更新，WebGL2 通过 `bufferSubData` 更新；
- 浏览器 LOD 始终在当前渲染窗口的显示坐标内进行；已聚合 Rust 行也可继续合并，但不会混用原始坐标，因此缠论/指标 Overlay 与包络仍保持同一坐标系；
- WebGPU → WebGL2 → Canvas 2D 自动回退链路；

当前已落地 Compute Shader 聚合 Min/Max、屏幕像素桶和可见窗口输出；实时长流还
支持固定容量环形 buffer，覆盖最旧物理槽位并通过 `head/capacity` 传给 Compute
Shader。WebGPU 设备丢失时会自动切到 WebGL2，WebGL2 不可用时继续切到 Canvas 2D。
Compute 仅在 `dynamicLod && activeBucket > 1` 时执行，单根显示或 WebGL2 降级仍走
等价的 packed 上传路径。

WebGPU 不应复制公式引擎或指标实现；指标仍在 Rust/SIMD/WASM 中计算，GPU 只负责大量几何的可见化。

## 大数据量策略

- 视口优先：只上传可见区间及 overscan；
- LOD：像素桶内保存 high/low/open/close、成交量聚合和源范围，避免缩小时逐点绘制；
- 增量更新：导出页暴露 `window.__finkitGpuChart.updateBar(index, [open, high, low, close, volume])`，WebGL2 使用 `bufferSubData`，WebGPU 使用 `queue.writeBuffer`；
- 实时追加：`reserve(extra)` 以幂次容量预留 packed buffer，`appendBar(date, [open, high, low, close, volume], timestamp?)` 追加单根数据；扩容时重建 GPU storage buffer，未扩容时只写入新增七通道，避免每次行情更新重新生成整页 HTML；
- 长流模式：`setRingBuffer(maxBars)` 将 GPU 原始缓冲限制在固定容量，追加到上限后覆盖最旧数据；`getState()` 返回 `ringEnabled/ringHead/ringCapacity` 以及 `gpuLost/gpuRecovery`，便于宿主同步窗口、监控内存并记录设备回退；
- 热路径优化：hover 重绘会复用 Canvas backing store；普通追加/更新只增量维护价格和成交量边界，只有替换或淘汰极值时才触发全量扫描，避免长流行情下每个 tick 都是 O(n)；
- 共享缓存：指标/缠论结果按数据 revision、配置签名和视口签名缓存；
- 语义分层：GPU 绘制重复几何，Canvas/SVG 绘制文字和命中对象；
- 成交量只由 GPU volume pass 绘制，不重复进入指标 overlay；隐藏 volume 图层时同步关闭 GPU pass；
- 内存上限：以可见 bars、packed stride、GPU buffer bytes 和上传耗时作为 telemetry；
- 禁止数据源耦合：GPU renderer 不创建网络连接，也不负责行情重连。

## 验收指标

至少覆盖 10 万、50 万和 100 万根 K 线：

- 首屏上传和首帧时间；
- 缩放/平移帧率与长帧比例；
- 实时单根更新耗时；
- GPU/Canvas/SVG 输出的价格、指标、缠论和浮窗一致性；
- WebGL2、WebGPU、无 GPU 环境的自动回退；
- 公式计算耗时与渲染耗时分开统计。

## 公开接口约定

```text
Rust:    KlineChart::to_webgpu_html_string() / to_webgl_html_string()
Python:  KlineChart.to_webgpu_html() / to_webgl_html()
Node:    KlineChartNapi.toWebgpuHtml() / toWebglHtml()
WASM:    WasmKlineChart.toWebgpuHtml() / toWebglHtml()
```

导出的 HTML 还提供 `window.__finkitGpuChart` 控制器：

```js
__finkitGpuChart.setViewport(start, end)
__finkitGpuChart.updateBar(index, [open, high, low, close, volume])
__finkitGpuChart.reserve(extra)
__finkitGpuChart.setRingBuffer(200000)
__finkitGpuChart.appendBar('2026-01-03', [12, 13, 11, 12.5, 140], 1767398400)
__finkitGpuChart.getState()
```

该控制器只更新内存中的图表缓冲和视口，不负责行情订阅；宿主应用仍负责把数据源事件转换为标准 OHLCV 更新。

仓库还提供 `scripts/test_webgl_runtime.mjs`：它用无外部依赖的 WebGL2/Canvas
最小模拟面执行生成 HTML，验证控制器的环形缓冲、追加、更新、容量预留、视口协议以及
TDX 风格浮窗的 OHLCV、指标和语义命中内容；`--webgpu` 模式还会执行 WebGPU 初始化、LOD
Compute/Render 提交及 `device.lost → WebGL2` 恢复状态机。CI 会同时检查大数据示例和包含
MA/缠论事件的交互示例。
该测试用于发现 JavaScript 状态机回归；真实 GPU 绘制质量和 WebGPU Compute 仍需在
浏览器硬件矩阵中验收。

行情适配、数据清洗和订单系统由宿主应用实现；Finkit 只接收已经标准化的数据和配置。
