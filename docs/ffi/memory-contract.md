# FFI Memory Ownership Contract

This document describes memory ownership for every `extern "C"` export in `ffi/c-binding/src/lib.rs`. It is the authoritative reference for C/C++/language-binding authors integrating AlphaTA.

## Ownership vocabulary

| Category | Meaning |
|----------|---------|
| **borrowed** | Caller retains ownership. The library reads (or writes into) the pointer only for the duration of the call. Caller must keep the buffer valid and not free/move it until the call returns. |
| **caller-owned** | Caller allocates and owns the resource before and after the call. For output buffers, caller must pre-allocate `len` elements (`f64` or `i32` as documented). |
| **callee-owned** | Library allocates; caller receives ownership and must release with the matching free function. |

## String and handle free functions

| Returned / created by | Release with | Notes |
|----------------------|--------------|-------|
| `ta_version()` | `alphata_free_string()` | NUL-terminated version string (`CString`). |
| `ta_last_error()` | `alphata_free_string()` | Per-thread error message snapshot. |
| `alphata_kline_chart_to_svg()` | `alphata_free_string()` | SVG document in memory. |
| `alphata_kline_data_new()` | `alphata_kline_data_free()` | Opaque `i64` handle (`0` = failure). |
| `alphata_kline_chart_new()` | `alphata_kline_chart_free()` | Opaque `i64` handle (`0` = failure). |

**Never** pass the same pointer to `alphata_free_string()` twice. **Never** mix free functions (e.g. do not `free()` a string returned by `ta_version()`).

## Thread safety

| Area | Guarantee |
|------|-----------|
| **Indicator / pattern `ta_*` calculations** | Safe to call concurrently from multiple threads when each call uses **disjoint** input/output buffers. Functions do not mutate caller-owned arrays beyond writing results. |
| **`ta_last_error()` / `ta_last_error_code()`** | **Thread-local.** Each thread has its own last-error string and code (`thread_local` in `lib.rs`). Safe to call concurrently; no cross-thread visibility. |
| **Opaque handles (`alphata_kline_*`)** | **Not thread-safe.** A handle must be used from one thread at a time, or guarded by external synchronization. Do not share a chart handle across threads without a lock. |
| **Panic isolation** | All exports are wrapped in `catch_unwind`. A Rust panic becomes `FfiStatus::InternalError` (`-4`) instead of aborting the process. Invalid non-null pointers are still undefined behaviour. |

## Return value convention

Most `ta_*` functions return `i32`:

- `0` — success (`TA_OK`)
- negative — error (see [error-codes.md](./error-codes.md))

Functions returning `*mut char` return `NULL` on failure. Handle constructors return `0` on failure.

---

## Ownership matrix — utility & error

| Function | Inputs | Outputs / return | Notes |
|----------|--------|------------------|-------|
| `ta_version` | — | return `char*` **callee-owned** | → `alphata_free_string` |
| `ta_last_error` | — | return `char*` **callee-owned** | Thread-local snapshot → `alphata_free_string` |
| `ta_last_error_code` | — | return `i32` (value) | Thread-local code |
| `alphata_free_string` | `s` **callee-owned** (from library) | — | Frees strings from table above |

---

## Ownership matrix — moving averages & overlays

All functions below: `*const f64` inputs **borrowed**; `*mut f64` outputs **caller-owned** (length `len`); return `i32` status.

| Function | Borrowed inputs | Caller-owned outputs |
|----------|-----------------|----------------------|
| `ta_sma` | `input` | `output` |
| `ta_ema` | `input` | `output` |
| `ta_wma` | `input` | `output` |
| `ta_dema` | `input` | `output` |
| `ta_tema` | `input` | `output` |
| `ta_kama` | `input` | `output` |
| `ta_mama` | `input` | `mama_out`, `fama_out` |
| `ta_t3` | `input` | `output` |
| `ta_bbands` | `input` | `upper`, `middle`, `lower` |
| `ta_midpoint` | `input` | `output` |
| `ta_midprice` | `high`, `low` | `output` |
| `ta_sar` | `high`, `low` | `output` |

