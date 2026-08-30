# 加密货币场景：流式指标实时更新

本教程面向加密货币量化与做市场景，演示如何使用 Finkit 流式（Streaming）指标 API 对实时 K 线流进行增量更新，并生成 RSI / EMA 信号。

## 目录

1. [安装与环境](#1-安装与环境)
2. [数据接入](#2-数据接入)
3. [流式 RSI / EMA 更新](#3-流式-rsi--ema-更新)
4. [策略信号生成](#4-策略信号生成)
5. [状态持久化与恢复](#5-状态持久化与恢复)
6. [完整实时循环示例](#6-完整实时循环示例)

---

## 1. 安装与环境

```bash
pip install finkit numpy
# 可选：WebSocket 数据源
pip install websockets
```

验证流式 API：

```python
import finkit as ta

rsi = ta.StreamingRSI(period=14)
print(rsi.update(50000.0))  # 首根 K 线，可能为 NaN
```

---

## 2. 数据接入

### 2.1 从交易所 REST API 拉取历史 K 线

```python
import json
import urllib.request

def fetch_binance_klines(symbol="BTCUSDT", interval="1m", limit=100):
    url = (
        f"https://api.binance.com/api/v3/klines"
        f"?symbol={symbol}&interval={interval}&limit={limit}"
    )
    with urllib.request.urlopen(url, timeout=10) as resp:
        data = json.loads(resp.read())
    closes = [float(row[4]) for row in data]
    highs  = [float(row[2]) for row in data]
    lows   = [float(row[3]) for row in data]
    return highs, lows, closes
```

### 2.2 从 CSV / Parquet 回放（回测式流式）

```python
import csv

def load_closes_from_csv(path: str) -> list[float]:
    with open(path, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    return [float(r["close"]) for r in rows]
```

### 2.3 WebSocket 实时推送（示意）

```python
# 需要 websockets 库
# async def on_kline(close_price: float):
#     signal = strategy.update(close_price)
#     if signal:
#         print(f"Signal: {signal}")
```

生产环境建议使用交易所官方 SDK 或成熟的行情网关，本教程聚焦 Finkit 指标层。

---

## 3. 流式 RSI / EMA 更新

Finkit 提供与批量 API 数值一致的流式指标类，每根新 K 线调用 `update()` 即可获得最新值，无需重算全量历史。

### 3.1 流式 RSI

```python
import finkit as ta

streaming_rsi = ta.StreamingRSI(period=14)

prices = [42000, 42100, 41950, 42200, 42300, 42150, 42400, 42500]
rsi_values = []

for price in prices:
    rsi_val = streaming_rsi.update(price)
    rsi_values.append(rsi_val)
    print(f"Price: {price:.2f}  RSI: {rsi_val:.2f}")

print(f"Ready: {streaming_rsi.is_ready()}, Count: {streaming_rsi.count()}")
```

### 3.2 流式 EMA

```python
streaming_ema = ta.StreamingEMA(period=20)

for price in prices:
    ema_val = streaming_ema.update(price)
    print(f"Price: {price:.2f}  EMA(20): {ema_val:.2f}")
```

### 3.3 批量预热后切换流式

先用历史数据预热状态，再接入实时流：

```python
import finkit as ta

# 历史预热
history = fetch_binance_klines(limit=100)[2]  # closes
streaming_rsi = ta.StreamingRSI(period=14)

for price in history:
    streaming_rsi.update(price)

print(f"预热完成，RSI ready: {streaming_rsi.is_ready()}")

# 实时更新
new_price = 42600.0
live_rsi = streaming_rsi.update(new_price)
print(f"实时 RSI: {live_rsi:.2f}")
```

### 3.4 流式 vs 批量一致性验证

```python
import finkit as ta
import numpy as np

closes = np.array(prices, dtype=np.float64)

# 批量
batch_rsi = ta.rsi(closes, timeperiod=14)

# 流式
stream_rsi = ta.StreamingRSI(period=14)
stream_vals = [stream_rsi.update(p) for p in closes]

# 对比最后一个有效值
last_valid = -1
np.testing.assert_allclose(batch_rsi[last_valid], stream_vals[last_valid], rtol=1e-10)
print("流式与批量结果一致")
```

### 3.5 其他可用流式指标

| 类名 | 用途 |
|------|------|
| `StreamingSMA` | 简单移动平均 |
| `StreamingEMA` | 指数移动平均 |
| `StreamingWMA` | 加权移动平均 |
| `StreamingRSI` | 相对强弱指数 |
| `StreamingMACD` | MACD 三线 |
| `StreamingATR` | 平均真实波幅（需 OHLCV bar） |
| `StreamingBollingerBands` | 布林带 |

---

## 4. 策略信号生成

### 4.1 RSI 超买超卖信号

```python
import finkit as ta

class RsiSignalStrategy:
    def __init__(self, period: int = 14, oversold: float = 30.0, overbought: float = 70.0):
        self.rsi = ta.StreamingRSI(period=period)
        self.oversold = oversold
        self.overbought = overbought

    def update(self, close: float) -> str | None:
        rsi_val = self.rsi.update(close)
        if not self.rsi.is_ready():
            return None
        if rsi_val < self.oversold:
            return "BUY"
        if rsi_val > self.overbought:
            return "SELL"
        return None
```

### 4.2 EMA 交叉策略

```python
import finkit as ta

class EmaCrossStrategy:
    def __init__(self, fast: int = 12, slow: int = 26):
        self.fast_ema = ta.StreamingEMA(period=fast)
        self.slow_ema = ta.StreamingEMA(period=slow)
        self.prev_fast = None
        self.prev_slow = None

    def update(self, close: float) -> str | None:
        fast = self.fast_ema.update(close)
        slow = self.slow_ema.update(close)

        if not self.fast_ema.is_ready() or not self.slow_ema.is_ready():
            self.prev_fast, self.prev_slow = fast, slow
            return None

        signal = None
        if self.prev_fast is not None and self.prev_slow is not None:
            if self.prev_fast <= self.prev_slow and fast > slow:
                signal = "BUY"   # 金叉
            elif self.prev_fast >= self.prev_slow and fast < slow:
                signal = "SELL"  # 死叉

        self.prev_fast, self.prev_slow = fast, slow
        return signal
```

### 4.3 多指标组合信号

```python
class MomentumStrategy:
    def __init__(self):
        self.rsi = ta.StreamingRSI(period=14)
        self.ema_fast = ta.StreamingEMA(period=12)
        self.ema_slow = ta.StreamingEMA(period=26)

    def update(self, close: float) -> str | None:
        rsi = self.rsi.update(close)
        fast = self.ema_fast.update(close)
        slow = self.ema_slow.update(close)

        if not all(x.is_ready() for x in (self.rsi, self.ema_fast, self.ema_slow)):
            return None

        if fast > slow and rsi < 35:
            return "BUY"
        if fast < slow and rsi > 65:
            return "SELL"
        return None
```

---

## 5. 状态持久化与恢复

流式指标支持序列化状态，适合断线重连或服务重启后恢复计算上下文。

```python
import finkit as ta

rsi = ta.StreamingRSI(period=14)
for price in [42000, 42100, 42200, 42300]:
    rsi.update(price)

# 保存状态（bytes）
state_bytes = rsi.save_state()

# 恢复状态
rsi_restored = ta.StreamingRSI.restore_state(state_bytes)
new_rsi = rsi_restored.update(42400.0)
print(f"恢复后 RSI: {new_rsi:.2f}")
```

---

## 6. 完整实时循环示例

以下脚本用历史数据模拟实时推送，演示完整工作流：

```python
"""模拟加密货币实时 K 线流 + 流式指标 + 信号输出"""
import time
import finkit as ta

def simulate_realtime_stream(closes: list[float], delay: float = 0.05):
    rsi_strategy = ta.StreamingRSI(period=14)
    ema_fast = ta.StreamingEMA(period=12)
    ema_slow = ta.StreamingEMA(period=26)

    for i, price in enumerate(closes):
        rsi_val = rsi_strategy.update(price)
        fast = ema_fast.update(price)
        slow = ema_slow.update(price)

        if rsi_strategy.is_ready():
            status = []
            if rsi_val < 30:
                status.append("RSI oversold → BUY bias")
            elif rsi_val > 70:
                status.append("RSI overbought → SELL bias")
            if fast > slow:
                status.append("EMA bullish")
            else:
                status.append("EMA bearish")

            print(
                f"[{i:03d}] close={price:.2f}  "
                f"RSI={rsi_val:.1f}  EMA12={fast:.2f}  EMA26={slow:.2f}  "
                f"{' | '.join(status)}"
            )

        time.sleep(delay)


if __name__ == "__main__":
    # 替换为 fetch_binance_klines() 或 CSV 回放
    demo_prices = [
        42000, 42150, 41900, 42200, 42400, 42300, 42500, 42650,
        42500, 42700, 42800, 42600, 42900, 43000, 42850, 43100,
        43200, 43000, 43300, 43400, 43250, 43500, 43600, 43400,
        43700, 43800, 43650, 43900, 44000, 43850,
    ]
    simulate_realtime_stream(demo_prices, delay=0.0)
```

运行：

```bash
python crypto_realtime_demo.py
```

---

## 性能提示

| 场景 | 建议 |
|------|------|
| 实时单指标 | `StreamingRSI` / `StreamingEMA`，O(1) 每根 K 线 |
| 多指标批跑 | 历史段用 `compute_indicators()` 单次 GIL 释放 |
| 高频 tick | 流式 API + 状态持久化，避免全量重算 |
| 回测 | 批量 API 更快；流式 API 用于验证一致性 |

---

## 下一步

- [A 股 TDX 迁移教程](astock_tdx_migration.md)
- [数据格式互转](../../examples/data_formats.py)
- [流式指标 API](../api-reference.md)
- [Python 绑定 README](../../ffi/python-binding/README.md)
