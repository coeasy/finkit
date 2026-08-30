# Finkit v0.1.0 优化与首发实现方案

## 1. 定位

Finkit v0.1.0 定位为高性能、跨语言、可扩展的开源金融指标、因子与公式计算基础库。

它不是行情平台、交易系统或回测平台，核心边界是：

```text
金融时间序列 -> 指标/因子/公式 -> 高性能计算结果
```

长期目标包括兼容国内外主要交易终端的公式体系，并通过统一 Rust Core 向 Python、Node.js、Java、.NET、Go、C/C++、WASM、Android、iOS 暴露能力。

## 2. v0.1.0 技术基线

现有代码已经形成成熟 Rust workspace，因此 v0.1.0 不进行 C++ 重写，而以现有 Rust 内核为唯一计算核心：

- Rust 2021，MSRV 1.75
- `alpha-ta-core` 作为 v0.1.x 内部兼容 crate 名
- SIMD / streaming / batch / formula 能力继续复用
- 多语言 FFI 共用同一套核心算法，禁止各语言重复实现指标
- 公共项目品牌、版本和新安装入口统一为 Finkit v0.1.0

## 3. v0.1.0 首发目标

首个版本必须形成真正可安装闭环，而不是只保证源码可编译。

### Python

公共包：

```bash
pip install finkit
```

版本：

```text
finkit==0.1.0
```

公共入口：

```python
import finkit
finkit.sma(...)
finkit.rsi(...)
finkit.macd(...)
```

Rust/PyO3 原生模块在 v0.1.x 内部保留 `alpha_ta` 名称，通过 `finkit.alpha_ta` 封装，避免一次性破坏已有绑定。

### CLI

生成公共可执行文件：

```bash
finkit
```

支持现有指标、公式、流式计算、特征、参数扫描和图表等命令能力。

### GitHub Release

Tag：

```text
v0.1.0
```

首发 Assets：

- Linux Python wheel
- macOS Python wheel
- Windows Python wheel
- Python sdist
- Linux `finkit` CLI
- macOS `finkit` CLI
- Windows `finkit.exe` CLI

## 4. 多语言版本统一

v0.1.0 要求所有发布元数据统一为 `0.1.0`：

| 组件 | v0.1.0 版本来源 |
|---|---|
| Rust workspace | `Cargo.toml [workspace.package]` |
| Python | `ffi/python-binding/pyproject.toml` |
| Python runtime | `finkit.__version__` |
| Node.js | `ffi/node-binding/package.json` |
| Node native packages | `ffi/node-binding/npm/*/package.json` |
| Java | `ffi/java-binding/pom.xml` |
| .NET | `AlphaTA.csproj` |
| Go / C / WASM / iOS / Android crates | Rust workspace version / Git tag |

增加 `scripts/check_version_alignment.py` 和 CI gate，禁止同一 release 出现 `0.1.0` / `1.0.0` 混用。

## 5. Node.js 安装链修复

历史代码存在主 loader 引用 `@alphata/core-<platform>`，而平台包实际声明为 `@alphata/node-<platform>` 的断链。

v0.1.0 统一平台包为：

```text
@alphata/core-darwin-arm64
@alphata/core-darwin-x64
@alphata/core-linux-arm64-gnu
@alphata/core-linux-arm64-musl
@alphata/core-linux-x64-gnu
@alphata/core-linux-x64-musl
@alphata/core-win32-arm64-msvc
@alphata/core-win32-x64-msvc
```

v0.1.x 暂保留 `@alphata` scope 作为兼容层，后续单独设计无破坏的 Finkit namespace 迁移。

## 6. 质量门禁

合并 v0.1.0 前至少通过：

1. Rust workspace `cargo check`
2. Rust core tests
3. 版本一致性检查
4. Cargo.lock 与 workspace 版本一致
5. Python release wheel 能构建
6. wheel 安装后 `import finkit` 成功
7. Python 3.8 / 3.11 / 3.13 ABI3 smoke test
8. `finkit.sma()` 实际执行成功
9. Node package/platform package 名称契约一致
10. GitHub Release workflow 可生成安装资产

## 7. 公式系统路线

Finkit 的核心差异化不是堆叠 UI，而是建立统一 Formula IR 与兼容层：

```text
Terminal Formula
   -> Dialect Adapter
   -> Parser / AST
   -> Normalized Formula IR
   -> Optimizer
   -> Vector / Streaming Runtime
   -> Result
```

优先兼容顺序：

1. 通达信
2. 同花顺
3. 东方财富常用公式语义
4. TradingView Pine Script 常用技术指标子集
5. Finkit 原生 DSL

不同终端的差异放在 dialect adapter，不复制指标算法。

## 8. 性能原则

- 所有语言绑定调用同一 Rust Core
- 批量计算优先连续内存和零拷贝
- 热点指标保留 SIMD 优化
- 实时场景使用 streaming O(1) 增量状态
- 大股票池支持并行 batch
- 每次性能重构必须保留 TA-Lib 对比基准和回归门禁
- 正确性优先于微基准结果，所有优化必须经过 golden vectors / precision test

## 9. v0.1.0 之后

v0.1.x：稳定安装、补齐公式兼容、完善类型/API 文档和多平台资产。

v0.2.0：扩大公式 dialect、因子 API 和跨语言公开 Finkit namespace。

v1.0.0：稳定 ABI/API、完整终端兼容矩阵、成熟多语言 registry 发布。
