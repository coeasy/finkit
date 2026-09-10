# 市场日历与时区适配

## 目标

Finkit 的多周期聚合、缠论结构和图表时间轴必须使用同一份 session 语义：

- 交易日、周末和节假日由日历决定；
- session 由市场 preset 或用户覆盖决定；
- Unix 时间戳先转换为交易所本地时间，再按 session 开盘对齐；
- 跨午夜 session 归属到开盘日；
- 半日市、临时休市和品种差异不写死在指标核心，而由配置注入。

## 内置市场 preset

| preset | 时区 | 常规 session | 说明 |
| --- | --- | --- | --- |
| `a_share` | `Asia/Shanghai` | 09:30-11:30、13:00-15:00 | 不包含集合竞价；年度假日由用户注入 |
| `china_futures` | `Asia/Shanghai` | 09:00-10:15、10:30-11:30、13:30-15:00、21:00-23:00 | 不同品种夜盘不同，可整体覆盖 session |
| `hong_kong` | `Asia/Hong_Kong` | 09:30-12:00、13:00-16:00 | 半日市使用日期覆盖 |
| `us_equity` | `America/New_York` | 09:30-16:00 | 内置主要美股假日和 DST；早盘/盘后需另行配置 |
| `crypto` | `UTC` | 24×7 | 可通过 holiday/override 配置维护所使用平台的维护窗口 |

常规时段参考交易所公开规则：上交所列出的连续交易时段为 09:30-11:30、13:00-15:00，[HKEX 证券市场](https://www.hkex.com.hk/Services/Trading-hours-and-Severe-Weather-Arrangements/Trading-Hours/Securities-Market)列出 09:30-12:00 和 13:00-16:00，[NYSE](https://www.nyse.com/trade/trading-information?force_isolation=true)的核心交易时段为纽约时间 09:30-16:00。期货不应被当成一个单一 session：CME 官方说明其市场接近全天交易且不同产品有独立交易时间，因此国内期货也必须允许按品种覆盖夜盘。

## Rust API

```rust
use finkit::calendar::{
    MarketCalendarPreset, SessionWindow, TimeZoneSpec, TradingCalendar,
};

let mut calendar = TradingCalendar::for_market(MarketCalendarPreset::UsEquity)
    .with_timezone(TimeZoneSpec::parse("America/New_York")?);

// 由交易所年度日历或行情适配器注入。
calendar.add_holiday("2026-07-03")?;

// 例：感恩节翌日半日市；空数组表示整日休市。
calendar.set_special_sessions(
    "2026-11-27",
    &[SessionWindow::new(9 * 3600 + 30 * 60, 13 * 3600)?],
)?;

let session = calendar.session_for_timestamp_local(unix_seconds)?;
```

`TradingCalendar::for_market_name` 接受 `a_share`、`china_futures`、
`hong_kong`、`us_equity`、`crypto`，也接受常用中文别名。`TimeZoneSpec`
支持 UTC、固定偏移、上海、香港和纽约；纽约会按美国 DST 规则转换。

## Python API

```python
result = finkit.chan_analyze_multi_timestamps_calendar(
    timestamps, open_, high, low, close, volume,
    market="china_futures",
    timezone="Asia/Shanghai",
    durations_seconds=[5 * 60, 30 * 60],
    holidays=["2026-10-01", "2026-10-02"],
    # 覆盖当前品种的常规 session
    sessions=[
        (9 * 3600, 10 * 3600 + 15 * 60),
        (10 * 3600 + 30 * 60, 11 * 3600 + 30 * 60),
        (13 * 3600 + 30 * 60, 15 * 3600),
        (21 * 3600, 2 * 3600 + 30 * 60),
    ],
    special_sessions=[
        ("2026-09-30", [(9 * 3600, 11 * 3600 + 30 * 60)]),
    ],
)
```

## Node 与 WASM

Node 提供 `resolveMarketSession()`，可在 WebSocket/行情适配器收到报价后先判断
其所属 session，再送入图表或多周期聚合。WASM 提供同名解析函数，参数同样支持
可选时区、节假日、常规 `sessions` 和日期级 `specialSessions` 覆盖，适合浏览器端
时间轴和数据窗口使用。两者都复用 Rust 核心，避免 JavaScript 与 Python 各自实现
一套 DST/跨午夜逻辑。

三端还提供 `resolve_market_session_csv` / `resolveMarketSessionCsv`：应用可以
直接把交易所或内部审计流程导出的年度 CSV 交给核心解析，不需要先转换成 JSON。
CSV 契约如下：

```text
date,status,sessions
2026-01-01,closed,
2026-01-02,open,09:30-11:30;13:00-15:00
```

`status` 支持 `open/trading/1/true` 与 `closed/holiday/0/false`；`sessions`
使用本地交易所时间，多个时段用分号分隔。空时段的 `closed` 行表示整日休市，
有时段的 `open` 行表示该日期的特殊 session。解析器严格校验日期、时钟和区间，
错误会在 Rust、Python、Node、WASM 返回，不会静默吞掉年度日历错误。

## 年度日历数据策略

交易所假日和临时安排是外部变化数据，建议行情接入层维护如下版本化配置：

```json
{
  "market": "a_share",
  "year": 2026,
  "holidays": ["2026-01-01", "2026-02-16"],
  "special_sessions": [
    {"date": "2026-02-13", "sessions": []},
    {"date": "2026-09-30", "sessions": [[34200, 41400]]}
  ],
  "source": "exchange-official-calendar",
  "revision": "2026.1"
}
```

启动时将配置加载进 `TradingCalendar`，并把 `source/revision` 写入行情和图表
元数据。这样回放、回测和线上实时图表可以复现同一份日历；日历更新时也能按
版本重新计算，而不会静默改变历史结果。

## 验收要求

1. 同一时间戳在 Rust、Python、Node、WASM 返回相同的 session 开始日和 UTC 边界。
2. 美股 DST 切换前后 09:30 本地时间分别映射到不同 UTC 偏移。
3. 周末、节假日、午休和 session 外报价不会进入高周期聚合。
4. 跨午夜夜盘归属开盘日，且源索引范围可以回溯到原始行情。
5. 期货品种、半日市和临时休市只需替换配置，不修改指标或缠论算法。
6. 年度 CSV 与 JSON 配置在同一日期、时区和 session 下返回一致结果；需要源配置
   版本审计时，使用 JSON 的 `source/revision` 字段并随图表元数据保存。
