# Cross-market screening formulas

Finkit `0.1.5` adds a small, allocation-conscious screening layer for common
selection logic. The primitives use aligned price/volume/benchmark series and
do not embed exchange-specific rules, so the same formulas can be used with
A-shares, Hong Kong stocks, US equities, ETFs, futures, and 24/7 crypto bars.

The functions return an aligned numeric series. Signal values are `1` for a
bullish event, `-1` for a bearish event, and `0` for no event unless noted.
Warm-up values are `NaN` for rolling calculations. Inputs must be ordered from
oldest to newest and have equal lengths when more than one series is used.

## Native API

The Rust indicator functions are exported from `finkit::indicators`:

| Function | Signature | Meaning |
| --- | --- | --- |
| `golden_cross` | `fast, slow` | First bar where `fast` crosses above `slow`; `0/1` |
| `dead_cross` | `fast, slow` | First bar where `fast` crosses below `slow`; `0/1` |
| `breakout_up` | `close, high, period` | Close exceeds the previous `period` highs; `0/1` |
| `breakout_down` | `close, low, period` | Close falls below the previous `period` lows; `0/1` |
| `volume_surge` | `volume, period, multiplier` | Current volume reaches prior average times `multiplier`; `0/1` |
| `ma_alignment` | `close, fast, mid, slow` | EMA alignment: `1` bullish, `-1` bearish, `0` mixed |
| `relative_strength` | `close, benchmark, period` | Excess percentage return versus the benchmark |
| `gap_signal` | `open, close, threshold` | Session gap: `1` up, `-1` down, `0` below threshold |
| `trend_breakout_signal` | `high, low, close, volume, fast, slow, breakout, volume_period, multiplier` | EMA alignment + breakout + volume confirmation |

The period ordering for `ma_alignment` is `fast < mid < slow`. Breakout
windows exclude the current bar, which prevents look-ahead bias. For crypto,
where there is no exchange session close, use the bar open/close gap only when
that interpretation is intended.

## Formula DSL

The same logic is available through the formula engine:

| Canonical function | Common aliases | Arguments |
| --- | --- | --- |
| `GOLDEN_CROSS` | `CROSSUP`, `BULLISH_CROSS` | `fast, slow` |
| `DEAD_CROSS` | `CROSSDOWN`, `BEARISH_CROSS` | `fast, slow` |
| `BREAKOUT` | `BREAKOUT_UP`, `PRICE_BREAKOUT` | `close, high, period` |
| `BREAKDOWN` | `BREAKOUT_DOWN`, `PRICE_BREAKDOWN` | `close, low, period` |
| `VOLUME_SURGE` | `VOLSURGE`, `VOLUME_EXPANSION` | `volume, period, multiplier` |
| `MA_ALIGN` | `MA_ALIGNMENT`, `TREND_ALIGN` | `close, fast, mid, slow` |
| `RELATIVE_STRENGTH` | `RELSTRENGTH`, `RS_EXCESS_RETURN` | `close, benchmark, period` |
| `GAP_SIGNAL` | `GAP` | `open, close, threshold` |
| `TREND_BREAKOUT` | `TREND_SCREEN`, `BREAKOUT_SCREEN` | `high, low, close, volume, fast, slow, breakout, volume_period` |

Examples:

```text
FAST := EMA(CLOSE, 20);
SLOW := EMA(CLOSE, 60);
BUY := GOLDEN_CROSS(FAST, SLOW);
BUY
```

```text
TREND_BREAKOUT(HIGH, LOW, CLOSE, VOLUME, 20, 60, 20, 20)
```

For the complete generated function catalog, see
[generated/formula-functions.md](generated/formula-functions.md). The formula
router validates arity and numeric parameters before entering the optimized
indicator implementation.

## Selection recipes

These are reusable screening building blocks, not investment advice:

```text
MA5 := EMA(CLOSE, 5);
MA20 := EMA(CLOSE, 20);
TREND := MA_ALIGN(CLOSE, 5, 20, 60);
VOLUME_OK := VOLUME_SURGE(VOLUME, 20, 1.5);
BUY := GOLDEN_CROSS(MA5, MA20) AND TREND > 0 AND VOLUME_OK > 0;
BUY
```

```text
RS := RELATIVE_STRENGTH(CLOSE, BENCHMARK_CLOSE, 20);
BREAK := BREAKOUT(CLOSE, HIGH, 20);
RS > 0 AND BREAK > 0
```

For A-shares, apply any exchange limit-up/limit-down or suspension data as a
separate data-quality filter. For HK/US equities, include corporate-action
adjustments in the input series. For crypto, explicitly choose the bar
timezone and benchmark before comparing symbols.

## Validation contract

The screening module tests equal-length validation, warm-up behavior,
look-ahead exclusion, NaN handling, parameter constraints, and market-neutral
gap/relative-strength semantics. Run the focused checks with:

```bash
cargo test -p finkit screening --locked
python scripts/gen_ssot_docs.py --check
```
