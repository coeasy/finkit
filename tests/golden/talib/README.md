# TA-Lib C Reference Golden Files

JSON reference outputs produced by `scripts/gen_talib_golden.py` using the
official TA-Lib Python bindings (C library). These files anchor AlphaTA's
numerical parity checks against TA-Lib C.

## Regenerating

From the repository root:

```bash
python scripts/gen_talib_golden.py
```

Preview the generation plan (no TA-Lib required):

```bash
python scripts/gen_talib_golden.py --dry-run
```

If fixture CSVs are missing, generate them first:

```bash
python scripts/gen_test_fixtures.py
```

The golden generator can also auto-run `gen_test_fixtures.py` when fixtures are
absent (disable with `--no-auto-fixtures`).

## Prerequisites

Non dry-run mode requires:

1. **TA-Lib C library** (e.g. 0.6.4)
2. **Python packages**: `numpy`, `TA-Lib`

| Platform | C library | Python |
|----------|-----------|--------|
| macOS | `brew install ta-lib` | `pip install numpy TA-Lib` |
| Linux | `sudo apt install libta-lib0-dev` | `pip install numpy TA-Lib` |
| Windows | TA-Lib release → `C:\ta-lib` | `pip install numpy TA-Lib` |

See `packaging/usage/all/bench-vs-talib.md` for detailed setup.

## Input Datasets

All outputs are computed on three shared fixtures from `tests/fixtures/`:

| Dataset ID | File | Type | Rows |
|------------|------|------|------|
| `ashare` | `ashare_sh_index_250d.csv` | A-share daily OHLCV | 250 |
| `crypto` | `crypto_btc_usdt_1m_1000.csv` | Crypto 1-minute OHLCV | 1000 |
| `synthetic` | `synthetic_waves_500.csv` | Synthetic waves + noise | 500 |

## Output Files

One JSON file per indicator (22 total):

| File | TA-Lib function | Key parameters |
|------|-----------------|----------------|
| `sma.json` | SMA | timeperiod=10 |
| `ema.json` | EMA | timeperiod=10 |
| `rsi.json` | RSI | timeperiod=14 |
| `macd.json` | MACD | 12, 26, 9 |
| `bbands.json` | BBANDS | 20, nbdev=2 |
| `atr.json` | ATR | timeperiod=14 |
| `adx.json` | ADX | timeperiod=14 |
| `stoch.json` | STOCH | fastk=14, slowk/d=3 |
| `cci.json` | CCI | timeperiod=14 |
| `willr.json` | WILLR | timeperiod=14 |
| `mom.json` | MOM | timeperiod=10 |
| `roc.json` | ROC | timeperiod=10 |
| `trix.json` | TRIX | timeperiod=14 |
| `obv.json` | OBV | — |
| `ad.json` | AD | — |
| `dema.json` | DEMA | timeperiod=10 |
| `tema.json` | TEMA | timeperiod=10 |
| `wma.json` | WMA | timeperiod=10 |
| `natr.json` | NATR | timeperiod=14 |
| `apo.json` | APO | 12, 26 |
| `cmo.json` | CMO | timeperiod=14 |
| `aroon.json` | AROON | timeperiod=14 |

Parameters align with the legacy CSV golden files in `tests/golden/`.

## JSON Schema

Each file has two top-level keys:

### `metadata`

| Field | Description |
|-------|-------------|
| `generator` | `gen_talib_golden.py` |
| `generator_version` | Script version |
| `generation_date` | ISO date when generated |
| `talib_version` | TA-Lib Python/C binding version |
| `indicator` | Indicator name (e.g. `SMA`) |
| `talib_function` | TA-Lib C function name |
| `parameters` | Parameter dict passed to TA-Lib |
| `inputs` | Required input series (`close`, `high`, …) |
| `outputs` | Output series names |
| `datasets` | List of dataset IDs included |

### `results`

Per-dataset entries keyed by `ashare`, `crypto`, `synthetic`:

```json
{
  "dataset_id": "ashare",
  "fixture_path": "tests/fixtures/ashare_sh_index_250d.csv",
  "fixture_type": "ashare",
  "rows": 250,
  "outputs": {
    "sma": [null, null, …, 3089.12, …]
  }
}
```

- `null` represents NaN (TA-Lib warmup / undefined values).
- Multi-output indicators (MACD, BBANDS, STOCH, AROON) expose several keys
  under `outputs`.

## Usage in Tests

Rust integration tests can load these JSON files to compare AlphaTA outputs
against TA-Lib C at `1e-9` absolute tolerance (see
`core/tests/common/golden_loader.rs`). Python parity scripts such as
`scripts/bench_vs_talib_precision.py` use the same TA-Lib bindings on
independent random input; these golden files use fixed fixture datasets for
regression stability.

## Related

- `tests/fixtures/README.md` — fixture dataset documentation
- `tests/golden/*.csv` — legacy self-golden CSV references
