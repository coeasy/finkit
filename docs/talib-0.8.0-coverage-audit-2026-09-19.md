# TA-Lib 0.8.0 覆盖审计（2026-09-19）

本审计用于防止“版本号已升级”被误认为“完整兼容”。它只记录已经从真实 TA-Lib Python 0.8.0 wheel 读取到的公开函数目录，以及当前 Finkit 代码中实际进入共享 dispatcher、catalog 和 numeric golden 的函数集合。

## 版本边界

- TA-Lib C upstream 当前公开 release 为 `0.7.1`（2026-07-03）。
- TA-Lib Python 最新 release 为 `0.8.0`（2026-09-13）。该 Python wheel 声明最低支持 TA-Lib C `0.8.1`，而 C `0.8.1` 在当前日期仍显示为未 release；因此不能把 C `0.7.1` profile 与 Python `0.8.0` 的新增目录称为同一套完整 upstream 语义。
- 本仓库当前固定 numeric corpus 标记为 Python `0.8.0`，已完成 180 个函数的 dispatcher、catalog 与 numeric golden 对照，仍不是 Python `0.8.0` 的全目录。

## 已验证差集

真实 Python 0.8.0 wheel 的 `talib.get_functions()` 返回 201 个公开函数；当前已验证 180 个，剩余差集为 21 个：

```text
AC ADR CMOU CVI EFI ERI FOSC FRACTAL KC KDJ MARKETFI MASSI PERCENTILE PVO QSTICK
RMA RVI RVOL SMI VHF WAD
```

其中剩余函数的一部分已有 Finkit Core/Formula/streaming 实现，但尚未经过 TA-Lib 0.8.0 的统一 profile dispatcher 和固定 numeric reference，不能因此直接宣称 TA-Lib 兼容已完成。

还有同名但输入/输出语义不同的边界，必须先做显式适配而不能复用名称：

- TA-Lib `RVI(real, timeperiod, stddevperiod)` 是单输入、单输出的 Relative Volatility Index；当前 Core `rvi` 是 OHLC 输入并返回 RVI/Signal 两条线。
- TA-Lib `ADR(high, low, timeperiod)` 是高低价平均区间；当前 Core `adr` 还带 close 和 `AdrMode`，不能静默当成同一接口。
- TA-Lib `QSTICK` 的 MA 类型、`KDJ` 的 RMA 默认类型、`PVO` 的百分比输出和 `SUPERTREND` 的方向输出都必须以 0.8.0 contract 为准。
- `HA`、`FRACTAL` 等返回整数或多输出结果，必须固定输出名称、类型、warm-up 和 null/NaN 规则，不能只注册函数名。

## 当前结论

当前状态是：

- TA-Lib 0.7.1 语义 profile：180 个可执行 dispatcher 名称，180 个固定 numeric golden；
- TA-Lib Python 0.8.0 公开目录：201 个函数；
- 0.8.0 全目录等价：未完成，缺 21 个函数及其跨语言 contract；
- “超越 TA-Lib”性能结论：未完成。现有 benchmark 只能证明已测 workload 的结果，不能外推到缺失函数或所有硬件。

## 生产化收敛顺序

1. 将剩余 21 个函数按 overlap/momentum/volatility/volume/price/statistics 分组，先确定 0.8.0 的输入签名、默认参数和输出 schema。
2. 对已有 Core 内核逐个增加 TA-Lib profile adapter；对 RVI、ADR、QSTICK 等同名冲突建立独立 profile kernel，禁止复用旧函数的参数含义。
3. 为每个函数生成来自 TA-Lib Python 0.8.0 wheel（其内置 C 0.8.1）的三数据集 golden，并把 warm-up、NaN、zero-volume、period=1、别名和多输出逐项固定。
4. 只有 dispatcher smoke、numeric golden、streaming/append conformance 和八语言 JSON contract 同时通过，才把函数加入 `talib_0_8_0` production catalog。
5. 在 201/201 完成前，所有文档和 binding metadata 必须显示真实覆盖数，不得生成“全量支持”或“完整等价”声明。

## 可重复审计

```powershell
python scripts/audit_talib_surface.py
python scripts/audit_talib_surface.py --require-complete
```

该脚本会检查当前 Python wrapper 版本、upstream 函数目录、仓库 numeric reference 差集；它不写 golden 文件。正式 golden 仍只能由 `scripts/gen_talib_golden.py` 在版本匹配环境中生成。
