# Finkit CLI Guide

The CLI package is `finkit-cli`. The command-line application name exposed by Clap is `finkit`, while source-build examples use the generated `finkit-cli` executable path to avoid ambiguity.

## Build or install

Linux x86_64 can download the published `v0.1.15` Release binary. The current v0.2.0 workspace and all other hosts can build from source:

```bash
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

## Input formats

### Close-only input

Close-only commands accept newline-delimited numbers:

```text
100.0
101.5
102.2
101.8
```

Where supported, stdin can be used instead of `--input`:

```bash
printf '100\n101\n102\n103\n104\n' | ./target/release/finkit-cli sma --period 3
```

### OHLCV CSV

CSV headers are matched case-insensitively. `close` is required. `open`, `high`, and `low` are optional at parser level and become `NaN` when absent; `volume` or `vol` becomes `0.0` when absent. Indicators that semantically require OHLCV still need meaningful columns.

```text
open,high,low,close,volume
100,103,99,102,1200000
102,105,101,104,1300000
```

## Common indicator commands

```bash
./target/release/finkit-cli sma --input close.txt --period 20
./target/release/finkit-cli ema --input close.txt --period 20 --format json
./target/release/finkit-cli rsi --input close.txt --period 14
./target/release/finkit-cli atr --input ohlcv.csv --period 14
./target/release/finkit-cli adx --input ohlcv.csv --period 14
./target/release/finkit-cli cci --input ohlcv.csv --period 20
./target/release/finkit-cli obv --input ohlcv.csv
./target/release/finkit-cli willr --input ohlcv.csv --period 14
./target/release/finkit-cli bbands --input ohlcv.csv --period 20 --stddev 2
./target/release/finkit-cli stoch --input ohlcv.csv --fastk-period 14 --slowk-period 3 --slowd-period 3
```

Use `--format json` or `--format csv` where supported, and `--output <path>` when the command exposes output-file support.

## Formula command

```bash
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli formula --expr "MA(CLOSE,5) + 2*STDDEV(CLOSE,5)" --input ohlcv.csv --format json
```

The CLI also exposes a dialect option. Terminal compatibility is intentionally explicit rather than assuming complete parity with every source terminal. See [formula.md](formula.md) and [generated/pine-compatibility.md](generated/pine-compatibility.md).

## Streaming commands

Streaming commands process bars chronologically and reuse indicator state:

```bash
./target/release/finkit-cli streaming sma --input ohlcv.csv --period 20
./target/release/finkit-cli streaming ema --input ohlcv.csv --period 20
./target/release/finkit-cli streaming macd --input ohlcv.csv --fast-period 12 --slow-period 26 --signal-period 9
```

Streaming subcommands always read an **OHLCV CSV**, including `sma`/`ema`, whose
non-streaming counterparts read a close-only file.

The implemented set is exactly: `sma`, `ema`, `rsi`, `atr`, `adx`, `stoch`,
`macd`, `boll`, `vwap`, `obv`, `supertrend`. Anything else exits with the list
of valid names. `--period` applies to all of them; `--fast-period`,
`--slow-period` and `--signal-period` are used by `macd`; `--nb-dev` is used by
`boll` and `supertrend`.

Do not reorder bars or combine multiple instruments in one stateful stream unless the command/API explicitly supports it.

## Transforms

```bash
./target/release/finkit-cli transform log_return --input close.txt
./target/release/finkit-cli transform pct_change --input close.txt
./target/release/finkit-cli transform zscore --input close.txt --period 20
```

## Feature packs

```bash
./target/release/finkit-cli features alpha_pack --input ohlcv.csv --period 14 --format csv
```

## Parameter sweep

`sweep` runs one indicator across a range of periods and reduces each run to a
single number, which is the quickest way to see how sensitive an indicator is to
its period on a given series.

```bash
./target/release/finkit-cli sweep sma  --input close.txt   --period-min 5 --period-max 50 --period-step 5
./target/release/finkit-cli sweep atr  --input ohlcv.csv   --period-min 5 --period-max 30 --period-step 5 --metric last
./target/release/finkit-cli sweep rsi  --input close.txt   --period-min 7 --period-max 21 --format json
```

| Flag | Meaning |
| --- | --- |
| `<INDICATOR>` | one of `sma`, `ema`, `rsi`, `atr`, `wma` |
| `--period-min` / `--period-max` / `--period-step` | inclusive period range; `--period-step` must be `> 0` and `min <= max` |
| `--metric` | reduction per period: `mean` (default), `std`, `min`, `max`, `last`, `slope` |
| `--format` | `csv` (default), `plain`, `json` |

Input shape follows the indicator, not the command: `atr` reads the **OHLCV
CSV**, the other four read the **close-only** file. Both come from `--input`;
neither reads stdin unless `--input` is omitted.

Use `./target/release/finkit-cli --help` and subcommand `--help` as the executable source of truth for optional flags because command details may evolve faster than prose documentation.

### Input errors

A file that is not the shape the command expects is reported as an error with a
hint, and the process exits `1`:

```console
$ ./target/release/finkit-cli sma --input ohlcv.csv --period 5
error: could not read close input: invalid float literal
hint: close-only commands read one number per line; OHLCV commands read a CSV with a `close` column
```

Close-only commands (`sma`, `ema`, `wma`, `rsi`, `macd`) read one number per
line; OHLCV commands read a CSV with a `close` column. Passing an OHLCV CSV to
a close-only command is the usual cause.

## Schema inspection

The workspace also includes `finkit-schema`, used by CI and tooling to inspect the canonical function/indicator schema:

```bash
cargo run -p finkit-cli --bin finkit-schema -- --help
```

Generated catalogs under `docs/generated/` are derived from the same source-of-truth metadata.

## Warm-up and output handling

Rolling indicators preserve input alignment. Their leading lookback region normally contains `NaN`. When consuming CSV/JSON results:

- preserve row alignment;
- treat leading `NaN` as warm-up, not a signal;
- combine multiple series only after all required outputs are finite;
- validate non-finite values that appear after a series has entered its valid region.

## Troubleshooting

### `command not found`

Use the full executable path after a source build:

```bash
./target/release/finkit-cli --help
```

### CSV command fails

Confirm that `close` exists and that OHLCV-dependent indicators receive the required columns.

### Output begins with NaN

That is normally the expected warm-up region. See [usage.md](usage.md) for alignment examples.

### Need an exact command list

Run:

```bash
./target/release/finkit-cli --help
./target/release/finkit-cli <subcommand> --help
```

Also see [getting-started.md](getting-started.md), [installation.md](installation.md), and [usage.md](usage.md).
