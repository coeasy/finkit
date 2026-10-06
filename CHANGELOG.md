# Changelog

## [0.2.0] - 2026-09-23

Trial release. This cycle closes the gap between "the engine can compute it"  
and "a user can find out that it can": the formula surface is now described by  
a machine-checkable contract, and the factor libraries are defined once and  
reused.


### Changed - 2026-10-06 (eighteenth pass — the question is "which recurrence shape", and it is measurable)

The seventeenth pass left one open sentence: the twelve kernels sitting in a 0.82–0.98 band  
"probably share a cause, but nobody knows what it is yet". This pass answers it for the largest  
member of that band, retracts a claim the previous pass made about the report, and turns up a  
structural finding in the `LINREG` family.

#### 1. The probe had to be repaired before it could be believed — again

`core/examples/talib_gap_probe.rs` keeps the previous probe's shape (both sides in one binary,  
interleaved) and fixes the thing that made the previous one produce wrong verdicts: the timed  
region is now a **batch of 16 calls reporting the block mean**, not a single call taking the  
minimum of 100. A single call is 16–120 µs — the same order as `Instant::now()` itself, one  
interrupt, or one allocator slow path — so min-of-100 was measuring the noise floor.

The null controls now run **first and last**, so the reader can see the harness's own drift:

| null control                                       |    opening |    closing | meaning        |
| -------------------------------------------------- | ---------: | ---------: | -------------- |
| `line slope` (same code both sides, no allocation) | **1.008x** | **0.996x** | 1% resolution  |
| `aroon` (same code both sides, two buffers each)   |     0.963x |     0.887x | 8 points apart |

The second row is half the result already: **the same function measured twice, 8 points apart**,  
and the only difference between the two rows is the allocation shape.

#### 2. The `public tier` section systematically understates implementations that allocate more

This is the pass's most important negative result. The one reading that disagrees with Criterion is  
`adx_14`: interleaved, the probe says **0.742x** (ours 104.27 µs, C 77.40 µs); Criterion says  
**1.008x** (`FTA_ADX_14` 74.28 against `TALib_ADX_14` 74.87 µs). Both harnesses build the C side  
**identically** — a `vec![0.0; len]` and one call — so this is not a wrapper difference, and the  
null controls rule out drift.

The difference is the allocation *shape*. Our side made three allocations per call where C makes  
one, and the interleaved rounds force the allocator to service two different free-list patterns  
alternately; Criterion's long run repeats one pattern and stays hot. **So `public tier` ratios are  
only meaningful when both sides can be made allocation-free**, and that limit is now written into  
the probe's module docs rather than only into a report.

**Retraction.** The seventeenth pass recorded that `FTA_AROON_14`/`FTA_ADX_14` were paired against  
the wrong C column in `docs/BENCHMARK_REPORT.md`. They are not. `target/criterion/directional_vs_talib/`  
contains all four of `fta_adx_14`, `talib_adx_14`, `fta_aroon_14`, `talib_aroon_14`, and  
`aroon_14` reads 58.97 / 49.65 µs = 0.842x on the spot, matching the report's 0.84x. There is no  
mispairing; the "finding" was the third artefact of the biased probe.

#### 3. `LINREG_SLOPE`: the cause is the weight orientation, not bounds checks and not the cast

The seventeenth pass proved that bounds checks are **exactly neutral** here (index arithmetic  
against pointer cursors, same seeding, same allocation: 1.000x) and left the cause unknown. This  
pass isolates it:

| comparison                 |      ratio | reading                                                                              |
| -------------------------- | ---------: | ------------------------------------------------------------------------------------ |
| `previous vs cast-hoisted` | **1.000x** | the in-loop `usize as f64` was already hoisted — **not** the cause                   |
| `hoisted vs new-shipped`   | **0.678x** | the other weight orientation is **1.47x faster**                                     |
| `new-shipped vs C`         | **1.026x** | writing that recurrence in Rust **beats the C library** (1.048x on the previous run) |

Not a gap closed — a reference passed. The public tier moves from 0.837x / 0.892x to  
**1.129x / 1.105x** (`line slope` / `line intercept`), and `linear_reg` from 1.714x to **2.038x**.

Weighting the **oldest** bar with `period - 1` makes the per-bar advance

```text
sum_xy += sum_y - period * old          // one add into the accumulator
```

where the other orientation needs

```text
sum_xy += (period - 1) * new - (sum_y - old)   // two dependent subtractions
```

The same number, a different dependency graph and different rounding. `Divisor` is the negative of  
the old `denom` and the numerator's weights reverse with it, so the two sign flips cancel and the  
slope — and therefore the intercept and the emitted value — is unchanged: 469 of 10,000 slots  
differ by more than `1e-12`, **worst case 1.18e-12**, against a golden tolerance of `1e-9` and a  
TA-Lib numeric contract of `1e-8` — three orders of magnitude of headroom. The probe now prints both  
the count and the worst difference, so a count alone cannot be mistaken for agreement.

#### 4. `MAX`/`MIN`: the block tables move from the heap to the stack

The probe's two variants are identical statement for statement; the only difference is  
`vec![0.0; window]` against `[0.0; 64]`. `heap vs stack tables` = **0.870x**. A monotonic index  
ring was tried as a third tier and **rejected** at 0.213x — five times slower — and the rejection  
and its number are recorded in the source so nobody rebuilds it. `van_herk_extreme_into` is now a  
dispatcher over a shared `van_herk_body`, so the two tiers cannot drift.

#### 5. Legacy: the `LINREG` recurrence exists **seven times** in this repository, in two orientations

Five are production paths, two are benchmark-only. The finding that makes this more than  
housekeeping is row 3: **`linreg_angle` already used the correct orientation**, and said so in a  
comment ("Keep TA-Lib's sign and operation order for numerical parity"), with the per-bar update  
`sum_xy += sum_y - p * trailing` and a reseed loop using `let mut weight = (period - 1) as f64;
weight -= 1.0` — character for character the shape this pass wrote into `linreg_slope`. The correct  
shape was already in the repository; the other three production kernels were the outliers. That  
source-level evidence is independent of the probe measurement above and points the same way.

| # | location                                                              | path                               | was                 |
| - | --------------------------------------------------------------------- | ---------------------------------- | ------------------- |
| 1 | `math/linear.rs::linreg_slope`                                        | production (both formula backends) | old                 |
| 2 | `math/linear.rs::linreg_intercept`                                    | production                         | old                 |
| 3 | `math/linear.rs::linreg_angle`                                        | production                         | **already correct** |
| 4 | `math/linear.rs::linreg` (`linear_reg` / `TSF`)                       | production                         | old                 |
| 5 | `math/simd_ops.rs::linreg_slope_avx2` / `_scalar`                     | production                         | old                 |
| 6 | `math/simd_ops.rs::linreg_avx2` / `linreg_scalar`                     | **benchmark-only**                 | old                 |
| 7 | `formula/simd.rs::SimdOps::linear_reg{_slope,_intercept,,_angle,_r2}` | **benchmark-only**                 | old                 |

All seven now share one orientation. **This is where the formula engine gains**: the tree path  
(`fn_linear_reg_slope`) and the plan path (`dispatch_linear_reg_slope_call`) both call  
`math::linear::linreg_slope`, so `LINEARREG`, `LINEARREG_SLOPE`, `LINEARREG_INTERCEPT`,  
`LINEARREG_ANGLE` and `TSF` all inherit it at once. `core/examples/plan_cache_probe.rs` grew  
`LINEARREG_SLOPE(CLOSE,14)` and `TSF(CLOSE,14)` rows so the end-to-end formula cost is on record.

Row 6 is kept rather than deleted, and `simd_linreg` now carries a doc comment explaining why  
production does not use it: it exists to answer "is a vector kernel worth it for `LINREG`", and  
the answer is **no** — only the `period`-element priming scan vectorises (three chunks at the  
default period of 14) while the per-bar recurrence is scalar. Production using the scalar loop is a  
decision, not an oversight, and nothing in the tree said so before.

#### 6. Legacy: the `ADX` recurrence exists three times, and two of them disagree on `NaN`

`adx()` used to route through `compute_adx_family`, which **always** filled three columns  
(`+DI`, `-DI`, `ADX`) for a caller that wants one — three allocations and three write streams per  
call, two of them discarded, where `TA_ADX` allocates one.

- `adx()` now goes through `compute_adx_only` (one column).
- `compute_adx_family` is renamed `compute_di_pair` and loses the `adx` column and the whole ADX  
  smoothing recurrence with it. Its one consumer, `dx()`, wants only the `±DI` pair and  
  deliberately rebuilds `DX` from the two columns to match TA-Lib's rounding.  
  `AdxFamilyResult::adx` consequently became dead code — the `-D warnings` gate surfaced that  
  rather than letting it keep looking complete.
- `di_pair_from_state` is extracted with `di_dx_from_state` layered on it: one source for the  
  three-way guard, and the pair-only path skips a division per bar.

**Deliberately not changed, with the reason recorded**: `compute_adx_only` and the public  
`adx_into` are two hand-copies of the same recurrence that **differ on `NaN` OHLC input**.  
`adx_into` uses a local `true_range_fast` (`if gap > range`, NaN-propagating, matching TA-Lib C's  
`>`); `compute_adx_only` uses `utils::true_range`, whose `f64::max` **ignores NaN**. Both  
`streaming/trend/adx.rs` ("mirror `adx_into` step for step") and `math/kernels/compat.rs`  
(asserted equal by test) treat `adx_into` as canonical, so merging means moving `adx()`/`adxr()`  
toward TA-Lib — a **public numeric change** that needs its own NaN-covering golden test. It gets  
its own pass rather than riding along in this one.

#### 7. A falsified comment, and a corrupted document

- `aroon_into` claimed that per-bar bounds checks "are what §42.8 identified as the remaining gap  
  for the `AROON`/`ADX`/`MAX`/`MIN` family". The seventeenth pass falsified that (pointer cursors:  
  exactly 1.000x). The comment now states what the `zip` loop does and explicitly that the  
  remaining gap is *not* this.
- `docs/talib-efficiency-deep-dive-zh.md` picked up 290 lines of "no content" changes. A  
  `git diff --ignore-all-space` showed only table separators and escaped underscores — an editor  
  markdown formatter, not a gate — and it had **damaged** the file: the `|` in `AVX2 Σ|x - mean|`  
  was read as a table separator and split the cell across four columns, `此` became `&#x6B64;`, and  
  ordered list items `2.`/`3.` were renumbered to `1.`/`2.`. Reverted. There is no prettier config  
  and `.git/hooks` holds only samples, so the source is an IDE plugin.

#### 8. Environment-limited gates (reproduced on clean HEAD, so not caused by these changes)

Two gates cannot run in this sandbox, and both were confirmed to fail identically with the working  
tree stashed:

| gate                                    | symptom                                                                                     | evidence                                                                                                                                                                                                                                                         |
| --------------------------------------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test -p finkit --doc`            | 241 of 254 failed, `Failed to spawn rustc.exe: Os { code: 231 }` (`ERROR_PIPE_BUSY`)        | **the same 241 on clean HEAD**; `--test-threads=2` changes nothing and the run finishes in 3.9 s, i.e. nothing was ever compiled                                                                                                                                 |
| `cargo check --workspace --all-targets` | `pyo3-ffi`'s build script: `failed to run the Python interpreter at python: (os error 231)` | `-j 1` and `PYO3_PYTHON=<real interpreter>` both fail the same way; `python` resolves through the sandbox `safe-bin` shim, which exhausts named pipes when spawning from a build script. Every change here is inside `core/` and touches no pyo3-dependent crate |

Gates that did run and passed: `cargo fmt --all --check`; `RUSTFLAGS="-D unfulfilled_lint_expectations"
cargo clippy`; all 22 `scripts/check_*.py`; and `cargo test -p finkit --release --tests` —  
**62 targets, 3965 passed, 0 failed**.

### Changed - 2026-10-06 (nineteenth pass — AROON rescan unroll closes the 0.83× gap)

Continuation of the eighteenth pass. `aroon_14` (0.83×) and `aroonosc_14` (0.81×) were the  
largest remaining "algorithmic micro-gap" items in `BENCHMARK_REPORT.md`. Both `ExtremeTracker`  
(our side) and TA-Lib `TA_AROON` use the *identical* cached-extreme-index algorithm — a full-window  
rescan fires only when the cached extreme leaves the window — so the structural cause was a  
**micro** one, not a rewrite: TA-Lib's `TA_AROON` rescan runs under `TA_UNROLL(4)` and our  
`rescan_extreme_window` was a plain `for`.

- `core/src/math/statistics.rs::rescan_extreme_window` — four-wide manual unroll of the dominance  
  scan (tail handles `period % 4`) plus `#[inline(always)]` so it folds into `ExtremeTracker::advance`  
  instead of taking a call on the rescan path. The comparison seed (`±INFINITY`), the `>=`/`<=` tie  
  rule, and the NaN-transparent skip are unchanged, so the values it returns are bit-for-bit  
  identical to the old loop (asserted by the existing `test_van_herk_extreme_matches_cached_index`  
  and the aroon streaming/batch convergence tests, 28 targeted tests pass).
- `core/examples/talib_gap_probe.rs` — added section **E** (`aroon_14` / `aroonosc_14` vs C) so the  
  unroll is measured, not argued.

Measured (in-process probe, `BENCHMARK_REPORT.md`'s 0.83× is the pre-change Criterion figure):

| duel          |     ours |        C |                                                                                   ratio |
| ------------- | -------: | -------: | --------------------------------------------------------------------------------------: |
| `aroon_14`    | 52.42 µs | 53.94 µs |                                                                        **1.029× ahead** |
| `aroonosc_14` | 48.08 µs | 44.99 µs | 0.936× (within probe resolution; our side allocates the full `AroonResult`, C one pass) |

So `aroon_14` goes from behind to ahead. The Criterion figure in `BENCHMARK_REPORT.md` still shows  
the old 0.83×; a full `--features talib-c` rerun (pending this commit) will refresh it. The unroll  
also benefits the cached rolling-extreme path (`MAX`/`MIN` for NaN-bearing input), since it shares  
`rescan_extreme_window`.


### Changed - 2026-10-06 (twenty-first pass — the §C public-path allocation shape is real, and two indicators reached parity because of it)

The twentieth pass ended with a claim: the remaining 15 ⚠️ pairs are "public-path allocation  
shape, not kernel gaps". That claim was a hypothesis. This pass converted three of them into  
code, with a warm-baseline A/B protocol that survives the drift the cold protocol suffered.

#### 1. Three public-path changes (kernels untouched)

- `acos`/`asin` (`math_transform.rs`) — dropped the separate serial O(n) domain pre-scan. New  
  `map_finite_checked` helper: one parallel `map_expensive` pass (rayon ≥8192) plus one  
  `simd_first_non_finite` SIMD post-scan. Out-of-domain inputs map to NaN outputs, so scanning  
  the result is equivalent to scanning the domain, and position preservation keeps the  
  "first offending index" error contract bit-identical.
- `ultosc` (public fn in `momentum.rs`) — `init_output` (zeros + SIMD NaN) followed by  
  `ultosc_into`'s own `output.fill(NAN)` was three full-length fill passes before compute;  
  the public fn now hands `_into` an uninitialized buffer and the single `_into` fill remains.  
  The `_into` contract for external callers is unchanged.
- `ad` (`volume.rs`) — `Array1::zeros` was a full memset in front of a kernel that writes every  
  slot (cumulative AD has no warm-up); now `uninit_output`.

#### 2. The measuring protocol had to be fixed first — again

A cold Criterion baseline (first run after a fresh `talib-c` build) made the *unchanged*  
controls (`var`, `max_30`) "improve" 14–15% — pure drift, the same artifact class the  
eighteenth pass documented for allocation shape. The comparison below uses a warm baseline  
(`--save-baseline warm_old` on the stashed tree, then `--baseline warm_old` on the restored  
tree). Under that protocol the controls move ±3%, which is the noise band.

| pair                            | ours net vs same-source C drift | same-session ratio        |
| ------------------------------- | ------------------------------- | ------------------------- |
| `ad`                            | **−22.5%** (far outside band)   | 0.76× → **0.99×**         |
| `acos`                          | **−4.0%** (just outside band)   | 0.95× → **0.995×**        |
| `ultosc`                        | −3.1% (at the band edge)        | 0.82× → 0.85×             |
| `var` (control, untouched)      | +1.6%                           | unchanged ✓               |
| `max_30` (control, untouched)   | −3.0%                           | unchanged ✓               |

`ad`'s −22.5% matches theory exactly: the removed 10k-element zero fill (~80 KB memset) is  
~4 µs against a 12–16 µs function. Correctness: 3054 lib tests, 53 golden parity tests and  
28 formula differential tests pass unchanged.

So "§C allocation shape" was not a variance excuse: two of the 15 ⚠️ reached parity from the  
API layer alone, and the batch report refresh (pending) will move them. Remaining candidates  
in the same family: `adxr` (two public allocations, single-buffer-able via `adxr_into` once  
`compute_adx_only` loses its last caller) and `ultosc_body`'s ring-buffer internals.


### Changed - 2026-10-06 (seventeenth pass — measured, reverted, and one number that did not move)

This pass was asked to (a) stop comparing, (b) actually optimize what is slow, (c) fix the remaining  
legacy defects. The honest summary is that **one** rewrite won, **three** were measured and  
reverted, **two** legacy defects were real, and the single true regression in the report turned out  
to have been mis-attributed by the previous pass.

#### 1. The measurement problem had to be fixed before the optimization could start

`BENCHMARK_REPORT.md` and this pass's own first instrument disagreed about three of four rewrites,  
and the instrument was wrong. Its `*_old` bodies wrote into a **preallocated** buffer while its  
`*_new` side called the **allocating** public function — charging one 80 KB `malloc`/`free` pair to  
the new code. All three micro-rewrites looked like regressions; on re-measurement with matched  
allocation, two were neutral and one was a large win.

The corrected probe (`core/examples/kernel_hotspot_probe.rs`, committed) puts both bodies in one  
binary and runs them interleaved, and carries **null controls** — `AVGDEV` and `LINREG_SLOPE` were  
reverted, so both of their sides are the same algorithm and their ratio is the probe's own noise  
floor:

| null control                             |      ratio | meaning              |
| ---------------------------------------- | ---------: | -------------------- |
| `AVGDEV_14` (same body both sides)       | **0.995x** | best-case resolution |
| `LINREG_SLOPE_14` (same body both sides) | **1.042x** | typical resolution   |

Two more controls came free from the Criterion rerun: `tanh` ran 59.23 / 37.15 / 24.20 µs across  
three independent processes while TA-Lib sat at 45.83 / 45.56 / 45.62 µs. The drift is **one-sided  
and up to 145%**, and it lives entirely in our column. A single-run ratio near 1.0 is a screening  
artifact, not a verdict.

#### 2. `PERCENTRANK` — 4.58x, and bit-identical

`TA_PERCENTRANK` wants a **count**: how many of the `timeperiod` preceding observations sit strictly  
below the current bar. The shipped code maintained a *sorted* window instead — a branchless binary  
search, then a `copy_within` memmove of roughly half the window, then an insert, per bar. The count  
is an integer, so the two forms agree **exactly**; this is a restatement of one predicate, not a  
reassociation of a float sum, and it needs no tolerance to defend.

| body                                            |  10k bars | ns/bar |
| ----------------------------------------------- | --------: | -----: |
| sorted window + binary search + memmove         | 309.40 µs |  30.94 |
| AVX2 direct count (`simd_ops::simd_count_less`) |  67.50 µs |   6.75 |

**4.584x faster**, and the probe asserts `10000/10000` slots bit-identical. In the full paired rerun  
`percentrank_30` went **293.76 → 70.90 µs, 1.08x → 4.64x**. Input containing `NaN` still takes the  
sorted path: that path compares with `partial_cmp().unwrap_or(Ordering::Equal)`, which gives a  
missing bar no defined position, so the two forms are not required to agree there and the shipped  
behaviour is preserved rather than silently redefined.

#### 3. `AROON` / `AROONOSC` — three hand-copied recurrences collapse to one, and get faster

`AROON`, the caller-owned `aroon_into` and `AROONOSC` each carried their own transcription of the  
same cached-index arg-extreme, with three different `NaN` behaviours. There is now one  
`ExtremeTracker`, shared by all three; they differ only in which output they pick out of the same  
state.

The first attempt at this was **slower**, and the probe caught it: factoring the recurrence into a  
per-leg helper and calling it twice means two passes (one over `high`, one over `low`) where the  
original did one fused pass over both. That measured **0.790x** at the kernel level. Restoring the  
single-pass shape:

| comparison                                    | previous |  current |             ratio |
| --------------------------------------------- | -------: | -------: | ----------------: |
| `aroon_14` (allocating)                       | 97.30 µs | 93.10 µs | **1.045x faster** |
| `aroonosc_14`                                 | 55.70 µs | 52.40 µs | **1.063x faster** |
| `aroon_into_14` (kernel level, no allocation) | 51.30 µs | 56.50 µs |     0.908x slower |

Recorded honestly rather than rounded up: the paths the benchmark and the higher layers use got  
faster, the bare `_into` kernel is 9% slower, and 9% sits inside the per-kernel resolution this  
probe demonstrated (0.5%–11.6%). It is **not** claimed as a win. Correctness: `aroon_up`,  
`aroon_down` and `aroonosc` all match at **0 mismatches @1e-12**. The three outputs also stop  
allocating a full-length `NaN` fill — every slot is written by the loop, which is what  
`utils::uninit_output` requires.

In the paired rerun `aroonosc_14` left the ❌ band: **0.71x → 0.85x**.

#### 4. `AVGDEV` — two rewrites built, measured, and reverted

| candidate                                       | result                                         |    |        |
| ----------------------------------------------- | ---------------------------------------------- | -- | ------ |
| ring index instead of `Vec::remove(0)` + `push` | slower                                         |    |        |
| AVX2 \`Σ                                        | x - mean                                       | \` | slower |
| null control for scale                          | 0.995x — so the difference was real, not noise |    |        |

The reason is visible in the sizes: a period-14 window is **13 doubles**, so the `remove(0)` being  
removed is a 104-byte memmove already resident in L1, while the vector form replaces 14 dependent  
scalar adds with three lane-wide adds *plus* a horizontal reduce *plus* a runtime dispatch check.  
There is no headroom in this loop. The straightforward body stays, with the negative result recorded  
in the source so the next reader does not rebuild it.

#### 5. `LINREG_SLOPE` — the pointer-cursor rewrite is exactly neutral, and the §42 attribution was wrong

`linreg_slope_14` is the report's only true ❌ (0.76x, and the same 0.76x in both runs). §42.8  
attributed that to per-bar bounds checks and predicted that closing it "requires rewriting every  
loop into `linreg_slope`'s cursor form".

Measured with identical seeding **and** identical allocation, changing only the loop body:

| loop body         | 10k bars |                           ratio |
| ----------------- | -------: | ------------------------------: |
| indexed (shipped) | 30.40 µs |                               — |
| pointer cursors   | 30.40 µs | **1.000x, 0 mismatches @1e-12** |

Exactly neutral. LLVM already removes those bounds checks, so the attribution does not hold for this  
kernel. The rewrite is therefore **not** landed — not because it is slower (the earlier "0.593x"  
reading was the biased probe from §1), but because it buys nothing. Anyone continuing from §42.8  
should know that its stated hypothesis has now been tested and falsified rather than merely  
doubted.

#### 6. Legacy defect: two `CCI` SIMD paths that existed only in their names

```rust
fn cci_period14_into_impl<const USE_AVX2: bool>(...) {
    let _ = USE_AVX2;   // the parameter is never read
    ...the same unrolled scalar code either way...
}
```

Both arms of the `is_x86_feature_detected!("avx2")` branch produced identical work. A second,  
structurally identical instance sat in `math/simd_kernels.rs`, where `cci_simd_into` branched on  
`has_avx2()` and called the same scalar function on both sides. Readers would reasonably conclude  
the period-14 path was vectorized and stop looking.

The fix is not "add the vectorization" but **delete the parameter and say why it must not be added**:  
vectorizing reassociates `Σ|x - mean|` into lanes, and a window whose deviation approaches zero  
amplifies that difference through the final division. `CCI` is compared against TA-Lib's golden  
series at an *absolute* `1e-8`, where this is measurable. Measurement agrees — the AVX2 form came  
out **18% slower** on the 10k series **and** pushed **13 of 10,000** values past `1e-8`.

A repo-wide sweep confirms this was the only such pattern: `let _ = <ignored const generic>` now  
appears zero times in `core/src`. `fast_moving_avg.rs`'s `USE_FMA` looks similar but is not the same  
defect — its two arms (`mul_add` vs `*` + `+`) genuinely differ, and **both** are instantiated  
(`true` from the `#[target_feature(enable = "fma")]` wrapper, `false` from the software-FMA  
fallback), which the comment there already explains.

