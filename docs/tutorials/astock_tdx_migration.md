# A股场景：从通达信 TDX 公式迁移到 AlphaTA

本教程面向 A 股量化用户，演示如何将通达信（TDX）常用技术指标公式迁移到 AlphaTA Python 绑定，并完成数据导入、指标计算、结果对比与可视化。

## 目录

1. [安装 AlphaTA](#1-安装-AlphaTA)
2. [导入 A 股 OHLCV 数据](#2-导入-a-股-ohlcv-数据)
3. [TDX 公式与 AlphaTA 对照](#3-tdx-公式与-AlphaTA-对照)
4. [运行 MACD / KDJ / BOLL](#4-运行-macd--kdj--boll)
5. [与通达信结果对比](#5-与通达信结果对比)
6. [可视化](#6-可视化)

---

## 1. 安装 AlphaTA

### 从 PyPI 安装（推荐）

```bash
pip install alpha-ta numpy pandas
```

### 从源码构建（开发版）

```bash
cd ffi/python-binding
pip install maturin
maturin develop --release
```

验证安装：

```python
import finkit as ta
import numpy as np

close = np.arange(1, 101, dtype=np.float64)
rsi = ta.rsi(close, timeperiod=14)
print(f"RSI length: {len(rsi)}")
```

---

## 2. 导入 A 股 OHLCV 数据

### 2.1 从 CSV 导入（通达信导出）

通达信可导出日线数据为 CSV。典型列名：`日期,开盘,最高,最低,收盘,成交量`。

```python
import pandas as pd

df = pd.read_csv("600519_daily.csv", encoding="gbk")  # A 股 CSV 常为 GBK
df.columns = ["date", "open", "high", "low", "close", "volume"]

open_  = df["open"].astype(float).values
high   = df["high"].astype(float).values
low    = df["low"].astype(float).values
close  = df["close"].astype(float).values
volume = df["volume"].astype(float).values
```

### 2.2 从 Parquet 导入

```python
df = pd.read_parquet("600519_daily.parquet")
```

### 2.3 使用 AlphaTA KlineData（内置可视化数据格式）

```python
import finkit as ta

data = ta.KlineData.from_csv(open("600519_daily.csv", encoding="gbk").read())
assert data.validate()
```

更多格式互转示例见 [`examples/data_formats.py`](../../examples/data_formats.py)。

---

## 3. TDX 公式与 AlphaTA 对照

| 通达信公式 | 含义 | AlphaTA 等价调用 |
|-----------|------|----------------|
| `MA(C,20)` | 20 日简单移动平均 | `ta.sma(close, timeperiod=20)` |
| `EMA(C,12)` | 12 日指数移动平均 | `ta.ema(close, timeperiod=12)` |
| `MACD` | MACD 三线 | `ta.macd(close, 12, 26, 9)` |
| `KDJ` | 随机指标 K/D | `ta.stoch(high, low, close, 9, 3, 3)` |
| `BOLL` | 布林带 | `ta.bollinger_bands(close, 20, 2.0, 2.0)` |
| `RSI` | 相对强弱 | `ta.rsi(close, timeperiod=14)` |
| `CROSS(A,B)` | A 上穿 B | 公式引擎或 Python 逻辑 |
| `REF(X,N)` | 引用 N 周期前 | `ta.ref(close, 5)` 或 `np.roll` |

### TDX MACD 公式示例

通达信默认 MACD：

```
DIF: EMA(CLOSE,12) - EMA(CLOSE,26);
DEA: EMA(DIF,9);
MACD: (DIF-DEA)*2;
```

AlphaTA 等价：

```python
import finkit as ta

macd_line, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
# macd_line ≈ DIF, signal ≈ DEA, hist ≈ (DIF-DEA)*2
```

### TDX KDJ 公式示例

```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100;
K:SMA(RSV,3,1);
D:SMA(K,3,1);
J:3*K-2*D;
```

AlphaTA 等价：

```python
slowk, slowd = ta.stoch(high, low, close, fastk_period=9, slowk_period=3, slowd_period=3)
j = 3 * slowk - 2 * slowd
```

---

## 4. 运行 MACD / KDJ / BOLL

完整示例脚本：

```python
import finkit as ta
import numpy as np
import pandas as pd

# 模拟 A 股日线数据（替换为真实 CSV）
n = 120
np.random.seed(42)
close = np.cumsum(np.random.randn(n)) + 50.0
high  = close + np.random.uniform(0.2, 1.5, n)
low   = close - np.random.uniform(0.2, 1.5, n)
open_ = close + np.random.uniform(-0.5, 0.5, n)

# --- MACD ---
macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)

# --- KDJ ---
slowk, slowd = ta.stoch(high, low, close, fastk_period=9, slowk_period=3, slowd_period=3)
j = 3 * slowk - 2 * slowd

# --- BOLL ---
upper, middle, lower = ta.bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)

# 合并到 DataFrame
df = pd.DataFrame({
    "close": close,
    "macd": macd,
    "macd_signal": signal,
    "macd_hist": hist,
    "kdj_k": slowk,
    "kdj_d": slowd,
    "kdj_j": j,
    "boll_upper": upper,
    "boll_mid": middle,
    "boll_lower": lower,
})

print(df.tail(5))
```

### 批量计算（单次 GIL 释放，更快）

```python
requests = [
    ("macd", [12, 26, 9]),
    ("stoch", [9, 3, 3]),
    ("bollinger_bands", [20, 2.0, 2.0]),
]
results = ta.compute_indicators(
    close=close, requests=requests, high=high, low=low
)
```

### DataFrame 访问器（目标 API）

```python
# 当 df.ta 访问器可用时：
# rsi = df.ta.rsi(14)
# enriched = df.ta.strategy([("macd", [12, 26, 9]), ("rsi", [14])])
```

---

## 5. 与通达信结果对比

### 5.1 导出通达信指标值

在通达信中，使用「公式管理器」将指标叠加到 K 线，然后通过「数据导出」或第三方工具导出指标数值列。

### 5.2 数值对比脚本

```python
import numpy as np
import pandas as pd

# tdx_export.csv: 从通达信导出的 MACD 列
tdx = pd.read_csv("tdx_macd_export.csv", encoding="gbk")
alphata_macd = macd  # 来自上一节

# 对齐有效区间（warm-up 期 AlphaTA 输出 NaN）
valid = ~np.isnan(alphata_macd)
diff = np.abs(alphata_macd[valid] - tdx["DIF"].values[valid])
max_diff = np.nanmax(diff)
mean_diff = np.nanmean(diff)

print(f"MACD DIF max diff: {max_diff:.6f}")
print(f"MACD DIF mean diff: {mean_diff:.6f}")

# 通常浮点误差 < 1e-6 即可认为一致
assert max_diff < 1e-4, "MACD 与通达信差异过大，请检查参数"
```

### 5.3 常见差异来源

| 差异原因 | 说明 |
|---------|------|
| 复权方式 | 前复权/后复权/不复权数据不同 |
| 参数默认值 | 确认 TDX 与 AlphaTA 周期参数一致 |
| Warm-up 期 | 前 N 根 K 线为 NaN，对比时跳过 |
| EMA 初始化 | 少数软件 EMA 种子值算法略有差异 |

---

## 6. 可视化

### 6.1 使用 AlphaTA 内置 K 线图

```python
import finkit as ta

dates = [f"2024-01-{i+1:02d}" for i in range(n)]
kline = ta.KlineData(
    dates=dates,
    opens=open_.tolist(),
    highs=high.tolist(),
    lows=low.tolist(),
    closes=close.tolist(),
    volumes=[1000.0] * n,
)

chart = ta.KlineChart(kline, language="zh", title="600519 日线", width=1200, height=600)
chart.add_indicator("MACD", ta.IndicatorType.MACD)
chart.add_indicator("KDJ", ta.IndicatorType.KDJ)
chart.add_indicator("BOLL", ta.IndicatorType.BOLL)

html = chart.to_html()
with open("600519_chart.html", "w", encoding="utf-8") as f:
    f.write(html)
print("图表已保存: 600519_chart.html")
```

### 6.2 使用 matplotlib

```python
import matplotlib.pyplot as plt

fig, axes = plt.subplots(4, 1, figsize=(14, 10), sharex=True)

axes[0].plot(close, label="Close")
axes[0].plot(upper, label="BOLL Upper", alpha=0.7)
axes[0].plot(middle, label="BOLL Mid", alpha=0.7)
axes[0].plot(lower, label="BOLL Lower", alpha=0.7)
axes[0].legend()
axes[0].set_title("价格 + 布林带")

axes[1].plot(macd, label="DIF")
axes[1].plot(signal, label="DEA")
axes[1].bar(range(n), hist, label="MACD", alpha=0.4)
axes[1].legend()
axes[1].set_title("MACD")

axes[2].plot(slowk, label="K")
axes[2].plot(slowd, label="D")
axes[2].plot(j, label="J")
axes[2].legend()
axes[2].set_title("KDJ")

plt.tight_layout()
plt.savefig("600519_indicators.png", dpi=150)
print("图表已保存: 600519_indicators.png")
```

---

## 下一步

- [加密货币流式指标教程](crypto_realtime.md)
- [数据格式互转示例](../../examples/data_formats.py)
- [API 参考](../api-reference.md)
- [公式引擎文档](../formula.md)
