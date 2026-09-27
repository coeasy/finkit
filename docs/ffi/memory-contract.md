# FFI Memory Ownership Contract

This document describes memory ownership for every shipped `extern "C"` export in
`ffi/c-binding/src/` — `lib.rs` (fixed-template entry points), `generated.rs`
(one `ta_*` per indicator) and `research.rs` (factor-study / quant-evaluation
JSON). It is the authoritative reference for C/C++/language-binding authors
integrating Finkit. `scripts/gen_c_header.py --check` enforces that the set of
exports matches `ffi/c-binding/include/*.h`, so this document describes the same
99 symbols the header declares.

> An earlier revision described a `alphata_*` chart API
> (`alphata_kline_data_new`, `alphata_kline_chart_add_ma`, …) and a free function
> `alphata_free_string`. **None of those symbols exist** in any binding. The
> kline-chart surface is implemented for the Java binding and the CLI only; the
> C ABI has never exposed it. That revision also omitted the entire JSON-contract
> surface (`ta_formula_*_contract_json`, `ta_factor_*_json`, …) and the research
> surface, so 18 real exports had no documented ownership. Both are corrected
> below.

## Ownership vocabulary

| Category | Meaning |
|----------|---------|
| **borrowed** | Caller retains ownership. The library reads (or writes into) the pointer only for the duration of the call. Caller must keep the buffer valid and not free/move it until the call returns. |
| **caller-owned** | Caller allocates and owns the resource before and after the call. For output buffers, caller must pre-allocate `len` elements (`f64` or `i32` as documented). |
| **callee-owned** | Library allocates; caller receives ownership and must release with the matching free function. |

## String free functions

There is exactly **one** allocator and therefore one free function per string
family. All of them wrap `CString::into_raw`, so any of the string free
functions can release any of these strings; the specific one to call is listed
for clarity, not because the others would be wrong.

| Returned by | Release with | Notes |
|-------------|--------------|-------|
| `ta_version()` | `finkit_free_string()` | NUL-terminated version string. |
| `ta_last_error()` | `finkit_free_string()` | Per-thread error message snapshot. |
| `ta_operation_catalog_json()` | `finkit_free_string()` | Operation catalog JSON. |
| `ta_factor_catalog_json()` | `finkit_free_string()` | Factor catalog JSON. |
| `ta_operation_execute_json()` | `finkit_free_string()` | Execution result JSON. |
| `ta_factor_execute_json()` | `finkit_free_string()` | Factor result JSON. |
| `ta_factor_cross_sectional_execute_json()` | `finkit_free_string()` | Cross-sectional factor JSON. |
| `ta_factor_stream_execute_json()` | `finkit_free_string()` | Streaming factor JSON. |
| `ta_composite_execute_json()` | `finkit_free_string()` | Composite result JSON. |
| `ta_composite_stream_execute_json()` | `finkit_free_string()` | Streaming composite JSON. |
| `ta_formula_eval_contract_json()` | `finkit_free_string()` | Formula evaluation JSON. |
| `ta_formula_eval_temporal_contract_json()` | `finkit_free_string()` | Temporal-contract JSON. |
| `ta_formula_eval_panel_contract_json()` | `finkit_free_string()` | Panel-contract JSON. |
| `ta_formula_eval_cross_sectional_contract_json()` | `finkit_free_string()` | Cross-sectional contract JSON. |
| `ta_formula_stream_execute_json()` | `finkit_free_string()` | Streaming formula JSON. |
| `ta_formula_compatibility_report_json()` | `finkit_free_string()` | Compatibility report JSON. |
| `finkit_factor_study_json()` | `finkit_factor_study_free_string()` | Factor-research JSON. |
| `finkit_quant_evaluation_json()` | `finkit_factor_study_free_string()` | Quant-evaluation JSON. |

**Never** pass the same pointer to a free function twice. **Never** mix allocators
(for example, do not call the C `free()` on a string returned by `ta_version()`).

## Thread safety