#### 7. Legacy defect: one indicator, two answers for a zero-deviation window

`CCI`'s period-14 path wrote `0.0`; its generic path wrote `NaN`. The same indicator returned  
different values depending only on the period it was called with. `TA_CCI` writes `0.0`. Both paths  
now do.

#### 8. Industrial-grade assessment (after the seventeenth pass)

- **Numerical**: unchanged. Every landed change is either bit-identical (`PERCENTRANK`'s integer  
  count, asserted on all 10,000 slots) or verified equal at `1e-12` (`AROON` family), and the two  
  `CCI` fixes move toward TA-Lib's behaviour rather than away from it.
- **Efficiency**: improved at the tail — paired rerun **❌3 → ❌1**, `percentrank_30` **1.08x →  
  4.64x**, `aroonosc_14` **0.71x → 0.85x**, `tanh` **0.77x → 1.68x** (noise, now confirmed).  
  Distribution **74 / 15 / 1** across 90 pairs.
- **Engineering**: three duplicate `AROON` recurrences → one shared type; both fake-SIMD branches  
  deleted; two negative results recorded in the source where they would otherwise be rebuilt.
- **Still open**: `linreg_slope_14` (0.76x, cause now *unknown* rather than assumed) and the  
  0.82–0.98x band of twelve kernels that share one unexplained cause. See  
  `docs/talib-efficiency-deep-dive-zh.md` §9.

#### 9. New document

`docs/talib-efficiency-deep-dive-zh.md` — the efficiency comparison in full: methodology and the  
probe's own noise floor, the 90-pair classification by cause, the per-change evidence, a catalogue  
of the rewrites that were measured and rejected, scale effects at 10k/100k/1M, and the list of what  
is still open.


### Changed - 2026-10-06 (sixteenth pass — "no kernel changed" was a wrong turn)

The fifteenth pass closed on this sentence: *"this pass and the previous one changed no indicator  
kernel under `core/src/math/`, so the ratio movement can only be measurement noise."* That was  
half right. It was right that the `linreg_slope` recurrence sits a few percent from C and resists  
the rewrites that were tried. It was wrong about `floor`, and wrong in a way worth recording: the  
conclusion was derived from *what had been edited*, not from *what was being measured*.

#### 1. `FLOOR` / `CEIL` — `f64::floor` is a function call, and `vroundpd` is 16x faster

`llvm.floor.f64` does not become an instruction on the baseline x86-64 target. It lowers to an  
out-of-line `libm` call, one per element. `FLOOR` and `CEIL` are pure element-wise transforms, so  
that call *is* the indicator. Measured on a 10k-bar series with the shipped kernel  
(`cargo run --release -p finkit --example rounding_kernel_probe`):

| kernel                                    | µs/call |                                             |       |
| ----------------------------------------- | ------: | ------------------------------------------- | ----- |
| \`data.iter().map(                        |     \&x | x.floor()).collect()\` — what the crate did | 24.95 |
| AVX2 `_mm256_floor_pd` (new `simd_floor`) |    1.37 |                                             |       |
| \`data.iter().map(                        |     \&x | x.ceil()).collect()\` — what the crate did  | 26.61 |
| AVX2 `_mm256_ceil_pd` (new `simd_ceil`)   |    1.39 |                                             |       |

AVX2's `vroundpd` performs the identical IEEE operation four lanes at a time and keeps every  
corner case — `NaN` propagates, `±inf` is fixed, `±0` keeps its sign — so this is a bit-identical  
kernel, not an approximation. The probe asserts that equality rather than arguing it, including on  
`NaN`, `±inf`, `±0`, `-0.5` and `1e300`.

The two outputs go through `utils::uninit_output`, so the kernel writes into an uninitialized  
buffer instead of a zero-filled one that is then overwritten — on a 10k-bar series the zero-fill  
alone was about half the cost of the new kernel.

Before this the sixteenth-pass benchmark said `floor` was **0.75x** TA-Lib. The fifteenth pass read  
that as "TA-Lib's sample landed on a fast iteration". It did — TA-Lib's `TA_FLOOR` calls `floor()`  
too, so both sides were paying for a libm call, and the 8%/20% asymmetry between `floor` and `ceil`  
on each side really was layout. But the conclusion "therefore nothing to fix" was only available to  
someone who never asked *why* a one-line element-wise loop costs 25 µs.

#### 2. Input validation was a serial scan, and `WMA`/`EMA`/`DEMA` paid it every call

`math::simd_ops::simd_first_non_finite` is the vector form of "is there a `NaN` or an `±inf`  
anywhere in this series?": four lanes per compare, with a scalar probe only when a lane comes back  
dirty. It replaces three separate serial `iter().position(|v| !v.is_finite())` scans:

| site                                                                                 | cost before                                                          |
| ------------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| `math::moving_avg::reject_if_non_finite` (`EMA`, `WMA`, `DEMA`, `ema_multi_periods`) | a full serial pass on every call                                     |
| `math::fast_moving_avg::reject_if_non_finite`                                        | a full serial pass, then a **second** pass from the warm-up boundary |
| `indicators::math_transform::ln`                                                     | a full serial pass over the *result*                                 |

`WMA` was the visible one at 0.906x; the scan was a full extra pass over the input on a kernel  
whose own work is two passes.

#### 3. One rule, three implementations — the duplicates are gone

The non-finite rule above existed three times with three different shapes. It now exists once, as  
`math::moving_avg::reject_if_non_finite` (`pub(crate)`), and `fast_moving_avg` delegates to it.  
Delegating also fixes its warm-up behaviour: the old copy scanned from index 0, found index 0  
non-finite, then rescanned from the warm-up boundary — two full passes for one answer.

The same consolidation was applied to two other copies:

- `indicators::momentum` carried `calc_di_dx` and `dx_from_state`, two hand-copied versions of the  
  same `+DI`/`-DI`/`DX` three-way guard, one per ADX entry point. Now one `di_dx_from_state`.
- `indicators::math_transform` had `ln` validating its **output** and `log10` validating its  
  **input** — the same domain, the same error contract, two dialects. Both now use `map_positive`,  
  and `log10`'s per-element input loop is gone.

#### 4. Full-length NaN fills the kernel immediately overwrote

Five sites allocated `vec![f64::NAN; len]` (or `Array1::from_elem(len, NaN)`) and then wrote every  
slot from `lookback` on. The fill is not free: it is a full-length store pass over memory the  
kernel discards. `utils::uninit_output` states the "caller writes every slot" contract once, with  
the `unsafe` and its SAFETY note in one place instead of five.

| site                                       | buffers × length | what the fill cost                    |
| ------------------------------------------ | ---------------- | ------------------------------------- |
| `overlap::bbands`                          | 3 × len          | 24 MB on the 1M-bar benchmark         |
| `momentum::compute_adx_family`             | 3 × len          | 240 KB at 10k, 24 MB at 1M            |
| `momentum::compute_adx_only` (ADXR)        | 1 × len          | 8 MB at 1M                            |
| `momentum::aroon_into`                     | 2 × len          | 160 KB at 10k                         |
| `math::moving_avg::sma_inner`'s validation | —                | replaced by the vectorised probe (§2) |

`overlap::bbands` is the one that moved a benchmark: 0.874x at 1M bars.

#### 5. Loop-invariant divisions in the LINREG family

`linearslope` / `linreg_intercept` / `linreg` each divided by a loop-invariant quantity on every  
slot — `denom` for the slope, the window length for the intercept. Both become reciprocal  
multiplies. That is one extra rounding, six orders of magnitude inside this family's 1e-8 golden  
tolerance, and it is exactly what `linear.rs::linreg_slope`'s scalar tail already did — the two  
paths were the only place in the crate where the same kernel divided *and* multiplied for the same  
quantity. `simd_ops::simd_linreg_slope` / `linreg_slope_scalar` / `linreg_scalar` / `linreg_avx2`  
are now consistent with it, and `linreg_avx2` no longer allocates a `Vec<f64>` of lane indices on  
every call.

#### 6. Measured result

Full re-run, 90 paired benchmarks against TA-Lib C 0.8.x, same suite and thresholds as the last two  
passes (`✅ ratio <= 1.0`, `⚠️ <= 1.25`, `❌` beyond):

**✅ 75 / ⚠️ 12 / ❌ 3** (was ✅72 / ⚠️16 / ❌2)

Status transitions, 90 paired benchmarks:

| indicator           | before   | after        |
| ------------------- | -------- | ------------ |
| `floor`             | 0.75x ❌  | **12.64x ✅** |
| `ceil`              | 0.97x ⚠️ | **17.79x ✅** |
| `ln`                | 0.97x ⚠️ | 1.63x ✅      |
| `bbands_20@1000000` | 0.87x ⚠️ | 1.28x ✅      |
| `sma_20@1000000`    | 0.98x ⚠️ | 1.14x ✅      |
| `ad`                | 1.02x ✅  | 0.99x ⚠️     |
| `aroonosc_14`       | 0.84x ⚠️ | 0.71x ❌      |
| `tanh`              | 1.46x ✅  | 0.77x ❌      |

Wall-clock, our side (TA-Lib's own column is unchanged within 1%):

| indicator           | finkit before → after |  wall |
| ------------------- | --------------------- | ----: |
| `ceil`              | 31.19 → **1.54** µs   | 20.3x |
| `floor`             | 33.66 → **2.16** µs   | 15.6x |
| `ln`                | 48.42 → 27.58 µs      | 1.76x |
| `bbands_20@1000000` | 12712.75 → 7485.53 µs | 1.70x |
| `exp`               | 34.97 → 23.77 µs      | 1.47x |
| `sma_20@1000000`    | 3322.32 → 2620.41 µs  | 1.27x |

`exp` and `sma_20@10k` were not edited in this pass; they moved because they sit behind the  
same `map_expensive` / warm-up path. Everything above is in the direction the code change  
predicts, which is the test that separates it from §6.1.

##### 6.1 The two new ❌ do not survive re-measurement — and this box is the reason

The three `❌` are `aroonosc_14`, `linreg_slope_14` and `tanh`. The first and third were `⚠️`  
and `✅` before this pass, and **no kernel they call was edited**. Rather than argue it from  
the diff again — the mistake §42.1 exists to record — the 15 non-✅ indicators were re-measured  
on their own with a longer warm-up and window (3 s / 6 s against the suite's default), which is  
the same code, same flags, quiet machine:

| indicator             | full suite | dedicated re-run | note                                  |
| --------------------- | ---------- | ---------------- | ------------------------------------- |
| `tanh`                | 0.77 ❌     | **0.82 ⚠️**      | also 0.53 in an earlier dedicated run |
| `aroonosc_14`         | 0.71 ❌     | **1.25 ⚠️**      |                                       |
| `linreg_slope_14`     | 0.76 ❌     | **1.30 ❌**       | slower in *both* — the one real ❌     |
| `ultosc_7_14_28`      | 0.80 ⚠️    | 1.22 ⚠️          |                                       |
| `linreg_intercept_14` | 0.83 ⚠️    | 1.19 ⚠️          |                                       |
| `aroon_14`            | 0.88 ⚠️    | 1.14 ⚠️          |                                       |
| `var_20`              | 0.89 ⚠️    | 1.15 ⚠️          |                                       |
| `linreg_angle_14`     | 0.95 ⚠️    | 1.10 ⚠️          |                                       |
| `min_30`              | 0.92 ⚠️    | 1.09 ⚠️          |                                       |
| `max_30`              | 0.95 ⚠️    | 1.06 ⚠️          |                                       |
| `adxr_14`             | 0.94 ⚠️    | 1.06 ⚠️          |                                       |
| `adx_14`              | 0.98 ⚠️    | 1.05 ⚠️          |                                       |
| `wma_20`              | 0.98 ⚠️    | 1.04 ⚠️          |                                       |
| `trima_20`            | 0.98 ⚠️    | 1.01 ⚠️          |                                       |
| `ad`                  | 0.99 ⚠️    | 1.01 ⚠️          |                                       |

Two findings, and they point in opposite directions:

1. **The re-measurement is not a rescue.** In the dedicated run the whole remainder sits at  
   1.0–1.3x rather than 0.7–1.0x, i.e. the honest reading of "how far behind TA-Lib is the  
   `ADX`/`AROON`/`VAR`/`LINREG` group" is 0–30%, not "already at parity". The full-suite  
   ratios for those eleven indicators were flattering, and §8's "still open" list is the  
   correct frame, not the full-suite column.
2. **`tanh` and `aroonosc_14` are noise, and provably so.** Our `tanh` measured 59.23 / 37.15  
   / 24.20 µs on three runs of identical code while TA-Lib's `TANH` measured 45.83 / 45.56 /  
   45.62 µs — a 0.2% spread against our 145%. Criterion's own confidence intervals are tight  
   *within* each run (e.g. `[35.274, 40.395]`), so this is between-process drift, not sampling  
   error, and it is one-sided: our number moves, TA-Lib's does not. `tanh` is therefore  
   faster than TA-Lib in every dedicated run (0.82x and 0.53x) and slower only in the suite  
   run that produced the ❌. `aroonosc_14` moves the same way (0.71 → 1.25).

So the pass's real shape is: five status transitions earned by code, and three that are  
artefacts of measuring 90 benchmarks back-to-back on a shared box. The `❌` that is real is  
`linreg_slope_14`, and it is real in both columns.

#### 7. A red gate that predates this pass: eight broken intra-doc links

Running the *whole* gate matrix rather than only the gates the previous passes happened to  
touch turned up `scripts/check_rustdoc.sh` failing on eight unresolved intra-doc links.  
`git blame` puts all eight on 2026-10-04 (`db78064`, `ca6d3f16`) — two passes before this one —  
so the `doc` CI job has been red since then and nothing noticed.

The pattern is specific: every failure is in a **module-level (`//!`) doc comment**. The very  
same `NodeKind` link written in an ordinary `///` comment one screen away  
(`core/src/semantic_graph.rs:394`) resolves correctly, so the defect is not the link text but the  
scope rustdoc resolves module docs in. Each link is now an explicit crate-rooted path — a bare  
`NodeKind` becomes `crate::semantic_graph::NodeKind`, and so on — which is unambiguous in either  
scope.

`core/src/compute.rs:162` was a different mistake: `[`Self::FixedLookback(0)`]` tries to link a  
*variant instantiation*, which is not an item. It is now a plain code span, matching the  
surrounding style that already writes `` `FixedLookback(_)` `` unlinked.

| file                             | links fixed                                                                       |
| -------------------------------- | --------------------------------------------------------------------------------- |
| `core/src/semantic_graph.rs`     | `NodeKind`, `ArtifactHash`, `SemanticGraph::content_hash`, `SemanticNode::inputs` |
| `core/src/runtime_context.rs`    | `BufferArena`, `StateArena`, `RuntimeContext`                                     |
| `core/src/formula/compute_ir.rs` | `SemanticGraph::content_hash`                                                     |
| `core/src/compute.rs`            | `Self::FixedLookback(0)` demoted to a code span                                   |

Documentation only: no runtime behaviour, no API surface, no measurable cost.

#### 8. Industrial-grade assessment (after the sixteenth pass)

- **Numerical dimension**: unchanged and still industrial-grade. Every edit in this pass either  
  preserves arithmetic bit-for-bit (`floor`/`ceil` are asserted bit-equal to `f64::floor`/`ceil`;  
  the reciprocal changes are one rounding inside a 1e-8 tolerance) or is a pure memory-management  
  change. Validated by the full suite: **3965 tests, 0 failures, 62 targets**, including  
  `golden_talib_tests`' value-by-value fixtures against TA-Lib.
- **Efficiency dimension**: improved, and the *reason* is now written down. Two indicators went  
  from slower-than to 12–18x faster, one from 0.97x to 1.63x, and the mega-scale `BBANDS`/`SMA`  
  gaps closed. The list of indicators slower than TA-Lib C is the shortest it has been, and  
  §6.1 now attaches an honest magnitude to what is left instead of a run-specific one.
- **Engineering dimension**: improved. Three duplicated implementations of two rules were  
  collapsed into one each, and the `unsafe` in the "write every slot" allocation pattern now lives  
  in a single place with its SAFETY argument.
- **Still open**: after §6.1, the reproducible gap is  
  `ADX`/`ADXR`/`AROON`/`AROONOSC`/`ULTOSC`/`MAX`/`MIN`/`VAR`/`LINREG*` at **0–30%** off TA-Lib  
  with their kernels *already* hand-optimized (monotonic queues, van Herk–Gil–Werman block scans,  
  cached indices), with `linreg_slope_14` (1.30x) the only one beyond 1.25x. What is left there is  
  not an algorithmic gap — it is the per-bar bounds checks on `high[i]`/`low[i]`/`out[i]` that  
  have no counterpart in C, and closing it means rewriting each loop around zipped cursors the way  
  `linreg_slope` was. That is a per-loop job with a per-loop risk, so it is scoped as its own pass  
  rather than smuggled into this one.
