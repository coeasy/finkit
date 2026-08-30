# Formula Corpus

真实公式语料回归集，收录通达信（TDX）、同花顺（THS）、大智慧（DZH）及跨平台方言差异的经典技术指标公式。每条语料用于验证 AlphaTA 公式引擎在多平台语法下的计算一致性。

## 目录结构

```
tests/formula_corpus/
├── README.md                  # 本文件
├── macd_tdx.json              # TDX MACD
├── kdj_tdx.json               # TDX KDJ
├── boll_tdx.json              # TDX BOLL
├── ...                        # 更多语料
└── cross_period_refdate.json  # 跨周期引用
```

## 语料 JSON 格式

每条语料是一个独立的 JSON 文件，字段说明如下：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 唯一标识，建议格式 `{indicator}_{platform}` |
| `platform` | string | 是 | 来源平台：`TDX` / `THS` / `DZH` / `CROSS` |
| `source_formula` | string | 是 | 原始公式文本（分号分隔多条语句） |
| `description` | string | 是 | 中文描述 |
| `input` | object | 是 | 输入数据配置（见下表） |
| `expected_output_columns` | array | 是 | 期望输出的变量名列表 |
| `tolerance` | number | 是 | 浮点比较容差（通常 `1e-8`） |
| `tags` | array | 否 | 分类标签，便于筛选 |

### `input` 对象字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `data_file` | string | 相对仓库根目录的 CSV 路径 |
| `columns` | array | 需要的列名：`open` / `high` / `low` / `close` / `volume` |
| `chip_data` | bool | 是否需要筹码分布数据（THS WINNER/COST） |
| `period_type` | int | 周期类型：1=日 / 2=周 / 3=月 |
| `period_data` | object | 跨周期数据映射（键为 `WEEK` / `MONTH`） |

### 示例

```json
{
  "id": "macd_tdx",
  "platform": "TDX",
  "source_formula": "DIFF:=EMA(CLOSE,12)-EMA(CLOSE,26);DEA:=EMA(DIFF,9);MACD:=2*(DIFF-DEA);",
  "description": "经典MACD指标 - 通达信版本",
  "input": {
    "data_file": "tests/fixtures/ashare_sh_index_250d.csv",
    "columns": ["close"]
  },
  "expected_output_columns": ["DIFF", "DEA", "MACD"],
  "tolerance": 1e-8,
  "tags": ["trend", "momentum"]
}
```

## 语料覆盖范围

### 通达信（TDX）— 10 条

| 文件 | 指标 |
|------|------|
| `macd_tdx.json` | MACD |
| `kdj_tdx.json` | KDJ |
| `boll_tdx.json` | BOLL |
| `rsi_tdx.json` | RSI |
| `cci_tdx.json` | CCI |
| `bias_tdx.json` | BIAS |
| `wr_tdx.json` | WR |
| `obv_tdx.json` | OBV |
| `dmi_tdx.json` | DMI |
| `expma_tdx.json` | EXPMA |

### 同花顺（THS）— 2 条

| 文件 | 指标 |
|------|------|
| `ma_system_ths.json` | MA5/10/20/60 均线系统 |
| `chip_distribution_ths.json` | 筹码分布（WINNER/COST） |

### 大智慧（DZH）— 2 条

| 文件 | 指标 |
|------|------|
| `pct_change_sort_dzh.json` | 涨跌幅排序选股 |
| `trix_dzh.json` | TRIX |

### 跨平台方言 — 4 条

| 文件 | 说明 |
|------|------|
| `dialect_close_vs_c.json` | `CLOSE` vs `C` 别名 |
| `dialect_close1_ths.json` | `CLOSE1` vs `REF(CLOSE,1)` |
| `dialect_ohlc_aliases.json` | `H/L/C` vs `HIGH/LOW/CLOSE` |
| `cross_period_refdate.json` | `PERIODTYPE` / `REFDATE` |

## 输入数据

语料默认使用 `tests/fixtures/` 下的共享测试数据集。详见 [tests/fixtures/README.md](../fixtures/README.md)。

| 数据集 | 行数 | 用途 |
|--------|------|------|
| `ashare_sh_index_250d.csv` | 250 | 默认 A 股日线语料 |
| `crypto_btc_usdt_1m_1000.csv` | 1000 | 高频场景（可选） |
| `synthetic_waves_500.csv` | 500 | 合成波形（可选） |

## 运行回归（规划中）

语料回归测试将集成到 CI，预期工作流：

```bash
# 运行全部语料回归
cargo test -p finkit formula_corpus -- --nocapture

# 仅运行 TDX 语料
cargo test -p finkit formula_corpus_tdx
```

当前语料作为回归基线定义；测试运行器将在后续 Story 中实现。

## 贡献新语料

1. 在 `tests/formula_corpus/` 下新增 `{id}.json`，遵循上述 JSON 格式。
2. `id` 不得与现有文件重复。
3. `source_formula` 应来自真实平台公式（通达信/同花顺/大智慧导出），或注明来源。
4. `expected_output_columns` 须与公式中 `:=` 赋值或 `:` 输出语句的变量名一致。
5. 优先使用 `tests/fixtures/ashare_sh_index_250d.csv`；需要特殊数据时在 PR 中说明。
6. 提交前确认 JSON 合法：

```bash
python -c "import json, pathlib; [json.load(open(p)) for p in pathlib.Path('tests/formula_corpus').glob('*.json')]"
```

### 命名规范

- 文件名：`{indicator}_{platform}.json`（小写，下划线分隔）
- 平台后缀：`_tdx`、`_ths`、`_dzh`；跨平台方言用 `_cross` 或 `dialect_*` / `cross_period_*`

### 标签建议

| 标签 | 含义 |
|------|------|
| `trend` | 趋势类 |
| `momentum` | 动量类 |
| `oscillator` | 震荡类 |
| `volatility` | 波动率类 |
| `volume` | 成交量类 |
| `selection` | 选股类 |
| `chip` | 筹码分布 |
| `dialect` | 方言/别名 |
| `cross_period` | 跨周期引用 |
| `moving_average` | 移动平均 |

## 相关文档

- [公式文法规范](../../docs/formula/grammar.md)
- [公式系统参考](../../docs/formula.md)
- [公式模板库](../../docs/formula-templates.md)