| Area | Guarantee |
|------|-----------|
| **Indicator / pattern `ta_*` calculations** | Safe to call concurrently from multiple threads when each call uses **disjoint** input/output buffers. Functions do not mutate caller-owned arrays beyond writing results. |
| **`ta_last_error()` / `ta_last_error_code()`** | **Thread-local.** Each thread has its own last-error string and code (`thread_local` in `lib.rs`). Safe to call concurrently; no cross-thread visibility. |
| **JSON-contract entry points** | Stateless per call: each call allocates and returns its own string. Safe to call concurrently; the caller owns each returned pointer independently. |
| **Panic isolation** | All exports are wrapped in `catch_unwind`. A Rust panic becomes `FfiStatus::InternalError` (`-4`) instead of aborting the process. Invalid non-null pointers are still undefined behaviour. |

## Return value convention

Most `ta_*` functions return `i32`:

- `0` — success (`TA_OK`)
- negative — error (see [error-codes.md](./error-codes.md))

Functions returning `*mut char` return `NULL` only when allocation fails; an
invalid request is reported as an error JSON document, not as `NULL`.

---

## Ownership matrix — utility & error

| Function | Inputs | Outputs / return | Notes |
|----------|--------|------------------|-------|
| `ta_version` | — | return `char*` **callee-owned** | → `finkit_free_string` |
| `ta_last_error` | — | return `char*` **callee-owned** | Thread-local snapshot → `finkit_free_string` |
| `ta_last_error_code` | — | return `i32` (value) | Thread-local code |
| `finkit_free_string` | `s` **callee-owned** (from library) | — | Frees any string in the table above |
| `finkit_factor_study_free_string` | `s` **callee-owned** (from library) | — | Same allocator; provided so the research surface is self-describing |

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

## Ownership matrix — chart patterns (Finkit-native)

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

## Ownership matrix — JSON contract surface

Every function below takes borrowed NUL-terminated UTF-8 request strings and
returns a **callee-owned** `char*` JSON document. Release it with
`finkit_free_string` (or `finkit_factor_study_free_string` for the research
pair). A `NULL` return means allocation failed only.

| Function | Borrowed inputs | Return | Release with |
|----------|-----------------|--------|--------------|
| `ta_operation_catalog_json` | `language` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_factor_catalog_json` | `language` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_operation_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_factor_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_factor_cross_sectional_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_factor_stream_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_composite_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_composite_stream_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_eval_contract_json` | `source`, `dialect`, OHLCV arrays, `len` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_eval_temporal_contract_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_eval_panel_contract_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_eval_cross_sectional_contract_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_stream_execute_json` | `request_json` | `char*` **callee-owned** | `finkit_free_string` |
| `ta_formula_compatibility_report_json` | `source`, `terminal` | `char*` **callee-owned** | `finkit_free_string` |
| `finkit_factor_study_json` | `request_json` | `char*` **callee-owned** | `finkit_factor_study_free_string` |
| `finkit_quant_evaluation_json` | `request_json` | `char*` **callee-owned** | `finkit_factor_study_free_string` |

The OHLCV arrays passed to `ta_formula_eval_contract_json` are **borrowed** for
the duration of the call; the caller keeps ownership.

---

## K-line visualization

**There is no C ABI for charts.** `ffi/c-binding` exports no `kline_*` symbol and
`ffi/c-binding/include/*.h` declares none, so there is nothing here to document.
Chart rendering is reachable through:

- the CLI (`finkit chart --format svg|html|json`), and
- the Java binding (`com.finkit.KlineChart`, backed by
  `Java_com_finkit_KlineChart_*`), which owns long-lived handles and frees them
  with `klineDataFree` / `klineChartFree`.

A C consumer that needs a chart should render through the CLI and read the file
it writes.

---

## Leak checklist (integration tests)

Use this checklist when validating bindings:

1. After any JSON-contract call, release the returned string exactly once with
   `finkit_free_string` (`finkit_factor_study_free_string` for the research
   pair). This includes `ta_version()` and `ta_last_error()`.
2. Indicator calls: no library allocation visible to caller — only caller
   buffers are used.
3. Repeated `ta_last_error()` without freeing previous strings leaks on the C
   heap.
4. A `NULL` return from a JSON entry point must not be passed to a free
   function; the free functions accept `NULL` as a no-op, but treating `NULL` as
   a document is a caller bug.

---

## Related documents

- [error-codes.md](./error-codes.md) — `FfiStatus` and `ta_last_error_code()` mapping
- `ffi/c-binding/include/finkit.h` — C declarations for the indicator and fixed-template surface
- `ffi/c-binding/include/finkit_research.h` — C declarations for the research surface