- **Measurement caveat, now on the record**: on this box, single-run ratios for the sub-1.25x  
  band are not trustworthy to better than about ±20–25%, and the drift is one-sided (our column  
  moves, TA-Lib's does not). Ratios near 1.0 should be read from a dedicated re-run, and the  
  `docs/BENCHMARK_REPORT.md` table should be read as a *screening* artifact, not a verdict.
- **Gate matrix**: green, for the first time in three passes. `cargo fmt --check`;  
  `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked`; both clippy  
  invocations under `RUSTFLAGS="-D unfulfilled_lint_expectations"`; `cargo test -p finkit --tests --no-fail-fast` (**62 targets, 3965 passed, 0 failed, 3 ignored**); the rustdoc policy  
  gate (§7); and 21 `scripts/check_*.py` housekeeping gates. The rustdoc gate is the one that was  
  red before this pass, and §7 is the only place in this changelog where a fix was made to code  
  the pass did not otherwise need to touch.


### Changed - 2026-10-06 (fifteenth pass — one formula source, five answers)

The previous four passes optimized *how fast* each path runs. This one asked a
different question: **does the same source produce the same answer on every
path?** It did not. Four defects, three of them silent, all found by running one
formula through every public entry point instead of profiling one.

`core/examples/backend_divergence_probe.rs` is the probe; every number below is
reproducible with `cargo run -p finkit --example backend_divergence_probe`.

#### 1. Five execution entry points compiled with the wrong pass set

`FormulaOptimizer::optimize` runs statement-level dead-code elimination.
`optimize_for_execution` deliberately omits it, and says why in its own doc
comment: an `X:=...` assignment is observable through
`FormulaContext::variables`, so statement-level DCE "is only valid for explicit
lazy/optimizer use, not for the normal compiled execution contract".

Five *execution* entry points called `optimize` anyway:

| Entry point | Was | Now |
|---|---|---|
| `FormulaCompiler::compile` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::compile_bytecode` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::eval_optimized` | `optimize` | `optimize_for_execution` |
| `FormulaEngine::eval_jit` | `optimize` *on top of* an already-optimized AST | dropped |
| `FormulaEngine::compile_jit` | `optimize` *on top of* an already-optimized AST | dropped |

The two JIT entry points were worse than "the wrong pass set": they compiled
`self.compile_shared(source)?.ast` — an AST that had *already* been optimized for
execution — and then ran a second optimizer over it that included DCE. The
second pass is gone; the first one is what they needed.

Two things went wrong, and the second is the serious one:

- **Side effects.** `JUNK:=MA(CLOSE,5); OUT: CLOSE;` published `["JUNK","OUT"]`
  through `eval` and only `["OUT"]` through `eval_optimized` and
  `FormulaCompiler`. `ctx.variables` is part of the documented contract (the
  language bindings build their result dictionaries from it), so dropping the
  assignment is not an optimization, it is a different result.
- **The returned numbers.** A dropped statement also drops the string literals
  it would have appended to `FormulaContext::string_table`, and a literal
  evaluates to *its index in that table*. So dropping an earlier statement
  shifts every later literal. Measured, before the fix:

  | source | `eval` | `eval_optimized` | `FormulaCompiler` |
  |---|---:|---:|---:|
  | `TMP:='HELLO'; OUT: 'WORLD';` | **1** | **0** | **0** |

  A wrong side table would be a bug; a wrong primary result is a wrong number.

The four `optimize` call sites that *should* keep DCE are untouched
(`FormulaOptimizer::optimize` is still public, and `core/benches/formula_bench.rs`
still uses it). What changed is that no path which *executes* uses it.

#### 2. `FormulaCompiler` deep-cloned on the miss path as well

The fourteenth pass removed the cache-hit clone from both caches. This one
removes the miss-path clone from `FormulaCompiler::compile`: it used to build
the formula, `insert(source, formula.clone())` and return the original — a deep
copy of the AST that existed only to satisfy the cache's owned storage. The
cache stores `Arc` now, so the miss path inserts the very handle it returns.
`compile` keeps its `-> CompiledFormula` signature, so one copy is still
inherent (the caller asked to own it); there is just no longer a second one.

#### 3. The plan backend could not run a string literal after a stateful statement

`FormulaComputeLowerer::add_effect` appends the *previous* effect to a node's
dependency list purely to serialize stateful statements. `STRING_LITERAL` was
lowered through `add_effect`, so a literal following an assignment picked up
that control edge — and `HotExecutionPlan` turns **every** semantic dependency
into a buffer input. The `STRING_LITERAL` kernel rejects any input with its own
arity check, so the plan backend failed outright:

```
FormulaEngine::new().with_execution_mode(Plan)
    .eval("TMP:='HELLO'; OUT: 'WORLD';", &mut ctx)
=> RuntimeError("... kernel dispatch failed for kernel 0x43f39d3d91f40744 with code 2")
   while the tree backend returned 1
```

The literal is now lowered with `add_node` — no control edge. That is sound
because the node reads nothing and is insensitive to *where* it runs: its value
is the index the plan baked into the parameter arena, and the caller
pre-populates `string_table` from `string_literals_ordered()`, which is keyed by
node id (lowering order), not execution order. `effect: Stateful` is kept on
purpose, because CSE must never merge two identical literals — a merge would
change the literal count and break those baked indices with
`LiteralBindingMismatch`.

The existing `formula_string_literals.rs` gate missed this because all four of
its cases are *function calls* with a literal argument (`EM_REF("IDX", 1)`); the
chain only appears when a literal follows a stateful statement.

#### 4. The bytecode VM ignored assignment shadowing of a builtin alias

`A` is an official alias for `AMOUNT` — `builtin_data_aliases_resolve_to_the_same_series_on_every_path`
asserts exactly that, on every path. But `BytecodeCompiler::normalize_variable`
rewrote reads of `A` to `AMOUNT` unconditionally, so a formula that *assigned*
`A` had its own variable replaced by the alias:

| path | `A:=CLOSE*2; B:=A+CLOSE; OUT: B;` |
|---|---|
| `eval` (tree, default) | 43.05 |
| plan | 43.05 |
| `FormulaCompiler` | 43.05 |
| bytecode VM | `RuntimeError("Data not available: AMOUNT")` |

`A` is also the single most common user variable in TDX formulas, so this was
not a corner. The compiler now tracks the names the formula binds
(`X:=...`, `X: ...`, `for X = ...`) and resolves a bound name to the formula's
own variable; unbound names still fall through to the alias table exactly as
before. Declarations are recorded *after* the value is compiled, so
`A := A + 1` with no prior `A` still reads the alias on the right-hand side, as
the tree path does.

#### 5. And a gap in the gate itself

`run_plan` in `formula_differential_tests.rs` called `executor.execute`, which
derives the series length from `inputs.first()` and therefore rejects a plan
with no bound inputs. Every constant-only formula was untestable on the plan
path — the plan backend's own `eval_plan_channels` handles that case with
`ctx.data_len`, so the *gate* was narrower than production, not the engine. The
helper now mirrors production and calls `execute_range` with the same length
rule.

#### The new gate

`every_execution_entry_point_agrees_on_assignments_and_string_table` runs a
corpus of assignment-shaped formulas through `eval`, `eval_optimized`,
bytecode, JIT, `FormulaCompiler` and the plan path, and asserts **both** the
returned values *and* `ctx.variables` / `ctx.string_table`. Every other
differential gate in this repository compares values only, which is precisely
why defects 1 and 4 survived four rounds of differential testing.

#### Re-measured: 90 paired benchmarks, ✅ 72 / ⚠️ 16 / ❌ 2

The full `talib_c_comparison` suite was re-run on the final tree. Nothing in
this pass or the previous one touched an indicator kernel — neither round edited
anything under `core/src/math/` — so any movement in the ratios is measurement
noise, and the two numbers below are the evidence for saying so rather than
asserting it:

| Row | Finkit (µs) | TA-Lib C (µs) | Ratio |
|---|---:|---:|---:|
| `floor` | 33.66 | 25.23 | 0.75x ❌ |
| `ceil` | 31.19 | 30.32 | 0.97x ⚠️ |
| `linreg_slope_14` | 32.99 | 24.74 | 0.75x ❌ |
| `linreg_intercept_14` | 30.44 | 25.14 | 0.83x ⚠️ |

`floor` and `ceil` are the same trivial elementwise loop. On our side they
measure 33.66 and 31.19 µs (8% apart, both stable); on the C side they measure
25.23 and 30.32 µs — **20% apart from each other**. The 0.75x is TA-Lib's fast
sample landing on `floor`, not a change on our side.

The LINREG pair is the known limitation this repository has recorded since the
thirteenth pass: the loop-carried `sum_xy` recurrence puts a latency floor under
`linreg_slope`, two candidate fixes were measured as no-ops (31.03 → 31.10 and
31.11 µs), and our side still measures 30-33 µs — exactly where it was. The
previous run reported the same row as 0.75x against a 23.1 µs C sample.

So the honest summary of this run is: **the set of indicators slower than
TA-Lib C did not change; two rows crossed the report's 1.25x warning threshold
because the reference measurement moved.** The distribution is 72/16/2 here
against 75/14/1 on the previous run, with the difference confined to those two
families.

### Changed - 2026-10-05 (fourteenth pass — both compiled-formula caches were charging rent on every call)

The previous passes audited the numeric kernels. This one audited the **formula
engine's own per-call overhead** and found a cost with nothing to do with
mathematics: **both** compiled-formula caches deep-copied their entry on every
hit. `FormulaPlanCache` cloned the whole `FormulaHotPlan`; `FormulaCache` — the
one the **default** tree-walker uses — cloned the whole `AstNode`. Neither
clone bought anything: every caller only ever borrows the compiled artifact.

Measured directly, on the pre-fix code, at a 250-bar series (the screening
window), the cache hit alone cost this much inside `compile_plan`:

| Formula | cache hit (µs) | share of the whole call |
|---|---:|---:|
| `MACD-ish` = `EMA(CLOSE,12)-EMA(CLOSE,26)` | 2.55 | 38.9% |
| `RSI(CLOSE,14)` | 1.38 | 41.1% |
| `(CLOSE-MA(CLOSE,20))/STD(CLOSE,20)*100` | 3.02 | 33.8% |
| `MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); OUT: MA5-MA20;` | 4.29 | 47.5% |

End-to-end, `cargo run --release -p finkit --example plan_cache_probe`, minimum
of 5 rounds, baseline and fixed measured minutes apart on the same machine
(µs per call, 250 bars):

| Formula | plan path | tree path (default) |
|---|---:|---:|
| `MACD-ish` | 6.55 → **4.13** (1.59x) | 2.82 → **2.36** (1.19x) |
| `RSI` | 3.37 → **1.97** (1.71x) | 0.94 → **0.73** (1.29x) |
| `BOLL` | 8.95 → **5.79** (1.55x) | 3.58 → **3.06** (1.17x) |
| `COMPOUND` | 9.02 → **4.80** (1.88x) | 4.14 → **3.29** (1.26x) |

- `FormulaPlanCache` now stores `Arc<FormulaHotPlan>` and hands out the handle;
  a hit is one refcount bump.
- `FormulaCache` now stores `Arc<CompiledFormula>` behind an additive
  `get_shared`/`insert_shared`. Its public methods keep their exact signatures —
  `get` still yields `&CompiledFormula`, `get_cloned`/`remove` still yield owned
  values — so this is an internal change with no API break.
- Each cache gains one private resolution point (`plan_for` /
  `compile_shared`) that the engine's own evaluation paths use. The **miss** path
  is clone-free too: it stores the very handle it returns. It used to build the
  artifact, clone it into the cache and hand back the original.
- `compile_plan` / `compile` keep their public owned-return signatures, so a
  caller that genuinely needs ownership still gets it and pays the one clone
  that ownership costs. Exactly one call site still needs that: the shared-batch
  helper moves each AST into a synthetic `Output` node, and it is marked as such.
- `Arc`, not `Rc`, so `FormulaEngine` stays `Send` exactly as before.

**What the measurements actually support.** The removed cost is *fixed per
call*, so it dominates short series and shrinks proportionally with length. At
2 000 bars the plan path gained 1.12-1.36x and the tree path 1.02-1.19x; at
10 000 bars the apparent gains (1.06-1.19x) are **not** trustworthy, because the
baseline process was uniformly slower — visible in the change-independent
`cold(compile+eval)` column, where the same "unaffected" measurement read
1.9-2.6 ms against 1.7-2.1 ms after. Inter-run drift of that size is the reason
the probe reports a minimum of 5 rounds rather than a mean, and the reason the
250-bar rows — where the effect is 20-90%, far outside that drift — are the ones
quoted as the result.

**The other thing the probe shows, and it is not flattering:** the compiled-plan
backend is **slower than the tree-walker it was built to replace, at every
length measured** — 4.13 vs 2.36 µs (250 bars), 18.07 vs 11.16 (2 000),
79.92 vs 58.10 (10 000). The cause is structural and was already visible in the
thirteenth pass: every node in a plan materialises a full-length buffer, so
`EMA(CLOSE,12)-EMA(CLOSE,26)` costs ~58-80 µs at 10 000 bars for two
recurrences. The tree-walker, which was supposed to be the slow reference path,
does not have that problem. Making the plan path actually win needs a fusion or
a buffer-lifetime pass over the plan — see
`docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` §40.3. Until then the
default staying `Tree` is the correct setting, not a backlog item.

Also left alone, stated plainly: the output materialisation copies a series
twice for repeated readers (`read_slot` clones into `already_read` and out
again; index 0 returns `primary.clone()`). Several names sharing one retained
buffer is the documented normal case (`DIF`/`DEA`/`MACD`), so the copies are
real — but removing the second one means handing back shared buffers, which
changes `FormulaPlanOutput`'s public shape.

`core/examples/plan_cache_probe.rs` is the probe, kept as an example so every
number above is reproducible rather than asserted.


### Changed - 2026-10-05 (thirteenth pass — last sub-1.0 TA-Lib gaps, one orphan kernel, one systemic codegen defect)

Three rounds, every finding in a round fixed before the next started. Two
questions drove the round: **are the remaining deficits algorithmic or
constant-factor**, and **is there anything else in the tree that looks alive but
is not**. The first answer is constant-factor throughout. The second turned up
three things: an orphan SIMD kernel, a "deleted" file that was still tracked, and
— found by auditing the tree rather than by profiling — a codegen defect that was
silently taxing most of the hot loops in the crate.

Measured against TA-Lib C on the paired Criterion suite (10k-bar OHLCV series);
the whole suite was re-run each time, so both columns come from comparable
conditions:

| Indicator | before | after | TA-Lib C (after) | speedup before → after |
|---|---:|---:|---:|---:|
| `STOCHRSI_14_14_3_3` | 267.3 µs | 101.7 µs | 116.8 µs | 0.45x → **1.15x** |
| `STOCHF_14_3` | 134.6 µs | 76.9 µs | 82.9 µs | 0.59x → **1.08x** |
| `PERCENTRANK_30` | 498.8 µs | 295.4 µs | 318.1 µs | 0.74x → **1.08x** |
| `ACOS` | 143.5 µs | 41.4 µs | 131.6 µs | 0.96x → **3.18x** |
| `MACD_12_26_9` | 134.5 µs | 63.0 µs | 142.8 µs | 1.06x → **2.27x** |
| `SAR` | 71.8 µs | 45.0 µs | 58.5 µs | 0.82x → **1.30x** |
| `PPO_12_26` | 54.1 µs | 19.1 µs | 46.7 µs | 0.86x → **2.45x** |
| `RSI_14` | 22.95 µs | 22.28 µs | 30.00 µs | 1.29x → **1.35x** |
| `LN` | 64.4 µs | 47.2 µs | 44.5 µs | 0.75x → 0.94x |
| `MIN_30` / `MAX_30` | 20.5 / 19.1 µs | 15.5 / 15.4 µs | 14.3 / 14.6 µs | 0.77x / 0.80x → 0.92x / 0.95x |
| `LINREG_INTERCEPT_14` | 34.8 µs | 30.3 µs | 25.2 µs | 0.74x → 0.83x |
| `LINREG_SLOPE_14` | 34.5 µs | 30.9 µs | 23.1 µs | 0.69x → 0.75x |
| `AROONOSC_14` | 59.1 µs | 53.5 µs | 44.5 µs | 0.76x → 0.83x |

Suite totals over the 90 paired benchmarks: **❌ 8 → 2 → 1, ⚠️ 15 → 17 → 14,
✅ 67 → 71 → 75** (each arrow is one of the three rounds). The 1M-bar scaled
rows moved with the same change: `MACD@1M` 0.94x → **1.64x**, `RSI@1M` 1.06x → 1.08x.

**The systemic one: `f64::mul_add` outside an FMA target feature.** `mul_add`
lowers to LLVM's `llvm.fma.f64`. On this crate's baseline target — x86-64
*without* `+fma`, which is what Rust defaults to and what CI builds — LLVM cannot
emit the instruction, so it emits a **call to libm's `fma()`**. A 20M-iteration
loop-carried recurrence measures the cost directly: `(v - x) * k + x` runs
2.6 ns/bar, `(v - x).mul_add(k, x)` runs 3.7 ns/bar — about 1.1 ns (~3.4 cycles)
lost per call, per bar.

Audited across `core/src`: **105 `mul_add` sites, 87 of them outside any
`#[target_feature]` function**, i.e. paying that call. The hot ones are the EMA
recurrences (MACD, MACDFIX, PPO, TRIX, KDJ, streaming MACD/MACDFIX), RSI's
`avg_gain`/`avg_loss`, the SAR update, `variance`/`stddev` sum-of-squares, and
the VIDYA/CMO/FISHER/STC cores. 74 of those were rewritten to the explicit
`*` + `+` form — the same expression with one extra rounding, and the form
TA-Lib itself produces, since TA-Lib is compiled without FMA. The sites that
really are inside (or reachable from) a `#[target_feature(enable = "fma")]`
function were left alone: there `mul_add` is one hardware instruction and is the
faster form.

**`PPO` (0.79x → 2.45x).** `ppo_with_ma_type` composed two whole `ma()` calls:
two full-length allocations, two full-length store passes, then a third pass to
divide — while the structurally identical `APO` was already fused at 2.59x. The
SMA and EMA selectors (the two that are actually called: `ppo` is the formula
shorthand `PPO:=(EMA(CLOSE,SHORT)-EMA(CLOSE,LONG))/EMA(CLOSE,LONG)*100`, and
TA-Lib's default profile is `matype=0`/SMA) are now fused into one pass that
never materialises either moving average. Both kernels are asserted
**bit-identical** to the `ma()` composition they replaced
(`ppo_fused_kernels_are_bit_identical_to_the_ma_composition`), because PPO feeds
golden TA-Lib parity for `matype=0` and `matype=1` and "close enough" is not an
acceptable bar for a recurrence.

The first attempt at that fusion made PPO *slower* (58.4 → 87.7 µs): the fused
EMA recurrence used `mul_add` in a plain function, i.e. two libm calls per bar.
It now runs inside `#[target_feature(enable = "fma")]` with runtime detection,
mirroring what `ema_inner` already does.

**`SAR`'s `USE_FMA` knob was a false promise — removed.** `sar_default_into_impl`
carried `const USE_FMA: bool` plus a `#[target_feature(enable = "fma")]` twin:
the `true` instantiation got real hardware FMA, the `false` one the split
multiply/add, chosen by a runtime feature check. Both halves failed here. The
`false` instantiation was effectively unreachable on x86, and its split form is
what every *other* SAR path (streaming `SarState`, general `sar_into`) uses — so
the fused path returned different last bits, which is exactly what
`incremental_state_matches_batch_exactly` and
`single_output_matches_with_af_projection` assert against (they did fail, which
is how this was caught). And the speedup was not there: on the paired 10k suite
the fused default kernel ran 71.8 µs and the split form 45.0 µs on the same
input. The knob was buying a cross-path inconsistency for a regression; every SAR
path now uses the split form. (KAMA keeps its FMA dispatch — there the fused form
really is faster, 24.7 vs 30.0 µs — and no test asserts cross-path bit-equality
for it.)

**`LINREG_*` main loops: index arithmetic → zipped cursors.** `input[i]`,
`input[i - period]` and `output[i]` each carried a bounds check, while TA-Lib's C
loop — same dependency chain, same one division per bar, *and* an extra `fabs`
pair for its drift guard — was faster. The three cursors are now zipped, so they
are in bounds by construction. Measured effect: none (31.03 → 31.11 µs). Kept
because it removes work rather than adding it, but recorded honestly as a no-op,
not a win.

Fixes, with the defect that caused each:

- **`stochf`** — the two monotonic `Vec<usize>` queues advanced a logical head
  and never compacted, so they grew to the full series length every call
  (10k bars → ~160 KiB and ~10 reallocations, each with a `memcpy`). That is the
  whole explanation for the odd pair "`stoch` 1.15x but `stochf` 0.59x" — same
  shape, different kernel. Now uses the cached-index arg-extreme kernel `stoch`
  uses: one comparison per leg per bar, no allocation.
- **`stochf` with `fastd_period == 0`** panicked: `vec![0.0; 0]` was indexed and
  `d_start = fastk_start + 0 - 1` underflowed. Now rejected as
  `InvalidParameter`.
- **`stochrsi`** — the public entry built four full-length buffers and ran four
  passes, while the zero-copy `stochrsi_into` already existed and nobody called
  it. It now delegates: RSI parks in the %D buffer, %K is rewritten in place,
  and one fused ring pass emits %K/%D.
- **`percent_rank`** — three branchy `partition_point` calls per bar plus
  `Vec::remove` + `Vec::insert` (two memmoves). Now: one branchless binary
  search, two per bar instead of three (the insert position follows from
  `count_less - (evicted < current)`, so the third search is gone), and a single
  `copy_within` span move.
- **`rescan_extreme_window`** (shared by AROONOSC / AROON / ULTOSC / STOCH /
  STOCHF / STOCHRSI) — evaluated `!is_nan() && (!found || cmp)` per candidate.
  Seeding with `NEG_INFINITY`/`INFINITY` folds the NaN skip into the comparison
  itself. `>=`/`<=` are deliberately kept: ties must resolve to the *newest*
  bar because AROONOSC reports the **index**, not just the value.
- **`ln`** — validated and `push`ed in one loop, paying a capacity check and a
  branch inside the transcendental loop. Since `ln` maps every invalid input to
  a non-finite value, the domain check moved onto the result: one branchless
  `map` plus a vectorizable scan. Error contract unchanged (first offending bar
  still rejects the call).
- **A full-length NaN fill pass removed** from `min`, `max`, `linreg_slope` and
  `linreg_intercept`: `fill_rolling_extreme` now guarantees it writes every slot
  from `window - 1` on (filling the leading-missing-run gap itself), so the
  buffer only needs its warm-up prefix seeded.

Orphan logic found and removed:

- `math::simd_ops::{simd_linreg_slope, simd_linreg, simd_linreg_angle}` had no
  production caller at all — the only reference in the tree was
  `benches/simd_statistics_bench.rs`, which is why `dead_code` and
  `check_orphan_modules.py` both stayed silent. They are now wired into
  `linear::linreg_slope` for the `warm_start == 0` case (those kernels have no
  leading-warmup concept, so forwarding unconditionally would change results).
  Worth recording honestly: they only vectorize the O(period) seeding — the
  O(len) recurrence is scalar in both paths, so the win is small; the point of
  wiring them is that the promise `math::simd_ops` makes should be true.
- `math::sar::sar_default_into_impl`'s `const USE_FMA: bool` — see above. After
  the `mul_add` audit the two arms had become the same code, then restoring the
  fused arm broke two cross-path bit-exactness tests, which is what proved the
  knob had no remaining reason to exist.

Repository state vs. documentation:

- `visualization/frontend/index.html.template` — the twelfth pass CHANGELOG said
  this was deleted, but `git ls-files` still listed it and it was still on disk.
  Deleted for real this time (history keeps it: `git log --diff-filter=D --`
  `visualization/frontend/index.html.template`). A deletion is not done until the
  index says so.

Not fixed, stated plainly: `LINREG_SLOPE` is now the **only** ❌ left (0.75x,
30.9 µs vs 23.1 µs) and `LINREG_INTERCEPT` sits at 0.83x. The bottleneck is the
`sum_xy` recurrence — `sum_xy[i]` depends on `sum_y[i-1]`, a loop-carried chain
that sets an ~8-cycle floor per bar and that no amount of local cleanup removes.
Two candidate fixes were measured this round and both were **no-ops**, which is
the useful part of the result:

- hoisting `1.0 / denom` so the per-bar slope is a multiply instead of a
  division: 31.03 → 31.10 µs. The loop is latency-bound on the recurrence, not
  throughput-bound on the divider, so the `divsd` was already hidden.
- replacing the per-bar bounds checks with zipped cursors: 31.03 → 31.11 µs.

TA-Lib's own `TA_LINEARREG_SLOPE` runs the same recurrence, carries one division
per bar *and* an extra `fabs` pair for its drift guard, and is still faster — so
the remaining gap is not the recurrence's shape. Breaking it needs a prefix-sum
or a genuinely vectorized reformulation, which trades numerical drift for speed
and deserves its own evaluation rather than being folded into this pass.

Also unchanged and worth naming: `KAMA`'s fused (hardware-FMA) form is genuinely
faster than the split form (24.7 vs 30.0 µs), so unlike SAR it keeps its runtime
FMA dispatch — which means its last bits depend on the host CPU. That is a
deliberate trade-off, not an oversight, and it is now documented at the call
site.

### Changed - 2026-10-05 (twelfth pass — connectivity audit, orphan-asset cleanup, CI coverage holes)

Audited "is every flow actually connected" with reproducible scans instead of
reading: the documentation reference graph, orphan-asset detection, feature-gated
`#[test]` inventory and loop-termination proofs. Three rounds; every finding in a
round was fixed before the next one started.

Frontend/backend: `docs/api-reference.md` and `api-reference-zh.md` had no
description at all of the web chart payload — the one public contract between
Rust and a browser frontend. Both gained a section covering the payload fields,
the `schema_version = 1` requirement, the "missing values are JSON `null`, never
`NaN`" rule, and the `createFinkitLightweightChart` entry point.

Deleted:

- `visualization/frontend/index.html.template` — referenced nothing and nothing
  referenced it; its `{{LOCALE_JS_PATH}}`/`{{CONFIG_JS_PATH}}` placeholders point
  at files that do not exist. Superseded by the Lightweight Charts adapter.
- `docs/archive/` (22 documents) and
  `docs/finkit-vs-talib-expanded-benchmark-results.md` — the archive's own index
  marked every entry "Superseded"; several describe modules that no longer exist
  (`runtime_engine.rs`), paths that were frozen (`bytecode`/`JIT`) and counts that
  moved on (399 formula functions; the catalogue is at 452). Git history keeps
  every file: `git log --diff-filter=D -- docs/archive/` recovers one by name.
  Dangling references in `docs/README.md`, two plan documents and two gate
  docstrings were repaired with the deletion.

Repository hygiene: six tracked build artifacts (7.3 MB) sat in `visualization/`
because both `.gitignore` and `check_no_tracked_build_artifacts.py` anchored the
"renderable output" rule to the repository **root**, so the gate reported
"none look like build output" while shipping them. The files are untracked, the
ignore patterns are no longer root-anchored, and the gate now derives example
output names from `*/examples/*.rs` so it matches at any depth.

CI coverage holes closed:

- `node --test visualization/frontend/lightweight-charts-adapter.test.mjs` — the
  adapter is `include_str!`-embedded into every generated chart, and its suite had
  never run.
- `cargo test -p finkit-visualization --features html` — with default features
  every generated-document test was compiled out. A new
  `rendered_html_wires_the_adapter_to_a_parseable_payload` test now parses the
  inlined payload out of the rendered HTML and asserts the adapter, the import,
  `schema_version`, the candle count and the indicator line actually survive
  rendering; that seam was previously untested end to end.
- `cargo test -p finkit --features rayon --lib -- batch::` — four parallel-batch
  tests (order preservation, error propagation, serial/parallel equivalence) were
  compiled out on every run.

`scripts/benchmark_talib_all_current_gate.py` could never fail: its entry point
called `run(...)` and discarded the summary, so `errors` and `parity_failures`
never affected the exit code. It now returns 2 like its sibling. Both it and
`benchmark_talib_full_current_gate.py` were orphaned — only documents that have
since been deleted mentioned them — and are now wired into
`talib-release-gate.yml`, which also closes a real gap: the 61 candlestick
functions and the math/operator/statistics families they add on top of the PR-28
surface had no gate at all.

Also fixed three broken relative links (`docs/src/quickstart.md`,
`PINE_COMPAT_MATRIX.md`, `migration/pine-to-AlphaTA.md`) left behind by renames.

Verified clean: no orphan formula functions (all 295 registered), no unbounded
loops, no nested lock acquisition, `sync_bindings.py --check --all` reports
`drift=none` for Python and Node.

### Changed - 2026-10-04 (eleventh pass — benchmark hygiene, extrema block scan, formula-layer O(n·w) sweep)

Full bench-vs-talib snapshot in `docs/BENCHMARK_REPORT.md`; the round is recorded
in V4 plan §36. Every before/after number below is a pair from **one** run (or a
targeted re-run) — absolute timings on this machine drift 1.3-1.6x between runs.

- **The benchmark itself was wrong before it was slow.** `criterion` 0.5 pulls
  `plotters` in by default and its gnuplot backend sprayed `Gnuplot not found`
  over the tenth-pass logs. `default-features = false` (Cargo.lock -30 lines);
  a clean run now finishes EXIT=0 with 0 banners and 185 paired groups. No
  algorithm changed, but the tenth-pass numbers were measured under that noise.

- **`MAX_30` / `MIN_30`: the slowness was algorithmic, not constant-factor.**
  The cached-index kernel pays a full-window rescan whenever the cached extreme
  leaves the window, and on the benchmark's noisy series that is nearly every
  bar — so amortized O(1)/bar degenerated to O(w)/bar. Both now dispatch to a
  Van Herk-Gil-Werman block scan (suffix/prefix tables, ~3 comparisons per bar,
  **independent of the data distribution**) when the window is <= 512 and every
  element is finite. `ULTOSC` and the 1M `BBANDS` row improved with it (the
  latter was a dirty sample, not a regression).

- **Hot-path modulo removal.** `stochf`, `stochrsi_into`, and the 7/14/28
  `ultosc` path each did one to three `%` per bar — a division, ~20 cycles.
  All replaced with wrap counters (the positions advance by exactly one per
  emitted bar, so `d_idx` still starts at 0).

- **Memory passes.** `sma_inner` did five passes over the series (validate,
  validate, warm-up scan, full NaN fill, kernel) — now one fused scan plus a
  warm-up-prefix-only fill; `rsi_inner` lost its full-length `init_output` fill
  (all three RSI kernels write their own warm-up NaNs). This is where the 1M
  `SMA` row was losing, not in the sliding sum.

- **An optimization that lost, recorded rather than hidden.** A fused Van
  Herk-Gil-Werman scan for `AROONOSC` (both legs, value+index tables, single
  pass, no allocation) measured **slower** than the cached-index kernel it was
  meant to replace: ~10ns vs ~7.5ns per bar across two targeted re-runs, and a
  two-kernel variant measured 0.63x vs the cached 0.72x. The eight dynamically
  indexed tables cost more than the rescans they remove. The fast path, the
  kernel, and its tests were deleted; `aroonosc` keeps the cached kernel and
  carries the measurement in a comment so nobody re-attempts it blind.

- **Formula layer: the remaining per-bar rescans.** `LONGCROSS` (sliding
  violation count), the 12 "m-th most recent pivot" walks across
  `TROUGH`/`PEAK`/`TROUGHBARS`/`PEAKBARS` and the `EM_`/`FOX_` families (deque
  capped at the last m pivots), `ts_argmax`/`ts_argmin` (monotonic deque),
  `BACKSET` (O(len*n) -> O(len) backwards countdown — it fills *forwards*, so a
  forward countdown would be wrong), `LAST` (O(len*(A-B)) -> O(1)/bar fixed-width
  sliding count), and `SUMBARS` (O(n^2) -> O(n log n) prefix table + binary
  search for non-negative input; signed or missing values keep the rescan
  because monotonicity does not hold there).

- **Six real defects found by the three audit rounds** (all with differential
  tests against the naive implementation):
  1. `sma_inner`'s fused scan used `is_nan()` to find the series start, but
     `leading_warmup` skips **every** non-finite bar — a leading `inf` used to
     be warm-up and was being rejected as bad input. Now `is_finite()`.
  2. Same trap in the extrema dispatch: the guard was `!is_nan()`, while this
     family's `first_finite` skips only `NaN` (a leading `inf` is a real value
     here). Tightened to `is_finite()`. The two layers define "warm-up"
     differently on purpose; the mismatch is now documented in both places.
  3. `MODE` picked its winner with `max_by_key` over a `HashMap`, and Rust
     randomizes `HashMap` iteration order per process — **tied counts returned
     different values on different runs**. Winner is now "highest count, then
     earliest occurrence in the window"; the per-bar `Vec` allocation is gone.
  4. `BACKSET`, `LAST`, `SUMBARS`: quadratic/linear-in-window costs above.
  5. `fill_mth_recent_pivot` now `debug_assert`s that pivots arrive in ascending
     index order (the zig-zag scan guarantees it; a future caller that sorts
     differently fails loudly instead of silently shifting answers).
  6. `aroonosc`'s fused kernel was deleted as dead weight once it lost (above).

- **Deliberately not changed (recorded):** `fn_cmo`, and
  `covariance`/`correlation`/`decay_linear` in `formula/ops/timeseries.rs` — all
  three could be made sliding, but each changes floating-point accumulation
  order for a public API with no benchmark coverage justifying the drift.

- **Verification:** 3047 lib tests (up from 3037, +7 new differential/contract
  tests, -2 deleted with the aroonosc kernel), integration targets green, 22
  `scripts/check_*.py` gates + `gen_ssot_docs --check` pass, `cargo fmt`,
  `cargo check` 0 warnings.

### Changed - 2026-10-04 (tenth pass — TA-Lib C head-to-head with item-by-item chase, formula-layer audit)

Full bench-vs-talib snapshot in `docs/BENCHMARK_REPORT.md`; the round is
recorded in V4 plan §35.

- **TA-Lib C 0.8.1 head-to-head ran on this machine for the first time**
  (90 paired benchmarks). First snapshot: 59 pairs faster than TA-Lib, 13 pairs
  ❌. Fixed a real bug in the report gate on the way: `bench_report.py` matched
  `FTA_`/`TALib_` case-sensitively while Criterion lowercases ids on disk, and
  swapped the bench/scale path components, so pairing always found 0 rows.

- **Chase list, item by item (all numbers 10K bars, before → after vs TA-Lib):**
  - `WILLR_14` 83.5 → **32.1 µs** (0.42x ❌ → **1.07x ✅**): the Van Herk
    block-scan kernel `willr14_into` existed but was never wired into the
    public entry — `willr_into` now dispatches to it for `period == 14` on
    NaN-free input.
  - `STOCH_14_3_3` 150.4 → **84.9 µs** (0.65x ❌ → **1.11x ✅**): the fast-K
    extrema maintenance in `stoch_monotonic_fast_into` moved from a 128-slot
    monotonic ring to cached-index tracking (one comparison per leg per bar,
    rescan only on expiry; tie and expiry rules preserved).
  - `AROONOSC_14` 170.1 → **59.4 µs** (0.26x ❌ → 0.74x ⚠️): rewritten from a
    `VecDeque` to the same cached-index structure `aroon_with_deques` uses.
  - `MAX_30` 29.1 → 22.3 µs, `MIN_30` 37.0 → 32.1 µs: the cached kernel was
    rewritten branch-lean — `NaN` fails every comparison so the explicit
    missing-bar test is gone, expiry is only evaluated after the dominance
    test failed, and the warm phase is a separate loop so the per-bar emit
    gate disappeared. TA-Lib's contract-free loop is still ~1.5x faster;
    root cause recorded.
  - `ln` 75.0 µs: validation and computation fused into one pass (same
    all-or-nothing error contract).

  ⚠️ **Behaviour change:** STOCH on NaN input no longer freezes fast-K at
  50.0 — a missing bar is now dropped by the extrema (contract B), matching
  the rest of the extrema family.

- **Formula layer: 15+ per-window rescans replaced with O(1)/bar algorithms.**
  `HHVBARS`/`LLVBARS`/`MAXINDEX`/`MININDEX` share a new `ArgExtremeDeque`
  (monotonic deque over indices, ties keep the earliest bar); `EVERY`/`EXIST`/
  `COUNT` became sliding true-counts (EVERY counts predicate *failures* so a
  missing bar still counts as true, bit-for-bit as before); `VWMA`/`MFI`/
  `TOTALVOL` use sliding sums with in-window missing tracking; `ICHIMOKU_*`
  and the `DONCHIAN` family route through the shared extrema kernels.

- **Potential defects found and fixed by the audit:**
  - Parser stack overflow (an abort no FFI guard catches): `parse_formula`
    now rejects parenthesis nesting deeper than 256 before pest runs.
  - `ICHIMOKU_TENKAN(x, y, 0)` panicked on `usize` underflow (`(n - 1)`
    bypassed the `extract_n` guard); the kernel handles `window == 0`
    idempotently instead.
  - `AROONOSC` indexed an empty deque on an all-missing window (panic); it
    now reports `NaN`.
  - `AROONOSC` subtracted `usize` indices that could go negative; cast to
    `f64` first.
  - `DONCHIAN_UPPER`/`LOWER` reported `±inf` for an all-missing window;
    now `NaN`, consistent with the family.

- **Assessed and deliberately not wired (recorded, not hidden):**
  `simd_ops::simd_linreg_slope` has zero external callers and its AVX2 variant
  only vectorizes the initial window sum (3 chunks at period 14), so wiring it
  buys nothing; `SimdOps::hhv/llv` (block-scan SIMD) likewise has no internal
  callers but is public SIMD API surface.

- **Loop/recursive audit:** all `loop` sites in the executor are bounded by
  `MAX_LOOP_ITERATIONS`, the stateful FOR by a construction-time rejection
  plus a backstop, the sandbox caps recursion depth, and the parser now caps
  nesting — every recursive chain is bounded.

### Changed - 2026-10-04 (ninth pass — rolling extrema on the cached fast path, NaN transparency, orphan fold collapse)

- **`MAX` / `MIN` and the whole extrema family now use the cached-index fast
  path.** `MAX`, `MIN` and every derived indicator that reads them
  (`MIDPOINT`, `MIDPRICE`, `WILLR`, `STOCH`, `AROON`) shared one rolling-extrema
  kernel that still rescaned the window for every bar — O(n·w) for a period-30
  window, while the same kernel already had a cached-index path that only
  `MIDPOINT`-style callers reached. The dispatch gate no longer needs a
  `has_missing` branch, so the branch that used to be unreachable at exactly
  `EXTREMA_CACHE_LIMIT = 512` is gone.

- **NaN transparency, with the warm-up contract written down for the first
  time.** Missing bars no longer poison a window: a `NaN` takes its slot in the
  window but never enters the monotonic queue, so `[2, 3, NaN]` reports `3`
  instead of `NaN` (before this round a `NaN` could never be dominated, and one
  missing bar turned *every later window* into `NaN`). The warm-up anchor is
  now documented as "the window must hold `period` bars, and a missing bar does
  not fill it" — pandas' `rolling(window=p, min_periods=p)`. The decisive
  evidence is the repository's own differential gate, which expects 188 finite
  values out of 200 for `HHV(X, 9)` where `X = MA(CLOSE, 5)`: only
  "skip the leading missing run first" yields 188, while "report from
  `period - 1` and drop NaNs inside" yields 192.

  ⚠️ **Behaviour change, three of them:**
  1. A leading missing run delays the first report. `HHV(_, 3)` on
     `[NaN, NaN, 10, 8, 12, 6, 14]` is now `[NaN, NaN, NaN, NaN, 12, 12, 14]`
     (was `[…, 10, 10, 12, 12, 14]`).
  2. A missing bar inside a window is dropped, not propagated (see above).
  3. A fully missing window returns `None` (streaming) / `NaN` (batch) instead
     of re-emitting the previous window's extreme.

- **Streaming `MAX` / `MIN` now match the batch kernels on NaN input.** A
  `started` flag skips the leading missing run, and `count` is redefined as
  "bars since the first finite bar", so the first report lands at exactly
  `first_finite + period - 1` on both paths. A missing bar still advances the
  bar index and still expires, which is what keeps `HHV(MA(CLOSE, 2), 3)`
  from diverging between the two paths.

- **Two orphan O(n·w) fold implementations deleted.** `talib_ext::rolling_minmax`
  (the upstream of `KDJ` and `SMI`) and `features::normalization::rolling_minmax`
  (the feature normaliser) each carried their own `fold`-based rolling
  min/max; all three callers now share `rolling_minmax_visit`. Side effect worth
  knowing: when the current bar is itself missing the normaliser now yields
  `NaN` instead of `0.5`, while a degenerate window (equal bounds) still yields
  `0.5` as before.

- **New gate `core/tests/extrema_cached_path.rs` (10 tests)** pins both
  strategies (cached index ≤512, monotonic ring >512) against a naive oracle
  across the cache boundary — periods 1/2/3/5/14/30/64/128/200/255/256/511/
  512/513/1024 — plus NaN-no-poison, leading-run delay, fully-missing windows,
  per-leg dropna in the fused high/low kernel, earliest-of-equals index, and
  long-series index drift.

- **`TEST_INDEX.md` was missing three real test targets**, which is what
  `test_index_contract` (a deliberately two-way gate) exposed this round:
  `extrema_cached_path`, `formula_draw_parity` and `formula_string_literals`
  existed on disk but were not discoverable from the index.

- **Docs:** V4 plan §34 records the NaN contract, the two wrong turns that
  took to derive it, and the environment failures that are not regressions;
  stale `target/t_round9*.log` scratch logs removed.

**Verification (2026-10-04, rustc 1.98.1):** `cargo test -p finkit --lib`
3036 passed / 0 failed · `extrema_cached_path` 10/10 · `formula_differential_tests`
52/52 · all integration targets green · `cargo fmt --all --check` clean ·
`clippy` introduces no new lint in the changed range (the 15 hits there are all
pre-existing workspace pedantic noise). Two failures are environmental and were
identical before this round, so neither is a regression: `cargo test --doc`
(240/240 fail with `ERROR_PIPE_BUSY` — rustdoc cannot spawn `rustc` in this
sandbox), and the `pyo3-ffi` build script (no usable interpreter discovery on
this machine; every other crate is 0 errors).

### Changed - 2026-10-04 (eighth pass — efficiency, dead benches, workspace hygiene)

- **Bytecode VM: the per-execution whole-context clone removed.** `BytecodeVM::
  execute` used to `ctx.clone()` — a deep copy of all five OHLCV arrays, often
  100s of KB — on *every* run, even though the only field the VM ever mutates
  is the (typically empty) string table, and the clone was discarded at the
  end. The data is now borrowed and only the string table is cloned, with the
  scratch table carried out in `ExecResult::string_table` exactly as before.
  Measured on the same machine (criterion change detection vs the stored
  baseline, 10k bars): MA_20 bytecode 49.0 → 34.5 µs (−34%, now *faster* than
  the AST interpreter), EMA_12 50.0 → 34.1 µs (−36%), RSI_14 58.0 → 42.2 µs
  (−31%); MACD is compute-dominated and unchanged. The bytecode VM's own 53
  tests and the full formula corpus (3029 lib + 52 tree==plan differential +
  17 runtime-convergence) stay green.
- **Three criterion benches that never measured anything, fixed.**
  `accuracy_test`, `formula_performance_bench` and `performance_benchmark` sat
  in `core/benches/` without `harness = false` entries, so `cargo bench`
  wrapped them in the libtest harness and they reported "0 tests" on every
  run — compiled, shipped, silent. The entries now exist; all three run
  (`accuracy_test`'s cross-indicator report prints all ✓).
- **Workspace hygiene: ~188 MB of stale, gitignored artifacts removed.** A
  225 KB leftover test log (`CargoLock`-era `.core_test_tail.log`), a stale
  `core/test_output.txt` build log, a 179 MB stale `target-verify/` build
  cache, two stale `ffi/c-binding/build-usage*` CMake directories, stale
  `dist/*.log` build logs, and the six chart outputs (`gpu_large_chart.html`
  et al.) that CI regenerates fresh on every WebGL job. `scripts/archive/` is
  deliberately kept: it is the recorded audit trail and the orphan-script
  gate already accounts for it.
- **V4 plan bookkeeping made self-consistent**: §31's conformance table still
  said Batch 3 item 6 was "未做" and kernel fusion "未实现" after §32 recorded
  both as executed; the table now points at §32 while keeping the pre-round
  text as audit trail. The remaining genuinely-open items in the plan corpus
  are user-decision gates (behaviour changes reserved for explicit
  confirmation: `dzh` silent mapping, `IF` truth value / `ema_scalar` seed,
  and the release-level default-path flip to `Plan`).

### Changed - 2026-10-04 (seventh pass — the plan's remaining items, executed)

The instruction this pass answered was: *finish the two items the plan still
records as open, and keep measuring before claiming.*

- **Batch 3 item 6 closed — the formula and factor frontends now lower through
  the semantic graph.** `FormulaLowerer` pushes every node into a
  `SemanticGraphBuilder` (push order = the old direct-assembly id order, so the
  graph is node-for-node isomorphic with what `ComputePlan::compile` saw before);
  `FormulaComputePlan::graph()` publishes the graph, so content-hash, CSE
  eligibility and scheduling levels are reachable for a formula without
  re-lowering. `factor-analysis`'s `ResearchPlan::compile` builds its graph the
  same way (`NodeKind::Factor`), with a stable Kahn pre-pass that preserves the
  old path's acceptance of non-topological stage declarations and a `node_stage`
  remap that keeps the public API speaking stage ids. No execution behaviour
  changed: the graph's validator *is* `ComputePlan::compile`, and the
  tree==plan differential corpus (empty allowlists) stays green.
- **Batch 4 item 6 (kernel fusion) executed to a data-backed verdict: built,
  gated, measured, rejected.** A complete elementwise-chain fusion
  (`FUSED:BINARY_CHAIN`, postfix program in the parameter arena, register
  machine in the dispatcher) passed all four of its own gates — and then lost
  to the unfused plan at **every** measured chain length on 1M bars (3-op:
  12.2ms vs 10.5ms; 7-op: 24.9 vs 20.4; 11-op with `%`: 60.0 vs 38.2; 27-op:
  108 vs 60.8). The per-op cost of a scalar register interpreter is about 2x
  the per-pass cost of the plain elementwise kernels, which LLVM auto-vectorizes.
  The code was removed rather than shipped slow or left dead: no orphan logic,
  no unmeasured kernel. The winning successor — compiling fused chains into
  SIMD kernels — sits outside the frozen JIT/`eval_simd` boundary and is
  recorded as future work in V4 §32.2.
- **Formula execution vs TA-Lib, refreshed.** The recorded cross-library
  snapshot (TA-Lib 0.6.8 Python binding, worst numeric divergence 4.7e-10,
  verdict `PARITY`) has finkit faster on all 14 indicators at `1.03x`–`2.53x`
  (geometric mean ~1.6x); the TA-Lib C 0.8.1 Criterion gate stays in CI where
  the pinned library is installed — this host has no `ta-lib-static`, and no
  local C-level numbers were fabricated. Fresh local formula-engine numbers are
  in V4 §32.3.

### Fixed - 2026-10-04 (sixth audit pass — plan conformance and documentation governance)

A sixth pass that asked two questions the earlier rounds never opened: *how much
of the V4 plan is actually implemented?* and *do the documents contradict each
other about what the plan is?*

- **Documentation governance: four documents each claimed to be the current
  baseline.** `docs/refactor-plan-2026-09-21.md` said "本文档是唯一执行基线";
  `docs/archive/README.md` said only that document is an execution baseline;
  `docs/development.md` said the current architecture baseline was
  `architecture-and-feature-audit-2026-09-26.md` with
  `refactor-plan-2026-09-26.md` as the active optimization route; and
  `docs/README.md` listed three different "current" documents at once. A reader
  got a different answer depending on which file they opened.
  `docs/README.md` and `docs/archive/README.md` now state that
  `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md` is the **single**
  execution baseline; `refactor-plan-2026-09-21.md` is re-labelled a *constraint
  source* (it holds the user-confirmed product boundary and the frozen-JIT
  decisions) rather than a baseline; `docs/development.md` points at V4.
- **Archived the superseded 2026-09-26 plan, its companion audit and the
  `upgrade-completion-matrix-zh.md` progress snapshot.** The matrix still listed
  `bytecode`/`JIT` as shipped paths after both were frozen, so it was publishing
  invalid capability claims. All three documents had no internal links, so the
  moves break nothing; the archive index gained three rows and each document
  gained an "archived" banner naming V4 as the current baseline.
- **`core/src/formula/engine.rs` justified the default execution mode with a gap
  that no longer exists.** The `FormulaExecutionMode::Tree` doc comment led with
  "Re-measured 2026-09-23: 12 red targets / 143 failing tests / 76 missing
  kernels" as the reason the default has not been flipped to `Plan`. That gap was
  closed on 2026-09-24 — `unified_dispatch.rs` routes every TA-Lib 0.7/0.8
  `CALL:<NAME>` kernel through `dispatch_modern_call`, and both allowlists in
  `core/tests/formula_plan_differential.rs` are empty (a stale entry fails that
  gate, so "empty" is an assertion, not a default). The comment now records the
  closure and leads with the *actual* remaining blocker, which is structural
  rather than a missing kernel: a formula containing a string literal cannot run
  on the plan path at all, because the plan executor receives no string table to
  append to. Documentation-only change — the default is still `Tree`.
- **`docs/api-reference.md` contradicted itself, left over from the previous
  round's own fix.** It still said "All **eight** public language surfaces share
  the same control-plane JSON contracts", but the list it introduces was reduced
  to seven when the non-existent C++ binding was removed, and the very next
  paragraph says iOS and Android do **not** expose that control plane. The C and
  Node bullets had also been left indented six spaces, breaking the list. Both
  corrected.

### Fixed - 2026-10-04

A fifth audit pass that re-asked the previous rounds' own questions against what
they had already shipped, plus two dimensions those rounds never opened: the
wasm frontend's CI coverage and the consumer-facing API reference.

- **Frontend build-coverage gap in `multilang-release.yml` and
  `multilang-cross-platform.yml`.** Both workflows narrow `pull_request.paths`
  to a hand-maintained list but omitted `visualization/**` and
  `factor-analysis/**` — directories that six crates depend on, including the
  `wasm` browser frontend (which depends on `visualization`). The `wasm32`
  build only runs in `multilang-release.yml`, so a breaking change to
  `visualization` or `factor-analysis` passed CI on the PR and only failed at
  release time. `cli/**` was likewise missing from `multilang-release.yml` and
  `python-wheels.yml`. Added the missing directories to all three path filters.
- **New gate `scripts/check_workflow_path_coverage.py`.** For every
  `pull_request.paths`-filtered workflow it extracts the crates actually built
  (`cargo ... -p <crate>`), computes the transitive local `path = "..."`
  dependency closure, and fails if any member directory is absent from the paths
  filter. Wired into `ci.yml`. Verified by injection: dropping `visualization/**`
  from a fixed workflow flips it to failure, then restores to green — it cannot
  stay green while a gap exists.
- **API reference named bindings that do not exist.** `docs/api-reference.md`
  listed `C++: finkit::operation_execute_json` as one of eight language
  surfaces, but the repository ships no C++ binding — `ffi/` contains eight
  language bindings (c, python, node, go, dotnet, ios, java, android) and no
  C++. iOS and Android instead expose per-indicator FFI (`alpha_ta_*` and
  `Java_com_finkit_indicators_Finkit_*Native`) and do not participate in the
  unified JSON control plane. Removed the C++ entry and documented the iOS /
  Android surfaces. Also corrected `FormulaEvalContractJSON` (a casing that
  exists nowhere in the tree) to `formulaEvalContractJson`. Both verified
  against the source.
- **Stray 205 KB log at the repository root** (`CargoLock_test_tail.log`, dated
  2026-08-30, referenced by nothing) removed.

Termination review (the "no infinite loops" requirement): all nine `loop { }`
sites carry a `SAFETY-TERMINATION` argument or an `iterations >=
MAX_LOOP_ITERATIONS` backstop; the formula executor's WHILE/FOR loops bail at
`MAX_LOOP_ITERATIONS = 10_000`, so a malicious or accidental formula cannot hang
the engine. `scripts/check_unbounded_loops.py` passes (9/9).

Verified: 22/22 script gates pass (including the new one); SSOT `--check` passes;
all four edited workflows parse as YAML; the cargo build of every non-pyo3 crate
finishes cleanly. (Full `cargo check --workspace --all-targets` was not re-run
here because the sandbox's pipe quota exhausts while pyo3-ffi's build script
spawns the Python interpreter — an environment limit, not a code change; the
fourth-round run already established the 0-error baseline and no `.rs` file
changed this round.)


### Fixed - 2026-10-03

A pre-release audit that stopped looking for missing features and started
looking for **green that lies**: gates blind to files they never scan, modules
with no caller, and loops whose termination was asserted by a comment rather
than by construction. Seven defects, all fixed, none allowlisted.

- **The orphan gates could not see untracked files.** `check_orphan_docs.py`
  and `check_orphan_scripts.py` read `git ls-files`, which is exactly right in
  CI and exactly wrong locally: a new document no one had `git add`ed was
  invisible, so the gates stayed green and CI went red on the first push.
  Running them with the tree staged surfaced **8 unreachable documents**
  (`docs/new.md`, the plan itself, and six archived AlphaTA-era reports). The
  misnamed `docs/new.md` is now
  `docs/archive/finkit-architecture-v3.1-implementation-plan.md`, and the
  archive index explains that the *deleted* V4 architecture documents are not
  the same thing as the current V4 plan.
- **`workflow_run` was a completely ungated dead-link risk.**
  `release-readiness.yml` — the release licence — waits on eight workflows *by
  display name*; one wrong letter and the trigger never fires, while
  `check_workflow_liveness.py` still reported "every workflow has a trigger that
  can fire" because the file kept its `workflow_dispatch`. The same required
  list is maintained a second time, by file name, in
  `release_readiness_aggregate.py`, with nothing comparing the two.
  `check_workflow_liveness.py` now resolves every `workflow_run` target against
  the declared `name:`s and checks the two lists in both directions. It also
  stops mis-reporting `workflow_run` as a non-automatic trigger — a workflow
  relying on `workflow_run` alone would have been failed as unreachable.
- **7.3 MB of build output was committed, and the no-build-artifacts gate said
  there was none.** `ci.yml` generates `gpu_large_chart.html` (7 MB) by running
  `cargo run --example gpu_large_chart` and consumes it in the next step; five
  sibling files come from `--example improved_chart`. All six were tracked, and
  none matched the gate's `target/`/`dist/`/binary-extension rules, so it
  reported "none look like build output" over 7.3 MB of it. The files are
  untracked (working copies kept), `.gitignore` covers them, and the gate grew a
  repository-root rule for renderable output.
- **`RuntimeContext::ArtifactCache` had no production caller.** Its own
  documentation says it is "the one cache the runtime context owns", replacing
  the per-layer caches that made "is this already compiled?" have a different
  answer in every layer — and nothing ever wrote to it. The plan states
  `content_hash` exists so that "formulas and factors computing the same series
  can share a cache"; both halves existed and were never connected. New
  `UnifiedRuntime::compile_semantic_graph_cached` stores a
  `CompiledPlanArtifact` (plan + CSE report + dependency shape — plain data) in a
  caller-supplied `RuntimeContext`, so a scan compiles one declaration instead of
  one per symbol. The executor is deliberately **not** cached: it owns an arena
  and persistent kernel state, and sharing it would let the second symbol
  overwrite the first one's working set.
- **`execute_into` could advance persistent kernel state on a rejected call.**
  Its documentation promised a "clean rejection rather than a half-updated
  result", but the length check ran *after* `execute()`: a stateful kernel had
  already advanced its arena slot, so a retry with a corrected destination
  computed the *second* value and returned it as the first. The logical length is
  known before the run, so both the count and the length are now validated up
  front and a rejected call has no side effects at all.
- **Cohen–Sutherland clipping terminated only by comment.**
  `visualization::geometry::ClipRect::clip_line` argued that "each pass either
  returns or clips away one outside region, and there are only four" — true in
  exact arithmetic, false in floating point, where rounding can leave the new
  coordinate a hair outside the boundary so the outcode bit survives and an
  unbounded `loop` re-clips the same endpoint forever. It is now `for _ in 0..4`,
  with a residual treated as unclippable: refusing to draw is strictly safer than
  emitting a point that is still outside the rect.
- **`parse_pine` could be made to overflow the stack by its input.** Pine source
  is user input (CLI, Python binding, HTTP), and the entry point had no size or
  depth limit in front of a recursive-descent pest grammar plus a recursive pair
  walk — twenty thousand `(` would abort the process instead of returning a
  `PineError`. A budget check now runs first: iterative and lexical (it must not
  recurse, or it would carry the very failure it prevents), 128 levels of bracket
  nesting and 1 MiB of source, skipping string literals and `//` comments.

Every fix ships with a gate that can fail, and the new gates were verified by
injection rather than by observing green — a gate that cannot go red is worse
than no gate at all.

A fourth pass re-ran the same three questions over the *result* of the first
three, on the theory that a refactor which deletes twenty thousand lines is
itself a fresh source of orphans. It found three more defects — and one of them
had been one command away from being committed as a "fix". The fourth item
below is not a defect but the guard that keeps the first one from returning.

- **`indicators::volume::adosc_into` was deleted by the kernel-unification
  pass, and the gate that noticed was about to hide it.** Deduplicating the
  fused ADOSC implementation onto `math::volume_kernels::adosc_into` was
  correct; forgetting that the public name now had nothing to resolve to was
  not. Nothing failed: `cargo check` cannot see a *removed* `pub fn` that no
  in-tree caller uses, and the unit test named
  `test_adosc_into_matches_allocating_path` had been rewritten to call the
  kernel directly, so the test that looked like it covered the public entry
  point was covering its implementation instead. The only artifact that
  complained was `gen_ssot_docs.py --check`, whose snapshot no longer listed
  `adosc_into` (389 → 388 public functions) — and the documented remedy is
  "run `--generate`", which would have recorded the breakage as the new
  baseline. The function is restored as a validating wrapper over the canonical
  kernel, which also gives it the kernel's stricter pre-conditions (a zero
  period is now rejected, and a too-short series reports
  `TaError::InsufficientData` instead of reading past warm-up). The unit test
  calls the public wrapper again.
- **The class, not just the instance.** New target
  `core/tests/indicator_api_surface.rs` pins all 53 public
  `indicators::<module>::<name>_into` entry points plus the root re-export seam
  in `core/src/indicators/mod.rs`, where an explicit
  `pub use math::volume_kernels::{ad, adosc, obv}` shadows same-named glob
  members from `pub use volume::*` — the exact place `adosc_into` fell through.
  Removing or renaming any pinned name now fails **compilation**, verified by
  injection (`E0425`), so the decision becomes deliberate and reviewable. The
  guard is one-directional on purpose: it constrains removals, not additions,
  so it cannot rot into a list nobody wants to maintain.
- **Two `#[allow(clippy::uninit_vec)]` attributes outlived their reason.**
  `indicators::volume::ad` and `obv` still carried the allow after the same
  refactor replaced their `set_len` blocks with `Array1::zeros` /
  `vec![0.0; len]`, leaving a safety-relevant waiver attached to code that no
  longer does the thing being waived — and inviting the next reader to copy the
  pattern as a licence to skip initialisation. Scanned all six `uninit_vec`
  sites in the workspace; the other four genuinely still call `set_len`.
  **Those four are now `#[expect(clippy::uninit_vec)]` instead of `#[allow]`,
  and the clippy job denies `unfulfilled_lint_expectations`**, so a suppression
  whose justification disappears fails the build rather than silently outliving
  it — which `#[allow]` cannot express at all. Both directions were verified:
  the four legitimate sites are fulfilled under clippy and produce nothing under
  plain `cargo check` with `-D warnings` (tool lints are not evaluated without
  the tool, so the other jobs are untouched), while an expectation planted on
  `volume::obv` — which no longer calls `set_len` — reports *"this lint
  expectation is unfulfilled"*.
- **`gen_ssot_docs.py` rewrote its output with CRLF on every run.**
  `Path.write_text` translates `\n` to the platform separator, while
  `.gitattributes` pins `*.md` to `eol=lf`, so each `--generate` left seven
  files whose *diff* was empty and whose `git status` entry was not. A gate
  directory that is permanently dirty trains people to stop reading
  `git status`. Writes now pass `newline="\n"`.
- **`docs/generated/indicators.md` is therefore unchanged by this pass**, which
  is the point: the restored surface makes the snapshot match again. The
  regenerated file that this pass initially produced has been discarded.

### Fixed - 2026-09-29

This maintenance pass connects the release pipelines end-to-end and removes the
last broken links, orphan logic and silent-pass gates so the project reaches a
publishable state.

- **GitHub Release now auto-builds installers.** New workflow
  `.github/workflows/release-installers.yml` listens to the `release:
  published` event (the "publish a Release from the UI / `gh release create`"
  path that every other tag-driven workflow silently missed) and builds
  native + MSI (Windows), native + deb + rpm (Linux) and native + pkg + dmg
  (macOS), then attaches them to the Release with `--clobber` so re-runs
  repair in place. A `report` job opens an issue if any installer is missing.
- **Installer build scripts are now real.** `scripts/build-installer.sh`
  supports multiple `--target` flags (previously repeated flags silently
  overwrote each other, dropping installers), fails loudly on unknown targets
  and missing tools instead of returning exit 0, and builds a self-contained
  `native` bundle on every host. The committed `packaging/wix/Product.wxs`
  (previously referenced but never committed, making the MSI target dead) now
  defines the Windows installer. `scripts/build-installer-msi.cmd` is reduced to
  a thin wrapper over the bash script.
- **Native SDK archive is cross-platform.** `scripts/build_native_archive.py`
  now builds `linux-x64`, `linux-aarch64`, `macos-x64` and `macos-arm64`
  (`.tar.gz`) in addition to `windows-x64` (`.zip`), which is why it was an
  orphan in CI. `scripts/check_installer_contract.py` asserts the MSI payload
  and the native archive agree on their member list.
- **Gates no longer lie.** `scripts/check_versions.py` ignored three real
  version mismatches (it now passes). `scripts/check_workflow_liveness.py` no
  longer short-circuits to "healthy" on `workflow_dispatch`, so the workflow
  pinned to a deleted `main` branch is now correctly flagged.
- **Release manifest gate is live.** `scripts/refresh_release_manifests.py`
  read a non-existent `artifacts` field (always empty → always passed); it now
  validates the real `components` entries and refreshes digests in place.
- **Makefile ghosts removed.** `make packages` (always failed), `make
  docker-bench` (no recipe) and the missing `installer` entry point are fixed.
- **Node binding type contract fixed.** Two Rust structs both named
  `MacdResult` (vector in `lib.rs`, scalar in `streaming.rs`) produced two
  `MacdResult` interfaces in `ffi/node-binding/index.d.ts`, a duplicate-identifier
  error for TypeScript consumers; the streaming struct is now
  `StreamingMacdResult`.
- **API reference corrected.** Go (function names, returns, module path),
  .NET (method names, `MacdResult`/`BbandsResult` fields, example), Python
  (exception class names), Rust (removed non-existent `bollinger_bands` /
  `head_shoulders`) and Node.js (camelCase names, required params, `cdlDoji` /
  `detectDoubleTop` moved out of `Indicators`) now match the actual exports.
  Java `macd`/`bbands`/`stoch` are correctly documented as `void` methods that
  write into a pre-allocated result object; pattern functions live in `Patterns`
  / `ChartPatterns`.

### Added

- `scripts/check_coverage.py` is wired into CI so it is no longer an orphan.
- `scripts/check_installer_contract.py` is wired into CI.

- The 31 TA-Lib 0.7/0.8 indicator functions that were numeric-green but
  unreachable from the formula engine: `AC`, `ACCBANDS`, `ADR`, `AO`, `AROON`,
  `CMOU`, `COPPOCK`, `CVI`, `EFI`, `ER`, `ERI`, `FOSC`, `FRACTAL`, `HA`, `KC`,
  `MAMA`, `MARKETFI`, `MASSI`, `NVI`, `PERCENTRANK`, `PVI`, `PVO`, `PVT`,
  `QSTICK`, `RVI`, `RVOL`, `SMI`, `VHF`, `VORTEX`, `WAD`, `ZLEMA`, together with
  their multi-output registrations.
- `tests/contracts/formula_dialect_coverage_v1.json`, a per-function coverage
  contract for 通达信 / 同花顺 / 大智慧 / TradingView Pine. Each row carries
  three independent facts: `status` (usable out of the box?), `registered` (a
  machine-derived fact about the live function table) and `provenance`
  (substantiated by a named in-repo file, or merely attributed to a vendor
  list). Each terminal also records which rows a checked-in corpus actually
  calls, recomputed by the gate from the corpus files.
- 大智慧 as the sixth `FormulaTerminal` (`dzh` / `dazhihui` / `大智慧`),
  routing the 同花顺 common subset.
- Alpha158 (158 factors) and WorldQuant101 (17 computable, the rest annotated
  with the reason they are not) as declarative factor graphs, plus a one-line
  `finkit.factor_library("alpha158")` entry point in Rust and Python.
- Pine user-defined function blocks, tuple destructuring, and plot visual
  metadata.
- `scripts/build_native_archive.py`, a builder for the native C/C++ SDK archive
  (`dist/native/<platform>/finkit-<version>-native-<platform>.zip`). The archive
  used to be hand-zipped, so its member list and timestamps drifted from release
  to release. It now packs a fixed member set with a fixed 1980-01-01 timestamp
  (so identical inputs give a byte-identical archive), warns when the target
  directory it did *not* choose holds a differing `finkit_ffi.dll` (two build
  trees can coexist and linker output is not reproducible), and offers
  `--verify` to confirm the archive on disk still matches the current build.
  Wired into `make build-native-archive` / `make verify-native-archive`.
- `scripts/check_orphan_scripts.py`, a gate that fails when a file under
  `scripts/` has no consumer anywhere in the tree. A dead script reads as
  documentation of a workflow that no longer exists, and a second, unused
  implementation of a gate silently lowers the bar for the one that runs.
- `scripts/check_workflow_liveness.py`, a gate that fails when a GitHub Actions
  workflow can never be triggered — every trigger is branch-filtered, no
  filtered branch exists, and there is no `workflow_dispatch`/`schedule`/
  `release`/`workflow_call`. It also reports *dormant* branch filters (filters
  matching no existing branch) on workflows that are still reachable, so a
  stale `on:` block is visible without failing the build.
- An enforcement point for ADR 0011: `scripts/check_rustdoc.sh` now runs from
  `make check-rustdoc` and from the CI `doc` job, so a local run and CI cannot
  disagree about what "the public surface is documented" means.
- `scripts/check_script_references.py`, the inverse of the orphan check: it
  fails when a workflow step, Makefile recipe or document invokes a `scripts/`
  path that is not in the tree. A reference like that has a name, a place in
  the release checklist, and often a paragraph explaining what it enforces —
  and no implementation, so nothing compiles it and nothing tests it. Paths
  that are legitimately absent (planned work, or a helper the caller writes
  itself before running it) must be recorded in `RECORDED_MISSING` with a
  reason, and the list is checked in both directions. Wired into the CI
  `binding-ssot` job and `make check-script-refs`.
- `scripts/refresh_release_manifests.py`, which recomputes the `size_bytes` /
  `sha256` records in `dist/manifest.json` and
  `dist/python/windows-x64/manifest.json` from the artefacts on disk and then
  re-reads what it wrote to confirm the match. Those numbers were refreshed by
  hand, which is the same failure mode the native archive already had: linker
  output is not reproducible, so every rebuild changes the digests and a stale
  record is indistinguishable from a correct one by inspection. `--check` is
  the verify-only form. Wired into `make refresh-release-manifests` /
  `make check-release-manifests`.
- `scripts/check_dead_code_allows.py`, which fails on any
  `#[allow(dead_code)]`, `#![allow(dead_code)]` or `expect(dead_code)` that
  does not carry a `// why` comment on the line, inside the attribute block
  above it, or on the line directly above that block. That attribute disables
  the only compiler check that can see orphan logic, so an item behind it is
  invisible to every build and every test, and a bare suppression is
  indistinguishable from a function that was never wired up. Wired into the CI
  `binding-ssot` job and `make check-dead-code`.
- `scripts/check_orphan_docs.py`, which fails when a tracked Markdown document is
  reachable from no other document, by link or by backticked repository-relative
  path. Part of `make check-orphans`.
- `scripts/check_ios_header_contract.py`, which compares
  `ffi/ios-binding/include/finkit.h` against the iOS binding's shipped
  `#[no_mangle] extern "C"` exports in both directions, ignoring
  `#[cfg(test)]`-gated symbols. It is the iOS counterpart to
  `gen_c_header.py --check` for the C binding.

### Changed

- `FormulaExecutionMode` is now a real execution switch. The default is still
  the tree interpreter; the compiled plan path is selected explicitly and is
  covered by a differential gate against the interpreter.
- The Pine canonical-name mapping is derived from the engine (the builtin
  table plus the `ast_mapper` special cases plus the `TA_<NAME>` fallback) and
  recorded in the coverage contract, instead of being mirrored by hand in the
  generator.
- The release version gate now covers `docs/getting-started.md`,
  `docs/cli.md`, `docs/language-bindings.md` and `docs/development.md`, and it
  matches any `MAJOR.MINOR.PATCH` series rather than only `0.1.x`. Those four
  documents had drifted to `v0.1.5` while the workspace was at `0.1.15`, and
  the old pattern could not have caught it.
- Removed the unreferenced `formula::pine::runtime` module (`PineRuntime`,
  `PineRuntimeError`, `SeriesValue` and its plot/security/barstate support
  types). It was introduced in the initial commit and never had a caller
  anywhere in the workspace, including the language bindings and the test
  suite. It was a second, parallel Pine evaluator for semantics the mapper plus
  the AlphaTA engine already own, so it could only ever drift from them.


### Fixed

- **45 float-ordering comparisons panicked on a NaN input.** Each
  `partial_cmp(..).unwrap()` aborts the whole evaluation as soon as a window
  holds a `NaN`, because `f64::partial_cmp` returns `None` for `NaN` and the
  `unwrap` then panics. That is the opposite of this codebase's
  `null_policy: "nan"` convention, where a gap value should propagate as `NaN`
  rather than crash. 34 of the sites were in `patterns/chart.rs` (every chart
  detector: `double_top` / `double_bottom`, `head_and_shoulders_top` /
  `_bottom`, `triple_top` / `_bottom`, the ascending/descending/symmetrical
  triangles, the rising/falling wedges, `pennant`, `flag` and `rectangle`), and
  the rest in `indicators/talib_ext.rs` (`percentile`),
  `indicators/volume_profile.rs`, `indicators/classic_patterns.rs`,
  `patterns/astock_kline.rs`, `patterns/classic_ext.rs` and
  `patterns/streaming.rs`. All now use `f64::total_cmp` — a total order that
  never returns `None`, and already the house style in `math/quantile.rs`,
  `math/rank.rs`, `math/regression.rs` and `factors.rs` — so a `NaN` propagates
  instead of aborting the process. Guarded by a new
  `scripts/check_nan_unsafe_ordering.py` (`make check-nan-safety`) that fails on
  any `partial_cmp(..).unwrap()` in the scanned Rust sources, and by regression
  tests that feed `NaN` to `percentile` and to the chart detectors.

- **Eleven functions were unusable from the compiled-plan path, and 28 names
  were affected.** `BARSLAST`, `BREAKDOWN`, `BREAKOUT`, `COUNT`, `DEAD_CROSS`,
  `GAP_SIGNAL`, `GOLDEN_CROSS`, `MA_ALIGN`, `RELATIVE_STRENGTH`,
  `TREND_BREAKOUT` and `VOLUME_SURGE` were declared pure, so the planner
  lowered them to `CALL:<NAME>` kernels — but no such kernel existed, and every
  formula using one failed with `unsupported kernel` (code 1). Because the
  planner resolves registry aliases to their canonical name, another seventeen
  spellings broke with them: `CROSSUP` / `BULLISH_CROSS`, `CROSSDOWN` /
  `BEARISH_CROSS`, `BREAKOUT_UP` / `PRICE_BREAKOUT`, `BREAKOUT_DOWN` /
  `PRICE_BREAKDOWN`, `BREAKOUT_SCREEN` / `TREND_SCREEN`, `VOLSURGE` /
  `VOLUME_EXPANSION`, `MA_ALIGNMENT` / `TREND_ALIGN`, `RELSTRENGTH` /
  `RS_EXCESS_RETURN` and `GAP`. No corpus case exercised any of them, so the
  differential gate stayed green throughout. Each now has a kernel delegating
  to the exact `canonical_*` (or `fn_count` / `fn_barslast`) implementation the
  tree, bytecode and JIT paths already used, so the plan path agrees by
  construction. Covered by
  `domestic_screening_formulas_match_the_ast_through_the_plan_path`, and by a
  new invariant `every_alias_reaches_a_kernel_through_its_canonical_name`,
  which is what actually protects an alias — checking `CALL:<alias>` directly
  would have measured something no caller can reach and stayed green through
  the whole outage.

- **The registry told callers the wrong input shape for seventeen functions.**
  `ICHIMOKU_TENKAN`, `ICHIMOKU_KIJUN`, `FISHER`, `FISHER_SIGNAL`, the five
  `DONCHIAN*` projections and `AROON_UP` / `AROON_DN` were declared
  `InputKind::Hlc` — high, low, *close*, plus a period — while their
  implementations take high, low, period. A caller who followed the metadata
  wrote `ICHIMOKU_TENKAN(HIGH, LOW, CLOSE, 9)` and got a tenkan-sen computed
  over a period of `CLOSE[0]` (about 100, not 9) on the tree path, and a hard
  arity error on the plan path. The enum had no way to say "two series", which
  is why `Hlc` was used as an approximation; it now has `InputKind::Hl`.
  A new gate, `declared_signatures_run_on_both_execution_paths`, rebuilds the
  call the metadata describes from the registry itself and runs it through both
  paths, so this class of drift cannot recur silently.

  The same investigation found that the *parameter* half of the declaration is
  a superset for five functions — `BBANDS` / `BOLLUP` / `BOLLDN` publish
  `nbdevup`, `nbdevdn` and `matype`, and `STDDEV` / `VAR` publish `nb_dev`.
  Those were **not** removed, because the operation and FFI surfaces really do
  accept them (`finkit.bbands(real, timeperiod, nbdevup, nbdevdn, matype)`,
  `finkit.stddev(close, timeperiod, nbdev)`); it is the formula surface that
  takes a shorter list, since a formula evaluates to a single series. The five
  are now recorded in the gate's `FORMULA_PARAM_SUBSET` list with the reason,
  and an entry that stops being needed fails the test.

- **`tests/formula_corpus/README.md` still said the plan path had no kernels.**
  A current (not dated-snapshot) document asserted that the 31 TA-Lib 0.7/0.8
  functions were available "only on the tree / bytecode / JIT paths", that
  `CALL:<NAME>` answered `code 1`, and that all 17 corpus cases were therefore
  registered in `DOMESTIC_UNSUPPORTED`. None of that has been true since
  2026-09-24: `unified_dispatch.rs` routes all 44 `CALL:<NAME>` plan kernels
  through `dispatch_modern_call`, which delegates each to the same
  `canonical_*` implementation the other paths use, so the plan path emits
  byte-identical output and the allowlist is empty. The prose now states the
  closed state and keeps the allowlist documented as the backlog mechanism it
  is. The corpus table itself was already bidirectionally gated; only the
  surrounding prose could drift.

- **`docs/api-reference.md` omitted seven shipped Python API groups.** The
  reference documented the classic TA surface and `Streaming*`, but said
  nothing about the Chanlun entry points (`chan_analyze` and its three
  multi-timeframe variants), `compute_composite`, `factor_library`, the formula
  template catalog (`formula_get_template` / `formula_search_templates` /
  `formula_list_categories` / `FormulaRegistry`), `CompiledFormula`, the market
  calendar (`resolve_market_session`) or `KlineChart` — all 18 of which are
  exported by the wheel. They are now documented, with the return shapes
  verified against a built wheel rather than inferred.
- **`factor_library` was documented nowhere and missing from the type stub.**
  It ships as a 0.2.0 entry point but appeared in neither `docs/python.md` nor
  `docs/api-reference.md`, and `.pyi` did not declare it, so a `mypy`/
  `pyright` user got a missing-name error on a public API. Both the docs and
  the stub now cover it. Its `FactorLibrary` return is documented as
  supporting `len()` and `in` but **not** iteration and **not** subscripting —
  it exposes neither `__iter__` nor `__getitem__`, so enumerate it with
  `names()`.
- **`docs/python.md` told users to iterate a `FactorLibrary`.** A first draft
  of the new section said to inspect it "with `len()` / iteration"; it is not
  iterable, so the example was corrected to `names()` before it shipped.

- **Six binding crates had `#[test]` functions that CI never ran.** `ci.yml`
  tested `finkit`, `finkit-factor-analysis`, `finkit-cli`,
  `finkit-visualization` and `finkit-ffi-common`; `multilang-release.yml` built
  `finkit-go`, `finkit-dotnet`, `finkit-java`, `finkit-android`, `finkit-ios`
  and `finkit-wasm` but only *compiled* them, and it drove the C/C++ `ctest`
  suite without ever running `cargo test -p finkit-ffi`. That left 55 Rust unit
  tests unexecuted — 31 in the C binding alone, including the
  `dispatch_ta`-unknown-name test that guards a fix from the previous round.
  `cargo test -p finkit-ffi` now runs in `ci.yml` and in the `v*`-tag job, and
  a new `binding-unit-tests` job covers the other six crates on every push and
  pull request.
- **The shipped Python type stub did not cover the documented surface.** Its
  own docstring promises it "covers the documented surface rather than all 400+
  exported names", but 16 documented, registered functions were missing from
  it: `formula_get_template`, `formula_search_templates`,
  `formula_list_categories`, `bbands`, `compute_indicators`, `cdl_doji`,
  `cdl_engulfing`, `cdl_hammer`, `detect_double_top`, `detect_double_bottom`,
  `detect_head_shoulders`, the four `chan_analyze*` entry points and the
  `FormulaRegistry` class. Each was callable at runtime and shown in `docs/`,
  and each was a type error for a `mypy`/`pyright` user. All 16 are now
  declared, and `check_python_stub.py` gained the reverse check that would have
  caught them — it previously only verified that a name *declared* in the stub
  exists, never that a documented name *is* declared.
- **`docs/generated/version-matrix.md` recorded a number that could not be
  correct in CI.** The "Criterion JSON benchmarks indexed: N" line counted files
  under a hard-coded `target/criterion/`, but this repository builds to
  `.cargo-target` (or `$CARGO_TARGET_DIR`), so the directory never existed and
  the line always read `0`. Pointing the generator at `$CARGO_TARGET_DIR`
  instead would have been worse: the count is a property of one developer's
  disk, so the committed document would disagree with CI and the
  `gen_ssot_docs.py --check` gate would flap. The count and the ~80 lines of
  helper code that computed it are removed; the section now points at
  `scripts/gen_benchmark_report.py`, which is the real local report path. The
  gate is now independent of the environment, verified both with and without
  `CARGO_TARGET_DIR` set against a populated `.cargo-target/criterion`.
- `docs/installation.md` §14a listed six members for the native C/C++ archive
  when `scripts/build_native_archive.py` packs eight — both
  `finkit_research.h` and `finkit_research.hpp` were missing from the table, so
  a reader following it would have shipped a bundle without the research
  headers. The section also omitted `$CARGO_TARGET_DIR` from the documented
  target-directory resolution order, which is the one step that matters in a
  checkout that redirects cargo.
- **`docs/archive/` was in `.gitignore` while 11 archived plans were already
  tracked.** Any newly archived document therefore became untracked *and*
  invisible to `git status`, so moving a file into the audit trail silently
  deleted it from the repository. The consequence was live, not theoretical:
  `docs/archive/README.md` — the archive index that `docs/README.md` links to
  three times — had never been committed, so `scripts/check_docs_links.py`
  reported success locally (the file was on the author's disk) and failed on a
  clean checkout. The rule is removed, the index is committed, and
  `check_docs_links.py` now fails when a link target exists on disk but is not
  tracked by git.
- **`docs/ffi/memory-contract.md` documented an API that does not exist.** It
  described `alphata_free_string` and an eleven-function `alphata_kline_chart_*`
  / `alphata_kline_data_*` chart surface, none of which is exported by any
  binding (the chart surface exists for the Java binding and the CLI only), and
  it omitted the entire JSON-contract surface plus the research pair — 18 real
  exports with no documented ownership. The document is the ownership contract a
  C consumer follows to avoid undefined behaviour, so a fictional free function
  in it is a memory-safety trap. All 99 shipped exports are now documented and
  the invented names are gone; `docs/ffi/error-codes.md` had the same
  `alphata_free_string` reference and additionally truncated the formula error
  tier at `55`–`59`, leaving `60` and `61` undocumented.
- `FormulaError::BackendUnsupported` — the variant that makes tree-only entry
  points *refuse* to run under `FormulaExecutionMode::Plan` instead of silently
  walking the tree — was mapped to C error code `61` but documented nowhere.
  `docs/formula-runtime-contract.md` now states the backend contract, lists the
  13 tree-only entry points and the plan-capable ones, and
  `docs/ffi/error-codes.md` spells out the full `50`–`61` formula tier.
- **`scripts/check_orphan_docs.py`** (new gate): `docs/formula-talib-contract.md`
  and `docs/quant-evaluation.md` were both current, substantive documents with
  zero inbound references anywhere in the repository, so the documentation index
  could not reach them. Both are indexed now, the dated competitive analyses and
  roadmaps are indexed under an explicitly non-authoritative section, and the
  gate fails if a document ever becomes unreachable again.
- **`scripts/check_ios_header_contract.py`** (new gate):
  `ffi/ios-binding/include/finkit.h` was missing declarations for three shipped
  exports (`finkit_ios_factor_study_json`, `finkit_ios_factor_study_free_string`,
  `finkit_ios_quant_evaluation_json`). The Swift wrappers reach them through
  `@_silgen_name`, which is exactly why nothing noticed — but a plain C consumer
  of the iOS static library could not see that the research and quant-evaluation
  entry points existed. The header is complete and the gate keeps it that way.
- **The scheduled TA-Lib head-to-head guardrail installed TA-Lib C 0.7.1 while
  the numeric contract pins 0.8.1.** The guardrail therefore measured Finkit
  against a superseded upstream and could pass more easily than its threshold
  implies. `competitive-benchmark.yml` now reads `talib_core_version` from
  `tests/contracts/talib_coverage_matrix_v1.json` at run time and refuses to
  record evidence when the installed library disagrees, so the two can no longer
  drift.
- `scripts/check_versions.py` covered 15 release-facing documents and missed 10
  more that state Finkit release versions — including `docs/usage.md`,
  `docs/troubleshooting.md`, `CONTRIBUTING.md` and `core/README.md`, which had
  rotted to `v0.1.5` and `v0.1.3` while the published release was `v0.1.15`.
  The list is extended and the stale values corrected.
- `docs/formula-templates.md` claimed 317 templates but printed a per-language
  export table with rows for C and iOS template lookup functions that are not
  exported by those bindings; `docs/formula-performance.md`'s multi-language
  parity table marked C as supporting `formula_eval`, `_multi`, `_draw`,
  `_debug` and `_validate`, none of which the C ABI has. Both tables are
  rebuilt from the actual exports.
- `docs/indicators.md` listed two indicators that do not exist
  (`KAMA_VOLATILITY`, `MEDIPRICE`) and documented the indicator error type as
  `Result<Array<TaError>>` with three variants (`InvalidPeriod`,
  `InvalidParameters`, `InvalidInput`) that no error enum defines.
  `docs/features.md` documented `MultiPeriodFeature::fast_periods/medium_periods/
  slow_periods` (the methods are `fast`/`medium`/`slow`), wrong default period
  sets, an unsupported indicator list, and `to_csv`/`to_json_lines`/
  `to_arrow_ipc` as returning strings when they write to a path.
  `docs/api-reference.md` gave `StreamingIndicator::next` a non-`Option` return
  and a `&dyn Ohlcv` input for `StreamingAtr`, which takes a `(high, low, close)`
  tuple.
- Java JNI multi-output exports (`MAMA`, `BBANDS`, `SAR`, `MACD`, `STOCH`, and
  `AROON`) could unwind a Rust panic across the JNI boundary because the binding
  synchronizer only wrapped functions with an explicit `->` return type. They
  return `()` and populate a result object, so the old parser skipped them.
  `sync_bindings.py` now recognizes Java void bodies and wraps them with
  `ffi_catch_void`; the generated six exports are guarded, and the structural
  warning contract prevents the hole from returning. The same pass wrapped 21
  unguarded manual Java exports (including chart and research bridges) with the
  return-type-appropriate guard.
- Five private functions were unreachable from every build and every test, and
  nothing reported it: each carried a bare `#[allow(dead_code)]`, which is
  precisely the attribute that hides an item from the compiler's dead-code
  check. Removing the suppressions and letting `cargo check` adjudicate found
  them, and they are gone — `push_sliding_max`, `push_sliding_min`,
  `aroonosc_scan_inner` and `aroonosc_deque_inner` in
  `core/src/indicators/momentum.rs` (two deque helpers with no call sites, and
  two alternative AROONOSC kernels superseded by the inlined deque loop in
  `aroonosc` itself, 178 lines), and `null_pointer` in
  `ffi/c-binding/src/lib.rs`, a second implementation of the
  `FfiError::NullPointer` arm already inside `map_ta_error`.
- Six `#[allow(dead_code)]` suppressions were stale: the item they guarded is
  genuinely used, so the attribute only served to make a future regression
  invisible. Removed from `bop_scalar`, `avgprice_scalar`,
  `avx2_fma_available`, `ta_t3`, `approx_eq` and the `Case::fixture_path`
  field (also deleted: `approx_eq` in the `top_bottom` test module, which the
  test build proved unused).
- The repository-hygiene gates were blind to any tracked file whose path
  contains a non-ASCII byte. `git ls-files` octal-escapes such paths
  (`"docs/...finkit-\350\220\275..."`), so `Path.is_file()` failed and nine
  tracked files — every one of them a document that mentions `scripts/` paths —
  silently dropped out of the scan. Both checks now enumerate with
  `git ls-files -z`.
- `scripts/check_orphan_scripts.py` could be fooled in two ways, both of which
  made a dead script look alive. A cluster of scripts that only mention each
  other satisfied the one-hop "is it mentioned anywhere" test for every member,
  and a bare stem matched as a substring, so a script named `_probe_a.py`
  counted as referenced by
  `fn the_probe_actually_exercises_the_degenerate_guard()`. References are now
  peeled to a fixed point and stem hits must fall on identifier boundaries.
- Four one-shot migration-bot workflows were still in the tree after their work
  landed: `apply-architecture-v3-round2.yml` and `apply-extrema-round6.yml`
  pushed to `perf/outperform-talib-v3-20260904`, and `apply-perf-plan.yml` and
  `apply-talib-performance-plan.yml` pushed to `fix/*` branches that no longer
  exist. They carried `contents: write`, and the first embedded a 27 KB base64
  payload that rewrote seven source files and force-pushed, so its own command
  line named a script that was never in the tree. Removed.
- Six completed TA-Lib migration codemods that nothing invoked:
  `apply_core_hotpath_fixes.py`, `apply_formula_output_fixes.py`,
  `apply_formula_runtime_performance_fixes.py`,
  `apply_talib_existing_contract_fixes.py`, `apply_talib_semantic_fixes.py`
  and `normalize_migration_parsers.py`. They rewrite Rust sources, so running
  one against today's tree would have applied a stale transformation; their
  effect is preserved by the commits that landed it.
- `make verify-native-archive` was red in any checkout that redirects cargo
  with `CARGO_TARGET_DIR`. The archive is packed from the tree cargo actually
  wrote, but `--verify` looked in `target/release` first and compared against a
  `finkit_ffi.dll` built in a different tree — and linker output differs
  between trees, so the comparison could never succeed and the message gave no
  clue why. The script now locates the release directory the way cargo does
  (`--target-dir`, then `$CARGO_TARGET_DIR/release`, then `target/release`),
  and when a member does differ it names the tree the archive *was* packed
  from and the flag to pass.
- `docs/refactor-plan-2026-09-21.md`, then the declared sole execution baseline
  (now re-labelled a constraint record — see the sixth-audit-pass entry above),
  pointed at `docs/improvement-plan-2026-09-20.md` and
  `docs/architecture-gap-assessment-2026-09-20.md` at their pre-archive
  locations, in the same sentence that says historical plans have been
  archived. Both paths now carry the `docs/archive/` prefix.
- Pine `na(x)` was silently mis-parsed. `na` is a grammar keyword, so it could
  not reach the call rule, and `na(close)` parsed *without error* as an `na`
  literal followed by a discarded `(close)` expression statement — so
  `is_missing = na(close)` evaluated to `NaN` instead of `ISNA(CLOSE)`. The
  grammar now has an explicit `na_call` rule.
- The Pine coverage figure was a false green. The generator's hand-written
  canonical-name mirror disagreed with the engine on 7 names, left 48 spellings
  unmapped, and credited 43 unusable names as out-of-the-box, reporting 97.8%
  coverage. Deriving the mapping from the engine drops it to the real 81.7%
  (263 rows, 47 `unsupported`).
- Formula and plan kernel correctness for `IF`, `STOCHF`, `SAR`, `BOLLUP` /
  `BOLLMID` / `BOLLDN`, `DEA`, `WILLR`, `PLUS_DI` / `MINUS_DI` / `ADX`, `TRIX`,
  `STDDEV`, `CROSS` / `FIXNAN` / `VAR`, and the money-flow / `TR` host kernels.
- External variables bound through `HostContext` are now visible on the
  bytecode and JIT paths, which previously resolved only their own private
  tables.
- `MACDFIX` used MACD's `2/(period+1)` EMA constants instead of TA-Lib's fixed
  `0.15` / `0.075`, so `MACDFIX(CLOSE, 9)` on the formula path disagreed with
  `indicators::macdfix` on the very same input. Every differential path agreed
  with every other path — on the wrong constants — so only an absolute
  comparison against the reference implementation could see it. The formula
  function now delegates to the parity implementation.
- The MACD family (`MACDFIX`, `MACDEXT`, `DIFF`, `DEA`) treated a leading
  warm-up run as bad data instead of skipping it, so a composed input such as
  `MACDFIX(MA(CLOSE, 5), 9)` collapsed to an all-NaN series. The plan path's
  `MACD` and `DEA` kernels carried the same gap independently of the tree path,
  which the corpus never exercised.
- `MACDEXT`'s three `matype` arguments were read with the period extractor,
  which rejects `0` outright. The canonical TA-Lib spelling
  `MACDEXT(close, 12, 0, 26, 0, 9, 0)` therefore failed with
  "period must be > 0", and the `0 => SMA` match arm behind it was unreachable:
  only the `EMA` selector ever worked.
- Pine `barstate.*` mapped to a variable name that no execution path resolved
  (`barstate_islast` and friends), so any script reading `barstate` parsed,
  mapped, and then failed with "Unknown variable". The fields are now
  translated in the mapper into `BARPOS` / `BARSCOUNT` comparisons, which keeps
  the decision inside the frontend and out of the five numeric backends.
- Pine's `hl2`, `hlc3`, `ohlc4` and `time` had the same defect: they were
  renamed to `HL2` / `HLC3` / `OHLC4` / `DATE` and handed to the engine as
  variable references, and none of those names resolves. The derived price
  sources are now expanded into OPEN/HIGH/LOW/CLOSE arithmetic (identical to
  `streaming::PriceSource`) and `time` is emitted as a call to the
  zero-argument `DATE()` builtin. The Pine corpus runner had been pre-binding
  `HL2` / `HLC3` / `OHLC4` into the context, which is exactly why a green
  corpus gate never noticed; that workaround is removed.
- SIMD documentation no longer claims acceleration that does not exist. The
  Hilbert section in `indicators::cycle` said the pipeline was AVX2-accelerated
  by the `simd_ht_*` kernels and that `ht_sine`'s terminal stage batched through
  `simd_sin_cos`; in fact the production Hilbert chain is a scalar TA-Lib-faithful
  state machine and `simd_sin_cos` is only reached from a unit test. Likewise
  `simd_ht_dcphase` documented an AVX2 path and radian output that its body does
  not provide (it is a scalar, degree-returning approximation), and
  `simd_aroon` / `simd_kama` / `simd_ema_next` / `simd_mama_hilbert` /
  `simd_sar_step` / `simd_atr` documented AVX2 batching that their scalar bodies
  do not perform. The module docs for `math::simd_ops` and `math::simd_ops_avx512`
  now state the real dispatch policy (AVX-512 falls back to scalar for every
  kernel except `simd512_sma`) and the `avx512` coverage table no longer lists
  non-existent `simd512_macd` / `simd512_bbands` / `simd512_atr` / `simd512_adx`
  names (they are `*_seed`). No numeric behaviour changed — these are
  documentation↔reality corrections.

- Streaming `OBV` diverged from the batch `obv` / `simd_obv` contract on flat
  bars. It accumulated `diff.signum() * volume`, and `f64::signum(0.0)` is
  `+1.0`, so a bar with an unchanged close wrongly added `volume` instead of
  leaving OBV unchanged (the comment even claimed the opposite). The streaming
  step is now an explicit three-way `> prev` / `< prev` / flat, matching the AVX2
  and scalar batch kernels. A regression gate pins the two implementations
  together on shared bars including a flat one.

- The formula streaming path (`FormulaStatefulStream`) was a differential
  blind spot: `check_all_paths` never instantiates it, so two of its stateful
  indicators silently disagreed with the batch formula table.
  1. `SMA(X, N[, M])` was implemented as a simple moving average (warm-up `NaN`
     prefix), while the batch `fn_sma` is recursive smoothing seeded at the
     first value (`value = (M*cur + (N - M)*prev) / N`). The streaming path now
     has its own `StreamingSmaSmoothed` and routes `SMA` (distinct from `MA`) to
     it in both `compile_state` and `compile_expression_stateful_function`, with
     the optional third weight argument honoured.
  2. `MACD(...)` streaming returned the histogram (`macd - signal`) while the
     batch `MACD` formula returns the DIF line (`fast_ema - slow_ema`); the two
     are different series. The streaming `MACD` now returns `value.macd`.
  A new gate `streaming_stateful_matches_batch` compares MA / SMA / EMA / WMA /
  RSI / HHV / LLV / SUM / REF / STD / VAR / CROSS / CROSSBELOW / ATR / MACD
  (direct and composed) against the batch (AST) path on oscillating data,
  closing the blind spot.

- The streaming formula path poisoned its rolling accumulators on a leading
  `NaN` run. A composed source such as `MA(MA(CLOSE,5),9)` (or raw input that
  opens with a warm-up `NaN` prefix) handed the outer indicator a leading `NaN`
  run; the batch math layer skips it via `math::leading_warmup`, but the
  streaming accumulators absorbed the `NaN` and returned an all-`NaN` series.
  Both the direct state kernels and the composed expression kernels now hold
  the indicator until the first finite input arrives, so its window starts on
  the first finite value exactly like the batch path. `RSI` is deliberately
  exempt: its batch kernel computes `change.max(0.0)` (and
  `NaN.max(0.0) == 0.0`), so it treats a leading `NaN` delta as a zero change
  and keeps its warm-up at `period`; feeding the `NaN` through reproduces
  `rsi_scalar` exactly, whereas skipping would shift the warm-up by the run. A
  second gate `streaming_stateful_survives_leading_nan` locks this on a context
  whose columns open with a `NaN` run.

- The `%` operator had four implementations that disagreed with the other eight.
  The dialect contract is the **floor-based** remainder (`-7 % 3` is `2`), which
  the tree scalar and array kernels, the bytecode VM, the plan's `BINARY:Mod`
  kernel and the streaming path all implement; but `FormulaOptimizer`'s
  constant folder, the JIT's bytecode folder, the JIT's *runtime* `OpCode::Mod`
  and `compute_ir`'s `const_eval` used Rust's truncating `%` (`-7 % 3` is `-1`).
  A folder that changes the value of a formula is the worst form of this: with
  literals the optimizer folded `-7 % 3` to `-1`, while the equivalent
  `CLOSE % 3` on the same bar produced the floor form. All eleven sites now
  call `math::floor_remainder`. `MOD(A, B)` the *function* stays truncating by
  design; a new gate pins the two apart, and `%` gained a negative-dividend case
  in the four-way harness (the harness data is strictly positive, so
  `CLOSE % 3` alone is satisfied by either definition).

- The logical operators had two truthiness conventions. `truth.rs` records that
  `And` / `Or` / `Xor` / `Not` treat a value as true only when it is **strictly
  positive** (deliberately unlike `IF`'s branch selection, which is
  `!= 0.0`), and eight implementations follow it — the scalar kernels, the
  bytecode VM, the JIT, the plan's `BINARY:And/Or/Xor` and `UNARY:Not` kernels,
  the streaming path, the optimizer's folder and `fn_not`. `SimdOps::logical_and`
  / `logical_or` / `logical_xor` / `logical_not` compared against zero
  (`!= 0.0`), and those are what the *array* legs call, so the same expression
  shape answered two ways depending on whether an operand happened to be a
  scalar: `CLOSE AND 0` was `0.0` while `CLOSE AND (0 - CLOSE)` was `1.0`. All
  sites now call `truth::is_logical_true` / `truth::logical_bool`, and
  `SimdOps::select` now calls `truth::is_true` instead of restating it.

- `==` / `!=` compared with a tolerance everywhere except the array-array leg.
  Every kernel uses `|lhs - rhs| < 1e-10`, but `SimdOps::simd_eq_arrays` /
  `simd_neq_arrays` compared exactly (`_CMP_EQ_OQ` in the AVX2 kernel), so
  `CLOSE == (CLOSE + 1e-12)` answered `0.0` while the scalar leg answered
  `1.0`. The JIT had the same split *within one path*, choosing the exact SIMD
  kernel from 16 elements up and the tolerant scalar loop below — the
  length-dependent anomaly that `truth.rs` exists to prevent. The AVX2, AVX-512
  and NEON kernels now compute `|a - b| < eps` and `|a - b| >= eps` (ordered, so
  a `NaN` operand makes both comparisons false, exactly as before), and every
  site calls `math::nearly_equal` / `nearly_not_equal`.

- The second-moment family used the one-pass form `sum(x^2) - sum(x)^2 / n`,
  which subtracts two large nearly equal totals and loses every significant
  digit when a series' mean dwarfs its spread. A series and its exact affine
  image `2x + 3` are perfectly correlated, yet `correlation` — exposed to Python
  as `correlation` — returned `0.667` at a `1e9` baseline, `covariance` returned
  `520.1` where the exact answer is `693.3`, `batch_zscore` returned `-1.607`
  where the offset-invariant answer is `-1.705`, and `rolling_std_simd` returned
  `0.0` for every window of `1e12 + i` with `window = 3` where the true sample
  deviation is exactly `1.0`. Every spelling now goes through the new
  `math::centred_moments`, which subtracts the mean first so each addend stays
  at the scale of the spread. `math::statistics::{correlation, covariance}`,
  `features::{correlation_simd, batch_zscore_simd, rolling_mean_simd,
  rolling_std_simd}`, `features::combinations::rolling_correlation`,
  `features::rolling_stats::{rolling_zscore, pearson_ic}` are routed through it.
  The rolling spellings delegate to the canonical `math::kernels` entries
  instead of carrying their own sliding-window recurrences.

- `statistics::rolling_std_dev` took the square root of an *unclamped* variance,
  so a window whose true variance is zero but whose removable-Welford residue
  came out slightly negative produced `NaN` — while the canonical kernel
  `rolling_sample_stddev_into` and `features::rolling_std_simd` clamped and
  returned `0.0`. Two spellings of "rolling sample standard deviation" must not
  answer differently on the same window: `rolling_std_dev` now delegates to
  `rolling_sample_stddev_into`, and `rolling_sample_variance_into` clamps its
  output at the source so the variance is never negative and the deviation is
  exactly its square root.

- `features::simd_opt` carried doc claims its bodies did not implement:
  `rolling_std_simd` claimed to "match `statistics::rolling_std_dev`" (it did
  not — see above), `batch_minmax_simd` claimed to use
  `SimdOps::min_elementwise` / `max_elementwise` "for the reduction" (those are
  *element-wise* kernels and cannot reduce; the body runs a scalar loop), and
  `sum_and_sum_sq_simd` claimed an "AVX2-accelerated mul + horizontal reduce"
  (the reduce is scalar `iter().sum()`). The claims now match the bodies.

- The streaming registry and each indicator's own `IndicatorMeta` are two
  independent sources for the same metadata, and nothing tied them together, so
  they had drifted. Eight types answered `IndicatorMeta::category()` with the
  slug `"statistic"`, which is not a member of `VALID_CATEGORIES` — the declared
  vocabulary — so a consumer grouping by that value would invent a category the
  contract does not define; five of them (`BETA`, `CORREL`, `STDDEV`, `TSF`,
  `VAR`) additionally contradicted their own registry entry, which says
  `"statistics"`. `StreamingSuperTrend` reported `"volatility"` while the
  registry entry it is published under says `"overlap"`. The slugs are aligned,
  `"smc"` (the category of the two smart-money-concepts detectors) is added to
  `VALID_CATEGORIES`, `impl_indicator_meta!` now `debug_assert!`s that its slug
  is declared, and a new gate
  `scripts/check_streaming_registry_contract.py` (wired into the Docs Check
  workflow) fails on any undeclared slug or any category that disagrees with the
  registry entry.

- Three contract tests had gone red in crates the release gate never ran.
  `cargo test -p finkit` was the only test command used to certify the workspace
  locally, and it does not build `finkit-ffi-common` or `finkit-ffi`, even
  though `.github/workflows/ci.yml` does. Two regressions had accumulated behind
  that blind spot:
  1. `factor_catalog::tests::catalog_is_stable_and_describes_dependencies`
     asserted a hard-coded factor count of `9`. The built-in library grew to the
     demo factors plus Alpha158 plus WorldQuant101, so the FFI catalog
     legitimately publishes 184 and the assertion could only ever rot. It now
     derives the expectation from `builtin_factor_registry()`, which is the same
     registry the catalog projects.
  2. `profile_only_entries_publish_executable_parameter_contracts` (ffi-common)
     and `operation_catalog_json_exposes_talib_parameter_contract` (c-binding)
     both asserted that the *flat* `params` field of `STOCH` carries the TA-Lib
     five-parameter contract. That was true while `STOCH` was a profile-only
     name; it is now a registry-backed formula bridge, and the flat field
     describes the default `core_registry` profile instead — publishing the
     TA-Lib names there would hand `core_registry` callers parameters the core
     kernel silently ignores (`STOCH` is positional, defaults to `fastk = 14`,
     and has no `matype` arguments). Both tests now assert the executable TA-Lib
     contract where it actually lives, in
      `profile_output_contracts["talib_0_8_0"].params`, and a new loop in the
      ffi-common test pins every declared TA-Lib parameter spec to that contract.

- The streaming discovery API was documented at a path that does not resolve.
  `docs/api-reference.md` told users to
  `use finkit::streaming::{all_indicators, by_id, by_category, registry_document, VALID_CATEGORIES}`,
  but `streaming/mod.rs` re-exported only `all_indicators`, leaving its natural
  companions reachable only through `streaming::registry::`. The whole family is
  now re-exported together, so every path the interface reference advertises
  resolves.
- `by_id` and `by_category` had no caller and no test anywhere in the workspace.
  They are an index-cache implementation of the same lookup `all_indicators()`
  provides by linear scan, so nothing proved the two agreed. A new test asserts
  they return pointer-identical entries from the shared cached slice, that the
  per-category buckets partition the registry exactly (a category typo can no
  longer silently drop an indicator from discovery), and that unknown keys fail
  closed.
- The interface reference had drifted from the code it documents.
  `all_indicators()` was written as returning `Vec<IndicatorInfo>` (it returns
  `&'static [IndicatorInfo]`), `RegistryDocument::indicators` likewise,
  `by_id` / `by_category` / `VALID_CATEGORIES` were missing from the listing
  entirely, `IndicatorInfo` was missing its `streaming` field, and the "valid
  category slugs" line listed 5 of the 15 slugs. All corrected, with a full
  category table and prose for the `convergence` / `streaming` semantics.
- The Chinese API reference documented no registry-discovery surface at all —
  the whole `Indicator Registry API` section existed only in English. It now has
  a matching section, and its table of contents lists the sections in document
  order (`跨市场信号公式` had been listed last while sitting second).
- The Windows MSI script staged the wrong native library names. It copied
  `AlphaTA_ffi.dll` / `.dll.lib` / `.lib`, but the crate is `finkit-ffi` with no
  explicit `[lib]` name, so the artifacts are `finkit_ffi.*`; `copy` does not
  fail a batch script, so the run would have produced an MSI whose `bin\` was
  empty. It also never created the `bin\` directory it copied into, used a
  component-group name that disagreed with `build-installer.sh`, and wrote a
  third spelling of the output filename. All corrected, and the script now
  preflights its inputs.
- No OS-level installer target has ever been buildable from this repository.
  `scripts/build-installer.sh` and `scripts/build-installer-msi.cmd` both hand a
  staged payload to WiX, but the `packaging/wix/Product.wxs` they read has never
  existed in any commit (checked across all branches), and no workflow installs
  the WiX toolset. `scripts/build-usage-packages.sh` is in the same position:
  its `packaging/usage/<lang>/` verification tree is likewise absent, as is the
  `packaging/build-installer.sh` its own RPM changelog refers to. Both installer
  scripts now fail up front with a message naming the missing file, instead of
  dying inside `candle`/`heat` with a tool error. The working release artifact is
  the Python ABI3 wheel, which is what `python-wheels.yml` actually ships.
- `docs/installation.md` §14 told readers to certify a change with
  `cargo test -p finkit --locked` — the same command that left the three
  contract tests in `finkit-ffi-common` / `finkit-ffi` unrun (see above) — and
  listed three of the ten gate scripts. It now gives the full CI-aligned
  per-crate test command, warns about `--no-fail-fast`, and separates the
  docs-check gates from the ci.yml gates. `scripts/check_coverage.py` was
  dropped from that list: it is a coverage *report*, not a CI gate, and is not
  wired into any workflow.
- `MONEY_FLOW` was advertised as streaming-capable in
  `streaming::registry` with no implementation behind it. Added
  `StreamingMoneyFlow` (`streaming::volume::money_flow`), which mirrors
  `indicators::astock::money_flow` bar-for-bar, including its NaN behaviour, and
  is re-exported from `finkit::streaming`.
- The streaming-registry contract gate silently skipped every `IndicatorMeta`
  impl written as `impl crate::streaming::IndicatorMeta for T` (three types:
  `StreamingDema`, `StreamingObv`, `StreamingWclPrice`), so a qualified-path
  impl could drift without ever being checked. The extractor now accepts
  qualified paths.
- The same gate's self-check was tautological: `count_declared_impls()` reused
  the extractor's own regexes, so a form the extractor could not read was also
  uncountable and the two counts agreed on the wrong number. The self-check now
  uses an independent, deliberately looser census.
- `streaming::registry` listed Elder Ray twice — `ELDERRAY` with a 14-period
  default and `Elder Ray` with the correct 13 — and `test_unique_names` compared
  raw strings, so both passed. Consolidated onto the canonical `ELDERRAY` entry
  with the 13-period default (`StreamingElderRay::new(13)`).
- `docs/generated/streaming-indicators.md` was stale after the registry
  corrections; regenerated via `gen_ssot_docs.py --generate`.
- `scripts/optimize_python_bindings.py` rewrote **integer** results to NumPy
  arrays, contradicting `sync_bindings.transform_python_numpy_body`, whose
  documented rule is that integer candlestick results keep the `Vec<T>` ->
  Python list boundary. The mismatch made `sync_bindings.py --generate` a
  non-idempotent operation (it would have introduced 13 wrappers the committed
  bindings never contained) and left `optimize_python_bindings.py --check`
  permanently red. The optimizer now targets floating-point series only, which
  restores `--generate` as a fixed point and turns the check green.
- The same rewriter wrote with `Path.write_text`, which translates every newline
  to `os.linesep`; run on Windows it silently rewrote a whole binding to CRLF.
  It now preserves the file's existing newline convention.
- The 13 hand-written `Vec<f64>` pyfunctions in `ffi/python-binding/src/lib.rs`
  (`dx`, `minus_di`, `minus_dm`, `plus_di`, `plus_dm`, `var`, `ichimoku`,
  `vwap`, `anchored_vwap`, `vwap_bands`, `elder_ray`, `donchian`,
  `pivot_points`) still returned Python lists, while the 39 generated numeric
  functions in the same module returned NumPy arrays and
  `finkit/__init__.pyi` declares `dx`/`var`/`plus_di`/… as `Array1D`. They now
  use the same NumPy-direct wrapper shape, so the native surface is uniform and
  `optimize_python_bindings.py --check` covers `lib.rs` as well as
  `generated.rs`.
- The NumPy-direct binding check never actually ran on a live branch: the only
  workflow that invoked it was `apply-talib-performance-plan.yml`, whose `on:`
  covers `fix/talib-performance-plan-20260904`. Because the check was dead, two
  binding modules kept their list-returning wrappers — `features.rs` (40
  numeric pyfunctions) and `transforms.rs` (5) — so `finkit.features.acf(...)`
  and friends returned Python lists while every top-level numeric function
  returned a NumPy array. Both modules are now NumPy-direct, and the check runs
  in `ci.yml` over **every** binding source, not two hand-listed files.
- The shipped type stub `ffi/python-binding/finkit/__init__.pyi` described an
  API that does not exist. It declared a `StreamingIndicator` base class with a
  `next()` method (the real classes have no base class and the method is
  `update`), declared `cdlhomingsoldier` (the real name is `cdlhomingpigeon`),
  omitted `StreamingMACD` entirely, and froze an `__all__` of 170 names while
  the runtime exported 449. `__all__` is now declared as an annotation so it
  cannot drift again, the streaming section documents the six documented
  classes with their real signatures (`restore_state` is a static method), and
  `scripts/check_python_stub.py` fails the build if the stub declares a name
  the extension does not have.
- Nine streaming classes were registered under their internal Rust struct name,
  so Python users had to reach `finkit.PyStreamingAnchoredVwap`,
  `finkit.PyStreamingPlusDi`, `finkit.PyStreamingStochRsi` and six others while
  the other seventy followed the `Streaming*` convention. They now register as
  `StreamingAnchoredVwap`, `StreamingPlusDi`, `StreamingStochRsi`, … .
- The streaming section of `docs/api-reference-zh.md` documented an API that
  cannot run: `StreamingMACD(fast=…, slow=…)` (the parameters are
  `fast_period`/`slow_period`/`signal_period`), `save()` and
  `from_state()` (the real methods are `save_state()` and the static
  `restore_state(bytes)`), and `macd, signal, hist = update(price)` where
  `update` returns a non-iterable `MACDResult`. `docs/api-reference.md` had no
  Python streaming section at all; both documents now describe the common
  interface and point at the generated catalog.
- Five tests in `ffi/python-binding/tests/` failed against a real wheel, and the
  live `python-wheels.yml` workflow runs them — so the Python release gate was
  red. The compatibility layer that would have fixed them exists only in
  `scripts/apply_perf_plan.py`, whose workflow triggers on
  `fix/perf-contract-talib-parity-20260904`. The layer is now applied for real:
  * `available_factor_libraries()` returned `array(['alpha158',
    'worldquant101'])`. The blanket `_as_numpy_result` adapter used "this list
    contains no containers" as its numeric predicate, which also matches a list
    of *strings*, so `== [...]` raised "truth value of an array is ambiguous".
    It now converts only lists of real numbers (`bool` excluded).
  * `bollinger_bands(matype=1)` and `stoch(slowk_matype=1)` raised a bare
    `ValueError`, so `except finkit.InvalidParameterError` did not catch them.
  * `sar` discarded the acceleration-factor series the engine computes with it,
    leaving no way to reach it from Python; `sar_with_af` now exposes it.
  * `CompiledFormula` had no explicit name for the owned/context-retaining
    evaluation mode; `eval_owned` is now an alias for `eval`.
- `InsufficientDataError` and `InvalidParameterError` inherited only from
  `FinkitError`, and `IndicatorNotFoundError` from `FinkitError` alone, so a
  caller writing `except ValueError` or `except KeyError` missed them. They now
  also inherit from the matching built-in, which is what the documented
  semantic names were always meant to specialise.
- The C-binding export gate was checking roughly one third of the ABI. It
  compared the committed headers against `ffi/c-binding/src/generated.rs` only,
  so the 18 fixed-template entry points in `lib.rs` (`ta_version`,
  `ta_last_error`, `ta_factor_execute_json`, `finkit_free_string`, ...) and the
  3 research functions in `research.rs` were never verified — 21 of the 99
  shipped exports could have disappeared from the Rust side while the header
  kept promising them. `scripts/gen_c_header.py --check` now scans every
  `ffi/c-binding/src/*.rs` and every `ffi/c-binding/include/*.h` and compares in
  both directions, correctly ignoring `#[cfg(test)]`-gated exports such as
  `ta_ffi_panic_test` (deliberately absent from a release build). Cross-checked
  against the shipped DLL's PE export table: 95 `ta_*` plus 4 `finkit_*`
  exports, matching the headers one-for-one.
- That gate had never run in CI either — it was reachable only through
  `make verify-ffi`, which no workflow invokes. It is now a step in `ci.yml`'s
  `binding-ssot` job, alongside the Python binding contracts.
- `scripts/check_python_stub.py` ran its dynamic half whenever *any* `finkit`
  was importable, so on a machine with an older wheel in `site-packages` it
  failed against a build the developer never touched — and, worse, it could
  report success after validating the wrong build. The dynamic check is now
  opt-in (`--require-extension`, or `--expect-prefix` to name the build), and
  `python-wheels.yml` pins the freshly installed wheel with
  `--expect-prefix "$pythonLocation"`.
- Documentation counts that had drifted by one: `docs/indicator_registry.json`
  holds 235 indicators, not 236, so 157 of them have no FFI binding rather than
  158. Corrected in `docs/language-bindings.md` and in the matching comments in
  `scripts/check_coverage.py` and `scripts/sync_bindings.py`.
- The generated API reference had 137 rustdoc diagnostics, all of them silent.
  `cargo doc` exits 0 because rustdoc lints are warn-by-default, and the CI
  `doc` job set `RUSTFLAGS` — but rustdoc reads `RUSTDOCFLAGS`, so nothing was
  denied. The bulk were broken intra-doc links: 62 were bare array indexing in
  prose (`a[i] / b[i]`) that rustdoc read as links to an item named `i`, and
  the module indexes in `math`/`patterns`/`features` used bare sibling names,
  which do not resolve because rustdoc resolves a module's own `//!` links
  against the parent scope. Fixed by escaping the indexing, adding crate-rooted
  paths, and unlinking the targets that are private (`HilbertState`,
  `TaVarianceState`, `ema_into`) or cross-crate (`FfiError`). Two of the
  references were simply stale: `crate::factor_graph::GraphPlan` no longer
  exists (`node_index` lives on `FactorGraphPlan`), and `math::simd_ops`
  documented an AVX2 kernel name that is not in the tree. Eight
  `invalid_html_tags` (`Arc<str>`, `Array1<i32>`) are now code spans. The build
  is clean under `RUSTDOCFLAGS="-D warnings"`.
- `.github/workflows/release-v014-orchestrator.yml` could never run and would
  have failed if it had: its only trigger was a push to `release/v0.1.4`, a
  branch that no longer exists, and its body asserted `VERSION = 0.1.4`, a
  hard-coded commit SHA and a hard-coded workflow run id while the workspace is
  at 0.2.0. Removed rather than left as a release gate that silently never fires.
- Fifteen dead files under `scripts/`, found by the new orphan gate and each
  removed only after establishing that it was superseded, broken, or falsely
  documented: `check-versions.sh`/`.ps1` (the live gate is
  `scripts/check_versions.py`, which the release checklist already names),
  `with-path.sh`, `audit_talib_numeric_contract.mjs`,
  `verify_python_binding.py`, `benchmark_full_coverage.py`,
  `full_accuracy_diagnose.py`, `bench_alpha_vs_talib_python.py`,
  `bench_gate.sh`, `bench_regression_check.sh`, `gen_compat_matrix.py`,
  `gen_competitor_comparison.py`, `gen_performance_report.py`, and
  `generated_samples/{node,python}_indicators.rs`.

## [0.1.15] - 2026-09-11

### Added

- Forming-bar repaint rollback for streaming MACDEXT.
- Scalar WMA, DEMA, TEMA, KAMA, T3, TRIMA, HMA, ALMA, and VIDYA variants for
  streaming MACDEXT fast, slow, and signal lines.
- Configurable streaming MACDEXT exposed through the Python binding.

### Changed

- Close-stream parsing is now injectable, which makes CLI tests deterministic.

## [0.1.14] - 2026-09-11

- Add registered, parameterized custom formula components with nested
  expansion, built-in shadowing protection and bounded expansion limits.
- Expose the same custom formula registry through the Python binding.
- Add explicit custom-edge PSI calculation with finite-value filtering and
  stable outlier handling.
- Add a shared crosshair data-window snapshot and visible-range-aware cursor
  mapping for native/WASM chart frontends.

## [0.1.13] - 2026-09-11

- Correct the Rust CDLDOJI public wrapper to use TA-Lib's default 0.1
  BodyDoji factor, matching the Python binding and compatibility contract.

## [0.1.12] - 2026-09-11

- Complete the maintained TA-Lib 0.6.x compatibility matrix at 161/161 on
  multiple long-input lengths and seeds, including SAR/SAREXT bootstrap
  semantics, T3 warm-up/coefficient behavior and extended candlestick rules.
- Add shared CandleSettings-based thresholds for candlestick compatibility,
  while preserving native full-length and short-input API behavior.
- Correct streaming T3 coefficients and align cycle/trendline compatibility
  paths with the batch implementation.

## [0.1.11] - 2026-09-11

- Add exact TA-Lib compatibility implementations for STOCHRSI, MACDFIX and
  BETA, including their multi-output, lookback and secondary-input contracts.
- Correct the shared Hilbert recursive warm-up and radian/degree period
  conversion; HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, MAMA and the
  trend-mode path now match TA-Lib on the maintained comparison matrix.
- Add the TA-Lib dominant-cycle phase projection and raw-price trendline
  calculation, plus regression coverage for the extended compatibility paths.
- Align CDLDOJI with TA-Lib's preceding real-body average candle setting.

## [0.1.10] - 2026-09-11

- Correct TA-Lib MACDEXT lookback selection for its `slowperiod` and
  `signalperiod` parameter positions in the compatibility adapter.

## [0.1.9] - 2026-09-11

- Add opt-in Python `compute_indicators(..., talib_compat=True)` semantics for
  TA-Lib lookback/NaN conventions, absolute MAX/MININDEX results and compatible
  multi-output ordering without changing native formula behavior.
- Align ADXR, AROONOSC, SAR, PLUS_DM/MINUS_DM and PPO compatibility paths with
  TA-Lib's window, smoothing, output and moving-average-type contracts.
- Correct the full Python comparison harness for TA-Lib 0.6.x multi-output
  finite-ratio accounting and explicit parameter signatures; the maintained
  smoke matrix now calls all 161 public functions and reports 124 exact rows.
- Add Python regression coverage for native-vs-TA-Lib compatibility semantics.

## [0.1.8] - 2026-09-11

- Add a shared formula result metadata contract for dtype, output names,
  NaN/null policy, lookback, warm-up and valid-start semantics across Rust,
  Python, Node and WASM.
- Add a complete 161-function public TA-Lib catalog with explicit runtime
  coverage, category, output-count and compatibility-report fields; unsupported
  and host-required functions are no longer ambiguous.
- Add O(1) append/eval-last formula paths for direct MA, RSI and formula-SMA
  ATR, with exact fallback on mutation or discontinuity.
- Add binding APIs for formula metadata and TA-Lib catalog discovery.
- Preserve separate Wilder/RMA streaming ATR semantics from the formula-layer
  rolling-SMA true-range contract, with differential regression coverage.

## [0.1.7] - 2026-09-11

- Add formula static analysis for dependencies, lookback, future-data risks,
  side effects, stateful nodes and streaming suitability.
- Add terminal semantic profiles and function-level compatibility reports for
  Finkit, TongDaXin, TongHuaShun, EastMoney and Pine.
- Add borrowed range evaluation and Python `eval_range_zero_copy` for chart
  window refreshes without copying complete OHLCV history.
- Add conservative O(1) EMA updates for continuous append/eval-last streams,
  with exact fallback when continuity cannot be proven.
- Expose formula analysis and compatibility APIs through Python, Node and WASM.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-09-09

### Added

- Cross-market screening indicators for crossover, breakout, volume expansion,
  moving-average alignment, relative strength, gaps, and trend confirmation.
- Formula DSL routes and aliases for the new screening indicators.
- Versioned multi-language release workflow documentation and package build
  contracts.

### Performance and validation

- Expanded the TA-Lib release-wheel gate to 155 compatible functions and 310
  observations at 100K/1M values.
- v0.1.5 local validation: 307/310 observations faster, geometric mean 2.03x,
  zero runtime errors; the two existing `HT_TRENDMODE` mask exceptions remain
  explicitly documented.

### Documentation

- Added the cross-market screening API and selection recipes.
- Updated generated indicator/formula/version catalogs and the multi-language
  installation/release guides.
- Replaced stale six-indicator benchmark claims with the current full-matrix
  report and per-observation caveats.

## [0.1.2] - Pending release (source baseline 2026-09-02)

- Added cross-language version checks for .NET and Java binding metadata, with SSOT version-matrix coverage.
- Connected the remaining Android, Go, and .NET generated indicator/chart entrypoints and hardened pre-epoch formula timestamp conversion with regression coverage.

> The `0.1.2` source metadata is ready on the repair branch in PR #13; the GitHub Release and downloadable assets are still pending until CI verification and merge.

### Changed
- 将公式运行时升级为可复用的零拷贝/增量执行模型：公共子表达式合并、`eval_range`/`eval_last`、容量增长式 `append_bar`、持久化 Bytecode/JIT 与缓冲池。
- Python ABI3 wheel 发布矩阵与版本元数据统一为 `0.1.2`，覆盖 CPython 3.8–3.14 的 Linux x86_64、macOS x86_64/arm64 和 Windows x86_64。

### Fixed
- 加固 GitHub Actions 的权限与并发控制，移除旧版 `v0.1.0` 强制回写逻辑，统一使用版本 tag 发布 wheel。

### Added
- Comprehensive CI/CD pipeline with fmt, clippy, and security audits
- Multi-language binding tests (Python, Node.js, Go, .NET, Java, WASM, CLI)
- Complete documentation suite (indicators, installation, API reference, development)
- Cross-platform compilation support (Linux, macOS, Windows)
- Dependency review for pull requests
- **`core/tests/golden_regression.rs`**：30 个黄金 CSV 回归断言（容差 1e-9），覆盖 SMA/EMA/RSI/MACD/ATR/STOCH/ADX/BBANDS/MOM/ROC 等核心指标
- **`core/tests/property_smoke.rs`**：3 个 proptest 属性测试 — SMA 线性、Bollinger 上下界包络、RSI 范围 [0,100]
- **`core/benches/zero_alloc_bench.rs`**：SIMD 路径 `n=10_000` 零分配验证（`fma_avx2` vs `scalar_fma`）
- **`streaming::registry` 额外缓存**：`by_category()` / `by_id()` 改 `OnceLock<HashMap<...>>` 缓存，与 `all_indicators` 保持一致
- **`core/benches/watchlist_self_bench.rs`**：9 个 watchlist 指标（AROON/WILLR/WMA/KAMA/MFI/STOCHF/AD/ADOSC/OBV）的自基准测试，3 档数据规模（1K/10K/100K）
- **`watchlist_self_bench` 注册到 `Cargo.toml`**：纳入 cargo bench 体系

### Changed
- **Crate-wide `#[allow(missing_docs)]`**：在 `core` / `visualization` / `cli` / 5 个 FFI binding / `wasm` 的 `lib.rs` 顶部集中抑制 internal helper 的 missing_docs warning（净减 1700+ pedantic warnings，cargo check 0 warnings）
- **`StreamingPpo` / `StreamingTsi` / `StreamingApo` / `StreamingCoppock` / `StreamingNatr`**：移出 `wasm_streaming_f64!` 宏，补独立 wrapper（PPO/TSI/APO 2 参构造、Coppock 3 参构造、NATR `&dyn Ohlcv` 入参）
- **`StreamingBoll` / `StreamingStoch` 构造签名**：wasm 入口补第 3 参数（`nb_dev_dn` / `k_slow`），与 core 对齐
- **`aroon` 阈值分流**：`period ≤ 16` 走 `aroon_scan`（线性扫描，cache 友好），`period > 16` 走 `aroon_with_deques`（单调双端队列，O(1) amortized）。消除原 O(period) rescan 慢路径
- **`willr` 单调双端队列**：完全重写为 max-deque + min-deque，热路径 O(1) amortized，删除 rescan 内层循环
- **`stochf` 单调双端队列**：max-deque 替代 rescan；保留 ring-buffered %D 累加器，输出流水线不变
- **`docs/BENCHMARK_REPORT.md` watchlist 章节**：补充 9 指标吞吐量表 + 算法形态分类 + 下一步 SIMD 优化建议

### Fixed
- **dotnet-binding & java-binding 内存管理 README 文档契约** 已在 `ffi/dotnet-binding/README.md` 与 `ffi/java-binding/README.md` 完善
- **`simd_kernels.rs` 两处死赋值**：`highest = *high_ptr.add(ws);` 与 `lowest = *low_ptr.add(ws);` 立即被覆盖，删除
- **`pca.rs::cov_idx` 死方法**：未引用，删除
- **dotnet-binding & java-binding CString UB**：为两者增加 `ta_free_cstring` / `freeJString` 配对释放函数；引入 `serde_json` 替代 36+ 处手写 JSON 序列化（DrawCommand / DebugEvent / FormulaTemplate 等核心类型现在 derive `serde::Serialize`）
- **FormulaCache 真 LRU**：从 O(n) 最小 counter 扫描改为 `lru` crate 的 O(1) LruCache，公开 API 完全兼容
- **JIT 路径 `load_variable` 零分配**：从 `name.to_uppercase().as_str()` 链式匹配改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **SIMD feature detection 缓存**：用 `std::sync::OnceLock<SimdLevel>` 缓存 CPUID 结果，18 个 public SIMD 入口改为 `match simd_level()`，100 万次 add 仅触发 1 次 CPUID
- **BufferPool 双池清理**：删除未使用的 legacy `VecDeque<Vec<f64>>` 字段与 8 个 API（`acquire/release/shrink_to/...`），构造 BufferPool 不再预分配 64KB `Vec<f64>`
- **`resolve_variable_zero_copy` 6 段重复代码抽取**：Close/High/Low/Open/Volume/Amount 6 段近似重复逻辑抽取为单一 `copy_view_to_pool` helper
- **Parser 别名表零分配**：`parse_variable` 中 16 个 C1/O1/CLOSE1 等别名从 `to_uppercase().as_str()` 改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **streaming `all_indicators` OnceLock 缓存**：避免重复调用时重复构造
- **FormulaCache 单次查找**：`get_cloned` 与 `insert` 中删除 `contains_key + get/get_mut` 双查找
- **workspace 构建配置精细化**：`[profile.release]` 改为 `lto="thin", strip="debuginfo"`；`[profile.release.package."alpha-ta-..."] codegen-units = 16`；`[profile.dev] opt-level = 1, debug = 1`；新增 workspace lints（`unsafe_op_in_unsafe_fn = "warn"`, `missing_debug_implementations = "warn"`, `clippy::pedantic`）
- **Go 绑定版本硬编码 bug**：`ta_version()` 从硬编码 `0.1.0` 改为 `env!("CARGO_PKG_VERSION")`
- **Java 绑定 panic 保护**：为公式评估函数添加 `catch_unwind` 包裹和 `RuntimeException` 错误传播

## [1.0.0] - 2026-06-24

### Added
- Hilbert Transform cycle indicators (HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDMODE)
- Statistics indicators (STDDEV, VAR, LINEARREG, ZSCORE, CORREL)
- Price transform functions (AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE)
- Volume indicators (AD, ADOSC, CMF)
- Pattern recognition for 60+ candlestick patterns
- Chart pattern detection (Double Top/Bottom, Head & Shoulders)
- Go binding with CGO support
- .NET binding with P/Invoke
- Java binding with JNI support
- CLI tool for command-line analysis
- WebAssembly module for browser usage
- Visualization module
- Formula engine with AST parsing, bytecode compilation, JIT optimization, and SIMD acceleration
- Streaming (incremental) indicator framework with 150+ indicators
- Feature engineering module for ML label generation

### Changed
- Improved performance of SMA and EMA calculations
- Enhanced error handling with detailed error messages
- Updated Python binding to use latest PyO3 features
- Optimized memory allocation in core library

### Fixed
- Fixed RSI calculation for edge cases with constant values
- Fixed MACD signal line initialization
- Fixed pattern detection boundary conditions

## [0.3.0] - 2024-XX-XX

### Added
- Python binding with PyO3
- Node.js binding with NAPI-RS
- Support for all major overlap indicators (SMA, EMA, WMA, DEMA, TEMA, KAMA, BBANDS, SAR)
- Momentum indicators (RSI, MACD, STOCH, ADX, AROON, CCI, WILLR)
- Volatility indicators (ATR, NATR, TRANGE)
- Basic candlestick pattern recognition

### Changed
- Refactored moving average calculations for better performance
- Improved test coverage for all indicators

### Fixed
- Fixed EMA calculation precision issues
- Fixed Bollinger Bands upper/lower band calculation

## [0.2.0] - 2024-XX-XX

### Added
- Initial release of FTA core library
- Basic technical analysis framework
- Moving average implementations (SMA, EMA)
- Core mathematical functions
- Rust API design
- Multi-platform support (Linux, macOS, Windows)
- CI/CD pipeline setup
- Comprehensive test suite

