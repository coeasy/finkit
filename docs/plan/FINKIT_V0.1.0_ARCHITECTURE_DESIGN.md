# Finkit v0.1.0 Architecture Design

## 1. 目标

Finkit v0.1.0 是首个可安装基础版本，基于现有 Rust 高性能计算内核演进，不进行 C++ 重写。

核心边界：指标、因子、公式和金融时间序列计算；不负责行情源、账户、交易和完整回测平台。

## 2. Workspace 架构

```text
finkit/
├── core/                 # 统一 Rust 计算核心
├── visualization/        # 可视化输出能力
├── cli/                  # finkit CLI
├── wasm/                 # WebAssembly
├── ffi/
│   ├── ffi-common/       # FFI 公共契约
│   ├── c-binding/        # C ABI
│   ├── python-binding/   # PyO3 / maturin
│   ├── node-binding/     # NAPI-RS
│   ├── go-binding/       # Go FFI
│   ├── java-binding/     # JNI
│   ├── dotnet-binding/   # .NET native interop
│   ├── ios-binding/      # iOS
│   └── android-binding/  # Android JNI
├── scripts/
└── docs/
```

## 3. 单一计算核心

所有语言 SDK 必须调用同一 Rust Core：

```text
Python / Node / Java / .NET / Go / C / WASM / Mobile
                         |
                         v
                 FFI / Binding Layer
                         |
                         v
                    Rust Core
                         |
        +----------------+----------------+
        |                |                |
   Indicators         Formula          Streaming
        |                |                |
        +----------------+----------------+
                         |
                         v
                    Result Model
```

禁止在 Python、JavaScript、Java 等绑定层重新实现指标算法，否则会产生数值和性能漂移。

## 4. 计算模式

v0.1.0 保留并强化三类运行方式：

- Batch Vector：历史序列批量计算
- Streaming：逐 Bar O(1) 增量更新
- Formula Runtime：公式表达式解析并调用统一指标函数

## 5. 性能架构

- Rust release `opt-level=3`
- LTO
- SIMD 热点路径
- 连续数组 / ndarray
- Python NumPy buffer 直接读取
- FFI 边界减少复制
- 多股票/多序列场景预留 Rayon 并行
- benchmark profile 独立于普通 release

## 6. Python v0.1.0 公共架构

```text
pip package: finkit==0.1.0

finkit/
├── __init__.py
├── py.typed
└── alpha_ta.<native-extension>
```

其中 `alpha_ta` 是 v0.1.x 内部兼容模块名称。用户只依赖：

```python
import finkit
```

PyO3 使用 Python ABI3，从 Python 3.8 起提供跨小版本兼容能力。

## 7. CLI

Cargo package 暂保留 `alpha-ta-cli` 内部名称，公开 binary 为：

```text
finkit
```

这样可以在不破坏内部 workspace 引用的前提下完成公共品牌迁移。

## 8. 版本与兼容策略

v0.1.x 原则：

- 公共品牌统一 Finkit
- 所有版本统一 0.1.0
- 内部历史 crate / namespace 允许暂时保留 AlphaTA 兼容名称
- 不在首发版本做无价值的大规模 rename
- 任何公开 namespace 迁移必须有兼容窗口

## 9. 公式兼容架构

终端公式兼容采用 adapter，而不是为每个终端复制引擎：

```text
TongDaXin / THS / Eastmoney / Pine / Finkit DSL
                  |
                  v
            Dialect Adapter
                  |
                  v
          Normalized Formula IR
                  |
                  v
       Optimizer / Runtime / Core
```

## 10. v0.1.0 Release Contract

只有满足以下条件才允许打 `v0.1.0` tag：

- workspace compile/check 通过
- lockfile 与 0.1.0 对齐
- Python wheel 构建成功
- wheel 实际安装并运行 SMA smoke test
- Python 3.8/3.11/3.13 ABI3 验证通过
- 版本一致性 CI 通过
- GitHub Release 可以产出 Python 与 CLI 安装资产