---

## Ownership matrix — momentum & oscillators

| Function | Borrowed inputs | Caller-owned outputs |
|----------|-----------------|----------------------|
| `ta_rsi` | `input` | `output` |
| `ta_macd` | `input` | `macd_out`, `signal_out`, `hist_out` |
| `ta_stoch` | `high`, `low`, `close` | `slowk`, `slowd` |
| `ta_adx` | `high`, `low`, `close` | `output` |
| `ta_aroon` | `high`, `low` | `aroon_up`, `aroon_down` |
| `ta_cci` | `high`, `low`, `close` | `output` |
| `ta_mom` | `input` | `output` |
| `ta_roc` | `input` | `output` |
| `ta_willr` | `high`, `low`, `close` | `output` |
| `ta_apo` | `input` | `output` |
| `ta_bop` | `open`, `high`, `low`, `close` | `output` |
| `ta_cmo` | `input` | `output` |
| `ta_mfi` | `high`, `low`, `close`, `volume` | `output` |
| `ta_trix` | `input` | `output` |
| `ta_vortex` | `high`, `low`, `close` | `vi_plus`, `vi_minus` |
| `ta_vzo` | `close`, `volume` | `output` |
| `ta_volume_momentum` | `volume` | `output` |
| `ta_volume_roc` | `volume` | `output` |
| `ta_chande_forecast` | `close` | `output` |
| `ta_twiggs_mf` | `high`, `low`, `close`, `volume` | `output` |
| `ta_inertia` | `open`, `high`, `low`, `close` | `output` |

---

## Ownership matrix — volatility & volume

| Function | Borrowed inputs | Caller-owned outputs |
|----------|-----------------|----------------------|
| `ta_atr` | `high`, `low`, `close` | `output` |
| `ta_natr` | `high`, `low`, `close` | `output` |
| `ta_trange` | `high`, `low`, `close` | `output` |
| `ta_obv` | `close`, `volume` | `output` |
| `ta_ad` | `high`, `low`, `close`, `volume` | `output` |
| `ta_adosc` | `high`, `low`, `close`, `volume` | `output` |

---

## Ownership matrix — Hilbert transform

| Function | Borrowed inputs | Caller-owned outputs |
|----------|-----------------|----------------------|
| `ta_ht_dcperiod` | `input` | `output` |
| `ta_ht_dcphase` | `input` | `output` |
| `ta_ht_phasor` | `input` | `in_phase`, `quadrature` |
| `ta_ht_sine` | `input` | `sine`, `lead_sine` |
| `ta_ht_trendmode` | `input` | `output` |
| `ta_ht_trendline` | `input` | `output` |

---

## Ownership matrix — statistics & price transforms

| Function | Borrowed inputs | Caller-owned outputs |
|----------|-----------------|----------------------|
| `ta_zscore` | `input` | `output` |
| `ta_beta` | `asset`, `benchmark` | `output` |
| `ta_correlation` | `input_a`, `input_b` | `output` |
| `ta_stddev` | `input` | `output` |
| `ta_tsf` | `input` | `output` |
| `ta_linear_reg` | `input` | `output` |
| `ta_percent_rank` | `input` | `output` |
| `ta_avgprice` | `open`, `high`, `low`, `close` | `output` |
| `ta_medprice` | `high`, `low` | `output` |
| `ta_typprice` | `high`, `low`, `close` | `output` |
| `ta_wclprice` | `high`, `low`, `close` | `output` |

---

## Ownership matrix — candlestick patterns

`*const f64` OHLC inputs **borrowed**; `*mut i32` output **caller-owned** (length `len`); return `i32` status.

