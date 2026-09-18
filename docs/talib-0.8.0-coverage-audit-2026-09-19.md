# TA-Lib 0.8.0 覆盖审计（2026-09-19）

本审计用于防止“版本号已升级”被误认为“完整兼容”。它只记录已经从真实 TA-Lib Python 0.8.0 wheel 读取到的公开函数目录，以及当前 Finkit 代码中实际进入共享 dispatcher、catalog 和 numeric golden 的函数集合。

## 版本边界

- TA-Lib C upstream 当前公开 release 为 `0.7.1`（2026-07-03）。
- TA-Lib Python 最新 release 为 `0.8.0`（2026-09-13）。该 Python wheel 声明最低支持 TA-Lib C `0.8.1`，而 C `0.8.1` 在当前日期仍显示为未 release；因此不能把 C `0.7.1` profile 与 Python `0.8.0` 的新增目录称为同一套完整 upstream 语义。
- 本仓库当前固定 numeric corpus 标记为 Python `0.8.0`，并按该 wheel 内置的 TA-Lib core `0.8.1` 完成 201 个公开函数的 dispatcher、catalog 与 numeric golden 对照。

## 已验证差集

真实 Python 0.8.0 wheel 的 `talib.get_functions()` 返回 201 个公开函数；当前已验证 201 个，剩余差集为 0：

```text
<none>
```

本轮新增的 21 个函数均已建立独立 TA-Lib profile adapter；与 Core 中同名但输入/输出不同的 ADR、KDJ、QSTICK、RVI 等没有复用旧语义。

还有同名但输入/输出语义不同的边界，必须先做显式适配而不能复用名称：

- TA-Lib `RVI(real, timeperiod, stddevperiod)` 是单输入、单输出的 Relative Volatility Index；当前 Core `rvi` 是 OHLC 输入并返回 RVI/Signal 两条线。
- TA-Lib `ADR(high, low, timeperiod)` 是高低价平均区间；当前 Core `adr` 还带 close 和 `AdrMode`，不能静默当成同一接口。
- TA-Lib `QSTICK` 的 MA 类型、`KDJ` 的 RMA 默认类型、`PVO` 的百分比输出和 `SUPERTREND` 的方向输出都必须以 0.8.0 contract 为准。
- `HA`、`FRACTAL` 等返回整数或多输出结果，必须固定输出名称、类型、warm-up 和 null/NaN 规则，不能只注册函数名。

## 当前结论

当前状态是：

- TA-Lib 0.8.0 / bundled core 0.8.1 语义 profile：201 个可执行 dispatcher 名称，201 个固定 numeric golden；
- TA-Lib Python 0.8.0 公开目录：201 个函数；
- 0.8.0 目录覆盖：已完成公开函数目录级 dispatcher、参数目录和 numeric golden 对照；仍需按发布平台继续扩展端到端 binding/ABI 矩阵；
- “超越 TA-Lib”性能结论：未完成。现有 benchmark 只能证明已测 workload 的结果，不能外推到缺失函数或所有硬件。

## 生产化收敛顺序

1. 已完成 21 个函数的 profile adapter、默认参数、输出 schema 和 warm-up contract。
2. 已完成来自 TA-Lib Python 0.8.0 wheel（内置 C 0.8.1）的三数据集 golden；零窗口、零成交量、period=1 和多输出边界按函数单独处理。
3. 已完成统一 dispatcher、201 项 catalog smoke、201 项 numeric golden 和 coverage matrix 一致性验证。
4. 后续生产门禁仍需按发布平台持续运行完整 ABI/宿主矩阵；本审计不把单机 workspace 通过外推为所有操作系统和编译器已验证。

## 可重复审计

```powershell
python scripts/audit_talib_surface.py
python scripts/audit_talib_surface.py --require-complete
```

该脚本会检查当前 Python wrapper 版本、upstream 函数目录、仓库 numeric reference 差集；它不写 golden 文件。正式 golden 仍只能由 `scripts/gen_talib_golden.py` 在版本匹配环境中生成。