| Function | Extra borrowed params |
|----------|----------------------|
| `ta_cdl_doji` | `doji_pct` (scalar) |
| `ta_cdl_dragonfly_doji` | `doji_pct` |
| `ta_cdl_gravestone_doji` | `doji_pct` |
| `ta_cdl_long_legged_doji` | `doji_pct` |
| `ta_cdl_hammer` | — |
| `ta_cdl_inverted_hammer` | — |
| `ta_cdl_hanging_man` | — |
| `ta_cdl_shooting_star` | — |
| `ta_cdl_engulfing` | — |
| `ta_cdl_harami` | — |
| `ta_cdl_morning_star` | — |
| `ta_cdl_evening_star` | — |
| `ta_cdl_three_white_soldiers` | — |
| `ta_cdl_three_black_crows` | — |
| `ta_cdl_marubozu` | `shadow_pct` |

---

## Ownership matrix — chart patterns (AlphaTA-native)

Optional outputs may be `NULL` (skipped). When non-null, caller pre-allocates `len` elements.

| Function | Borrowed inputs | Caller-owned outputs (if non-null) |
|----------|-----------------|-------------------------------------|
| `ta_darvas_box` | `high`, `low`, `close` | `out_top`, `out_bottom` (`f64`), `out_signal` (`i32`) |
| `ta_renko` | `high`, `low` | `out_bricks` (`f64`), `out_dir` (`i32`, optional) |
| `ta_kagi` | `close` | `out_kagi` (`f64`), `out_dir` (`i32`, optional) |
| `ta_point_and_figure` | `high`, `low` | `out_pnf` (`f64`), `out_col`, `out_new` (`i32`, optional) |
| `ta_three_line_break` | `close` | `out_line` (`f64`), `out_dir` (`i32`, optional) |
| `ta_williams_alligator` | `close` | `out_jaw`, `out_teeth`, `out_lips` |
| `ta_heikin_ashi` | `open`, `high`, `low`, `close` | `out_o`, `out_h`, `out_l`, `out_c` (all optional) |

---

## Ownership matrix — K-line visualization

| Function | Inputs | Outputs / return | Ownership |
|----------|--------|------------------|-----------|
| `alphata_kline_data_new` | `dates` **borrowed** (`len` NUL-terminated C strings), `opens`, `highs`, `lows`, `closes`, `volumes` **borrowed** | return `i64` handle **callee-owned** | Library copies all data into the handle |
| `alphata_kline_data_free` | handle **callee-owned** | — | Destroys handle |
| `alphata_kline_data_validate` | handle **borrowed** | return `i32` | Read-only |
| `alphata_kline_chart_new` | `data_handle` **borrowed**, `language`, `title` **borrowed**, `width`, `height` | return `i64` handle **callee-owned** | Clones data from data handle |
| `alphata_kline_chart_free` | handle **callee-owned** | — | Destroys chart |
| `alphata_kline_chart_add_ma` | handle **borrowed** (mutated), `periods` **borrowed** | return `i32` | |
| `alphata_kline_chart_add_macd` | handle **borrowed** (mutated) | return `i32` | |
| `alphata_kline_chart_add_rsi` | handle **borrowed** (mutated) | return `i32` | |
| `alphata_kline_chart_add_boll` | handle **borrowed** (mutated) | return `i32` | |
| `alphata_kline_chart_save_as_svg` | handle **borrowed**, `path` **borrowed** | return `i32` | Writes to caller's filesystem path |
| `alphata_kline_chart_to_svg` | handle **borrowed** | return `char*` **callee-owned** | → `alphata_free_string` |

---

## Leak checklist (integration tests)

Use this checklist when validating bindings:

1. After `ta_version()` / `ta_last_error()` / `alphata_kline_chart_to_svg()`, call `alphata_free_string()` exactly once.
2. After `alphata_kline_data_new()`, call `alphata_kline_data_free()` on success (`handle != 0`).
3. After `alphata_kline_chart_new()`, call `alphata_kline_chart_free()` on success (`handle != 0`).
4. Indicator calls: no library allocation visible to caller — only caller buffers are used.
5. Repeated `ta_last_error()` without freeing previous strings leaks on the C heap.

---

## Related documents

- [error-codes.md](./error-codes.md) — `FfiStatus` and `ta_last_error_code()` mapping
- `ffi/c-binding/include/alphata.h` — C declarations (kept in sync with `lib.rs`)
