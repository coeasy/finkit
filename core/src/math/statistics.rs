use crate::error::{Result, TaError};
use crate::math::kernels::{
    rolling_mean_into, rolling_sample_stddev_into, rolling_sample_variance_into, KernelCompatError,
};
use crate::math::rank::fractional_ranks;
use ndarray::Array1;

fn map_kernel_error(error: KernelCompatError) -> TaError {
    match error {
        KernelCompatError::InvalidWindow(window) => TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: format!("greater than 0 (got {window})"),
        },
        KernelCompatError::LengthMismatch { input, output } => TaError::ComputationError {
            message: format!("rolling kernel length mismatch: input={input}, output={output}"),
        },
        KernelCompatError::PairLengthMismatch { left, right } => TaError::ComputationError {
            message: format!("rolling pair kernel length mismatch: left={left}, right={right}"),
        },
        KernelCompatError::OhlcLengthMismatch => TaError::ComputationError {
            message: "rolling statistics received mismatched OHLC lengths".to_string(),
        },
    }
}

/// Calculate arithmetic mean
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The mean value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::mean(&data).unwrap();
/// assert_eq!(result, 5.5);
/// ```
pub fn mean(data: &[f64]) -> Result<f64> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    Ok(data.iter().sum::<f64>() / data.len() as f64)
}

/// Calculate variance (sample variance with Bessel's correction)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The variance value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::variance(&data).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn variance(data: &[f64]) -> Result<f64> {
    if data.len() < 2 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 2,
        });
    }
    let m = mean(data)?;
    let sum_sq: f64 = data.iter().map(|x| (x - m).powi(2)).sum();
    Ok(sum_sq / (data.len() - 1) as f64)
}

/// Calculate standard deviation (sample)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The standard deviation value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::std_dev(&data).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn std_dev(data: &[f64]) -> Result<f64> {
    variance(data).map(|v| v.sqrt())
}

/// Calculate covariance between two series (sample)
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series
///
/// # Returns
/// The covariance value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
/// let result = statistics::covariance(&x, &y).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn covariance(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x and y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if x.len() < 2 {
        return Err(TaError::InsufficientData {
            length: x.len(),
            required: 2,
        });
    }

    let n = x.len() as f64;

    // Sample covariance from the *centred* moments. The one-pass form
    // `sum_xy - sum_x*sum_y/n` subtracts two large nearly-equal totals and loses
    // every significant digit when the mean dwarfs the spread; see
    // [`crate::math::centred_moments`]. Measured on `x = 1e9 + i`,
    // `y = 2x + 3`: the one-pass form returned `520.1` where the exact answer is
    // `693.3`.
    let (cross, _, _) = crate::math::centred_moments(x, y);
    Ok(cross / (n - 1.0))
}

/// Calculate Pearson correlation coefficient
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series
///
/// # Returns
/// Correlation coefficient in range [-1, 1]
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
/// let result = statistics::correlation(&x, &y).unwrap();
/// assert!((result - 1.0).abs() < 1e-10);
/// ```
pub fn correlation(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x and y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if x.len() < 2 {
        return Err(TaError::InsufficientData {
            length: x.len(),
            required: 2,
        });
    }

    // Centred moments rather than the one-pass `sum_sq - sum^2/n` form: the
    // latter reports `r = 0.667` for a series and its exact affine image
    // `2x + 3` at a `1e9` baseline. See [`crate::math::centred_moments`]. The
    // returned variances are sums of *squared deviations*, so they are
    // non-negative and `(var_x * var_y).sqrt()` cannot produce a `NaN`.
    let (cov, var_x, var_y) = crate::math::centred_moments(x, y);

    if var_x < 1e-15 || var_y < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero for one or both series".to_string(),
        });
    }

    Ok(cov / (var_x * var_y).sqrt())
}

/// Calculate rolling mean over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling mean values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_mean(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_mean(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let mut output = vec![f64::NAN; data.len()];
    rolling_mean_into(data, window, &mut output).map_err(map_kernel_error)?;
    Ok(Array1::from_vec(output))
}

/// Calculate rolling variance over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling variance values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_variance(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_variance(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "at least 2".to_string(),
        });
    }

    let mut output = vec![f64::NAN; data.len()];
    rolling_sample_variance_into(data, window, &mut output).map_err(map_kernel_error)?;
    Ok(Array1::from_vec(output))
}

/// Calculate rolling standard deviation over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling standard deviation values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_std_dev(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_std_dev(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "at least 2".to_string(),
        });
    }

    // Delegate to the canonical kernel rather than `rolling_variance(..).map(sqrt)`.
    // The map form applied `sqrt` to an *unclamped* variance, so a window whose
    // true variance is zero but whose removable-Welford residue came out
    // slightly negative produced `NaN` instead of `0.0` — while the canonical
    // kernel (`features::rolling_std_simd`, `StreamingStddev`) clamped and
    // returned `0.0`. Two spellings of "rolling sample standard deviation" must
    // not answer differently on the same window.
    let mut output = vec![f64::NAN; data.len()];
    rolling_sample_stddev_into(data, window, &mut output).map_err(map_kernel_error)?;
    Ok(Array1::from_vec(output))
}

/// Calculate skewness of a data series
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// Skewness value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::skewness(&data).unwrap();
/// assert!(result.is_finite());
/// ```
pub fn skewness(data: &[f64]) -> Result<f64> {
    if data.len() < 3 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 3,
        });
    }

    let n = data.len() as f64;
    let m = mean(data)?;
    let s = std_dev(data)?;

    if s.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero".to_string(),
        });
    }

    let sum_cubed: f64 = data.iter().map(|x| ((x - m) / s).powi(3)).sum();

    Ok((n / ((n - 1.0) * (n - 2.0))) * sum_cubed)
}

/// Calculate kurtosis of a data series (excess kurtosis)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// Kurtosis value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::kurtosis(&data).unwrap();
/// assert!(result.is_finite());
/// ```
pub fn kurtosis(data: &[f64]) -> Result<f64> {
    if data.len() < 4 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 4,
        });
    }

    let n = data.len() as f64;
    let m = mean(data)?;
    let s = std_dev(data)?;

    if s.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero".to_string(),
        });
    }

    let sum_fourth: f64 = data.iter().map(|x| ((x - m) / s).powi(4)).sum();

    let k = ((n * (n + 1.0)) / ((n - 1.0) * (n - 2.0) * (n - 3.0))) * sum_fourth;
    let correction = (3.0 * (n - 1.0).powi(2)) / ((n - 2.0) * (n - 3.0));

    Ok(k - correction)
}

/// Window size at or below which the cached-index strategy is preferred.
///
/// The cached index spends one comparison per bar in the common case but up to
/// `window` steps whenever the cached position expires; the monotonic ring pays
/// one insertion and one removal per bar no matter what. Capping the window keeps
/// that worst case from running away on pathological inputs such as a strictly
/// monotone series, while every TA-Lib window in the catalogue stays below the
/// cap. Both strategies answer missing bars the same way, so the cap is the only
/// thing that selects a path.
const EXTREMA_CACHE_LIMIT: usize = 512;

/// Window size at or below which the block kernel's two tables live on the
/// stack instead of the heap.
///
/// The block algorithm needs a suffix table and a prefix table, each `window`
/// entries, and both are written and read strictly inside one call. Every
/// catalogue window is small, so a fixed stack buffer covers the common case
/// and removes two allocator round trips per call.
///
/// Measured, not assumed. `core/examples/talib_gap_probe.rs` section B duels
/// the two tiers directly — same body, `vec![0.0; window]` against
/// `[0.0; 64]` — at window 30 over 10,000 bars and reads the stack tier at
/// **0.870x the heap tier's time**. The same section also tried a monotonic
/// index ring as a third tier and **rejected** it at 0.213x: each bar pays a
/// data-dependent inner loop that the block kernel amortises away. Do not
/// rebuild the ring.
///
/// The cap is a table-size choice, not a correctness one: the heap tier is the
/// same body over a `window`-sized allocation.
const VAN_HERK_STACK_TABLES: usize = 64;

/// Index of the first finite bar, `None` when the series holds no finite bar.
///
/// The warm-up contract of the extrema family is "the window must be fully
/// populated", and a non-finite bar is the only way a bar goes unpopulated. So
/// a leading run of missing bars is skipped outright rather than emitted as
/// partial-window extremes: the first report sits at `leading_run + window - 1`,
/// which is what `MA(X, 9)`, `SUM(X, 9)`, `STD(X, 9)` all do and what the
/// composition gates assert.
#[inline]
fn first_finite(values: &[f64]) -> Option<usize> {
    values.iter().position(|value| !value.is_nan())
}

/// Rescan `values[start..=end]` for the trailing window's extreme, dropping
/// missing bars. Returns `(extreme, position, found_a_finite_bar)`; the third
/// component is the only way to tell "no finite bar in this window" apart from
/// "the extreme is very large", so it must be carried out with the value.
#[inline]
#[inline(always)]
pub(crate) fn rescan_extreme_window<const WANT_MAX: bool>(
    values: &[f64],
    start: usize,
    end: usize,
) -> (f64, usize, bool) {
    let mut best = if WANT_MAX {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_index = start;
    let mut found = false;

    // The `is_nan` / `found` guards the previous form evaluated per candidate
    // are subsumed by the seed: against `NEG_INFINITY` (or `INFINITY`) a `NaN`
    // fails `>=` (or `<=`) and is skipped for free, and the first finite bar
    // wins outright, so `found` only records that some bar did win. Keeping
    // `>=` / `<=` rather than a strict test preserves the tie rule callers
    // depend on: the *newest* tied bar holds the position, which matters
    // wherever the index — not just the value — is reported (AROONOSC).
    //
    // The scan is unrolled four-wide on purpose: TA-Lib's `TA_AROON` rescan
    // runs the same dominance test under `TA_UNROLL(4)`, which keeps the branch
    // off the loop's back-edge and lets the four comparisons run back-to-back.
    // A plain `for` leaves AROON ~17% behind TA-Lib on the 10k series; the
    // unroll closes most of that. The tail handles the `period % 4` remainder.
    let mut index = start;
    while index + 3 <= end {
        let c0 = values[index];
        let w0 = if WANT_MAX { c0 >= best } else { c0 <= best };
        if w0 {
            best = c0;
            best_index = index;
            found = true;
        }
        let c1 = values[index + 1];
        let w1 = if WANT_MAX { c1 >= best } else { c1 <= best };
        if w1 {
            best = c1;
            best_index = index + 1;
            found = true;
        }
        let c2 = values[index + 2];
        let w2 = if WANT_MAX { c2 >= best } else { c2 <= best };
        if w2 {
            best = c2;
            best_index = index + 2;
            found = true;
        }
        let c3 = values[index + 3];
        let w3 = if WANT_MAX { c3 >= best } else { c3 <= best };
        if w3 {
            best = c3;
            best_index = index + 3;
            found = true;
        }
        index += 4;
    }
    while index <= end {
        let candidate = values[index];
        let wins = if WANT_MAX {
            candidate >= best
        } else {
            candidate <= best
        };
        if wins {
            best = candidate;
            best_index = index;
            found = true;
        }
        index += 1;
    }
    (best, best_index, found)
}

/// Cached-index rolling extrema with NaN-transparent comparison.
///
/// Missing bars are **dropped**: a `NaN` never holds the extreme and never
/// blocks a finite bar behind it, and a window whose bars are all missing
/// reports `NaN`. This is the same contract [`rolling_median_into`] documents,
/// so the extrema family no longer poisons a window merely because one of its
/// bars happened to be absent.
///
/// The running extreme and its position are carried across bars and the window
/// is only rescanned when that position leaves it. Expiry must be tested even on
/// a missing bar: the bar itself never holds the extreme, but a missing bar can
/// still push the cached extreme out of the window. `has_finite` distinguishes
/// "no finite bar yet seen" from "the extreme is very large"; without it a
/// rescan that finds nothing would leave a stale value to be emitted.
///
/// Ties deliberately keep the **newest** position, which defers the next rescan;
/// reported values are unaffected because tied bars hold equal values.
///
/// `best` starts at the side's infinity so the first `window` bars are absorbed
/// by the fast path alone and the first emission is already the true window
/// extreme. `warm_at` is the first index that may report: `window - 1` for a
/// finite first bar, later when a leading run of missing bars was skipped, so a
/// partially populated window is never mistaken for a full one.
#[inline]
fn rolling_extreme_cached<const WANT_MAX: bool>(
    data: &[f64],
    window: usize,
    warm_at: usize,
    output: &mut [f64],
) {
    let mut best = if WANT_MAX {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_index = 0usize;
    let mut has_finite = false;
    let source = data.as_ptr();
    let target = output.as_mut_ptr();

    // The update is branch-lean on purpose. `NaN` fails every comparison, so a
    // missing bar falls through the dominance test with no explicit NaN check,
    // and the expiry test only runs when the incoming bar failed to take the
    // extreme (a bar that does take it is never out of the window).
    macro_rules! track {
        ($i:expr) => {{
            // SAFETY: `$i < data.len() == output.len()` (callers keep the two
            // in step) and both pointers are derived from slices that outlive
            // the loop.
            let value = unsafe { *source.add($i) };
            let dominates = if WANT_MAX {
                value >= best
            } else {
                value <= best
            };
            if dominates {
                best = value;
                best_index = $i;
                has_finite = true;
            } else if has_finite && best_index + window <= $i {
                // The cached extreme left the window: rescan the window from
                // its left edge, dropping the bars that are missing.
                let (rescanned, position, found) =
                    rescan_extreme_window::<WANT_MAX>(data, $i + 1 - window, $i);
                best = rescanned;
                best_index = position;
                has_finite = found;
            }
        }};
    }

    // Warm phase: the extrema are tracked but nothing is emitted yet, so the
    // per-bar emit gate of the old single loop disappears.
    let warm_end = warm_at.min(data.len());
    for i in 0..warm_end {
        track!(i);
    }
    // Reporting phase: same update plus one unconditional store per bar.
    for i in warm_end..data.len() {
        track!(i);
        // SAFETY: `i < output.len()`, see the invariant above.
        unsafe { *target.add(i) = if has_finite { best } else { f64::NAN } };
    }
}

/// Van Herk–Gil–Werman rolling extreme for **fully finite** input.
///
/// The cached-index kernel is amortized O(1)/bar, but its rescan fires whenever
/// the cached extreme leaves the window — on noisy series that is most bars, so
/// the observed cost degenerates to one full-window scan per bar and loses to a
/// plain C loop. The block algorithm keeps ~3 comparisons per bar *regardless
/// of the data distribution*: every window is the extreme of a suffix of the
/// older block combined with the extreme of a prefix of the newer one, both
/// precomputed once per block.
///
/// Callers dispatch here only after an `is_finite` prescan, so the missing-bar
/// contract is moot: entries `0..window - 1` are left untouched (the caller's
/// warm-up fill stays) and every later entry is written with the exact window
/// extreme — the same values the cached-index kernel would produce bit for bit.
/// Infinites would break that equivalence, hence the stricter prescan.
pub(crate) fn van_herk_extreme_into<const WANT_MAX: bool>(
    data: &[f64],
    window: usize,
    output: &mut [f64],
) {
    let len = data.len();
    if window <= 1 || window > len || output.len() != len {
        return;
    }

    // See `VAN_HERK_STACK_TABLES`: every catalogue window fits the stack tier,
    // so the heap tier is the long tail rather than the common case.
    if window <= VAN_HERK_STACK_TABLES {
        let mut suffix = [0.0f64; VAN_HERK_STACK_TABLES];
        let mut prefix = [0.0f64; VAN_HERK_STACK_TABLES];
        van_herk_body::<WANT_MAX>(data, window, output, &mut suffix, &mut prefix);
    } else {
        let mut suffix = vec![0.0f64; window];
        let mut prefix = vec![0.0f64; window];
        van_herk_body::<WANT_MAX>(data, window, output, &mut suffix, &mut prefix);
    }
}

/// The block body, shared by both table tiers.
///
/// Split out so the stack and heap tiers cannot drift: they differ only in
/// where `suffix`/`prefix` come from, and the measured win (0.870x) is produced
/// by the storage alone — the probe's two variants are otherwise identical
/// statement for statement.
fn van_herk_body<const WANT_MAX: bool>(
    data: &[f64],
    window: usize,
    output: &mut [f64],
    suffix: &mut [f64],
    prefix: &mut [f64],
) {
    let len = data.len();
    debug_assert!(
        suffix.len() >= window && prefix.len() >= window,
        "block tables must hold at least `window` entries"
    );

    // SAFETY: every indexed read stays within `data` (offsets are bounded by
    // `len` checks in the loop) and every write stays within `output`, which
    // has the same length; both slices outlive the loop. `suffix[o]` and
    // `prefix[o]` are indexed with `o < window`, and both tables hold at least
    // `window` entries — the assertion above covers the stack tier, where the
    // buffer is `VAN_HERK_STACK_TABLES` long, and the heap tier allocates
    // exactly `window`.
    let source = data.as_ptr();
    let target = output.as_mut_ptr();

    unsafe {
        let mut block_start = 0usize;
        let mut today = window - 1;
        while today < len {
            let block_end = block_start + window - 1;

            // suffix[o] = extreme of data[block_start + o ..= block_end].
            let mut extreme = *source.add(block_end);
            suffix[window - 1] = extreme;
            let mut offset = window - 1;
            while offset > 0 {
                offset -= 1;
                let value = *source.add(block_start + offset);
                let takes = if WANT_MAX {
                    value > extreme
                } else {
                    value < extreme
                };
                if takes {
                    extreme = value;
                }
                suffix[offset] = extreme;
            }

            // The bar at `block_end` is covered by the suffix table alone.
            *target.add(today) = suffix[0];

            let block_next = block_start + window;
            if block_next >= len {
                break;
            }
            let n_available = (len - block_next).min(window - 1);

            // prefix[o] = extreme of data[block_next ..= block_next + o].
            extreme = *source.add(block_next);
            prefix[0] = extreme;
            let mut grown = 1usize;
            while grown < n_available {
                let value = *source.add(block_next + grown);
                let takes = if WANT_MAX {
                    value > extreme
                } else {
                    value < extreme
                };
                if takes {
                    extreme = value;
                }
                prefix[grown] = extreme;
                grown += 1;
            }

            // Windows crossing the block boundary: suffix covers the older
            // part, prefix the newer one.
            let mut offset = 1usize;
            while offset <= n_available {
                let prefix_extreme = prefix[offset - 1];
                let suffix_extreme = suffix[offset];
                *target.add(today + offset) = if WANT_MAX {
                    if prefix_extreme > suffix_extreme {
                        prefix_extreme
                    } else {
                        suffix_extreme
                    }
                } else if prefix_extreme < suffix_extreme {
                    prefix_extreme
                } else {
                    suffix_extreme
                };
                offset += 1;
            }

            block_start += window;
            today += n_available + 1;
        }
    }
}

/// Fused high/low form of [`rolling_extreme_cached`].
///
/// The two legs warm up independently: `warm_high_at` and `warm_low_at` may
/// differ when only one leg opens with a missing run, which keeps each extreme
/// from being reported from a partially populated window.
#[inline]
fn rolling_minmax_cached(
    high: &[f64],
    low: &[f64],
    window: usize,
    warm_high_at: usize,
    warm_low_at: usize,
    mut emit: impl FnMut(usize, f64, f64),
) {
    let mut highest = f64::NEG_INFINITY;
    let mut highest_index = 0usize;
    let mut has_high = false;
    let mut lowest = f64::INFINITY;
    let mut lowest_index = 0usize;
    let mut has_low = false;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();

    // Branch-lean update, mirroring `rolling_extreme_cached`: `NaN` fails every
    // comparison and so never takes an extreme, and each leg's expiry is only
    // tested after its dominance test failed.
    macro_rules! track {
        ($i:expr) => {{
            // SAFETY: `$i < high.len() == low.len()` and both pointers come
            // from slices that outlive the loop.
            let candidate_high = unsafe { *high_ptr.add($i) };
            let candidate_low = unsafe { *low_ptr.add($i) };

            if candidate_high >= highest {
                highest = candidate_high;
                highest_index = $i;
                has_high = true;
            } else if has_high && highest_index + window <= $i {
                let (rescanned, position, found) =
                    rescan_extreme_window::<true>(high, $i + 1 - window, $i);
                highest = rescanned;
                highest_index = position;
                has_high = found;
            }

            if candidate_low <= lowest {
                lowest = candidate_low;
                lowest_index = $i;
                has_low = true;
            } else if has_low && lowest_index + window <= $i {
                let (rescanned, position, found) =
                    rescan_extreme_window::<false>(low, $i + 1 - window, $i);
                lowest = rescanned;
                lowest_index = position;
                has_low = found;
            }
        }};
    }

    // Warm-up split: while either leg is still short of its warm point nothing
    // is emitted. Both warm points then coincide with plain tracking.
    let tracked_end = warm_high_at.min(warm_low_at).min(high.len());
    let emit_from = warm_high_at.max(warm_low_at).min(high.len());
    for i in 0..tracked_end {
        track!(i);
    }
    for i in tracked_end..emit_from {
        track!(i);
    }
    for i in emit_from..high.len() {
        track!(i);
        emit(
            i,
            if has_high { highest } else { f64::NAN },
            if has_low { lowest } else { f64::NAN },
        );
    }
}

/// Visit fused rolling maximum/minimum values without materializing extrema arrays.
///
/// Architecture v3.1 consumers share this kernel across MIDPOINT, MIDPRICE,
/// WILLR and other extrema-family consumers. Both strategies are NaN-transparent
/// and give the same answer for every input, so no gating is needed to keep
/// them in agreement:
///
/// * **Cached index** — `window <= EXTREMA_CACHE_LIMIT`. One comparison per side
///   per bar, rescanning only when an extrema leaves the window. This is what
///   makes the TA-Lib C `TA_MAX`/`TA_MIN` fast path fast.
/// * **Monotonic ring** — oversized windows. Every bar is inserted and removed
///   at most once, which bounds the worst case the cached index would otherwise
///   pay in rescans.
///
/// Missing bars are dropped by both: neither can hold an extrema, and a window
/// whose bars are all missing emits `NaN`.
#[inline]
pub(crate) fn rolling_minmax_visit(
    high: &[f64],
    low: &[f64],
    window: usize,
    emit: impl FnMut(usize, f64, f64),
) {
    debug_assert_eq!(high.len(), low.len());
    debug_assert!(window > 0);

    if high.is_empty() || high.len() != low.len() || window == 0 {
        return;
    }

    // Each leg warms up from its own first finite bar; see `first_finite`.
    let warm_high_at = first_finite(high).map_or(high.len(), |start| start + window - 1);
    let warm_low_at = first_finite(low).map_or(low.len(), |start| start + window - 1);

    if window <= EXTREMA_CACHE_LIMIT {
        rolling_minmax_cached(high, low, window, warm_high_at, warm_low_at, emit);
        return;
    }

    rolling_minmax_ring(high, low, window, warm_high_at, warm_low_at, emit);
}

/// Monotonic-queue form of [`rolling_minmax_visit`], for windows above
/// [`EXTREMA_CACHE_LIMIT`].
///
/// Two queues hold the surviving candidates for each leg. Every bar is inserted
/// and removed at most once, so the total work is linear in the series regardless
/// of window size — the bound the cached index cannot offer on strictly monotone
/// input. Missing bars are never queued, which is why an empty queue at read
/// time means the window holds nothing finite.
fn rolling_minmax_ring(
    high: &[f64],
    low: &[f64],
    window: usize,
    warm_high_at: usize,
    warm_low_at: usize,
    mut emit: impl FnMut(usize, f64, f64),
) {
    let mut high_queue: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(window);
    let mut low_queue: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(window);

    for i in 0..high.len() {
        let new_high = high[i];
        if !new_high.is_nan() {
            push_extreme_candidate::<true>(&mut high_queue, high, i, new_high);
        }

        let new_low = low[i];
        if !new_low.is_nan() {
            push_extreme_candidate::<false>(&mut low_queue, low, i, new_low);
        }

        expire_extreme_candidates(&mut high_queue, window, i);
        expire_extreme_candidates(&mut low_queue, window, i);

        if i >= warm_high_at && i >= warm_low_at {
            let high_value = high_queue
                .front()
                .map(|&front| high[front])
                .unwrap_or(f64::NAN);
            let low_value = low_queue
                .front()
                .map(|&front| low[front])
                .unwrap_or(f64::NAN);
            emit(i, high_value, low_value);
        }
    }
}
/// Find maximum value in a rolling window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling maximum values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_max(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_max(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    rolling_max_into(
        data,
        window,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    );
    Ok(output)
}

/// Insert `index` into a monotonic queue of candidates, first dropping every tail
/// that `value` dominates. `WANT_MAX` selects a decreasing queue (maximums) or an
/// increasing one (minimums).
///
/// Shared by the single-series and fused high/low ring paths so the two can never
/// disagree about which bars are evictable.
#[inline]
fn push_extreme_candidate<const WANT_MAX: bool>(
    queue: &mut std::collections::VecDeque<usize>,
    values: &[f64],
    index: usize,
    value: f64,
) {
    while let Some(&back) = queue.back() {
        let dominated = if WANT_MAX {
            values[back] <= value
        } else {
            values[back] >= value
        };
        if dominated {
            queue.pop_back();
        } else {
            break;
        }
    }
    queue.push_back(index);
}

/// Expire every queued candidate whose bar has left the trailing window.
#[inline]
fn expire_extreme_candidates(
    queue: &mut std::collections::VecDeque<usize>,
    window: usize,
    i: usize,
) {
    while queue
        .front()
        .is_some_and(|&front| front.saturating_add(window) <= i)
    {
        queue.pop_front();
    }
}

/// Write the rolling maximum into a caller-owned buffer of the same length.
///
/// Entries `0..window - 1` are left untouched so callers keep their own warm-up
/// fill. Missing bars are dropped: a `NaN` is never the window maximum, and a
/// window of all-`NaN` bars reads back as `NaN`.
#[inline]
pub(crate) fn rolling_max_into(data: &[f64], window: usize, output: &mut [f64]) {
    fill_rolling_extreme::<true>(data, window, output);
}

/// Write the rolling minimum into a caller-owned buffer of the same length.
///
/// See [`rolling_max_into`] for the warm-up and missing-value contract.
#[inline]
pub(crate) fn rolling_min_into(data: &[f64], window: usize, output: &mut [f64]) {
    fill_rolling_extreme::<false>(data, window, output);
}

#[inline]
fn fill_rolling_extreme<const WANT_MAX: bool>(data: &[f64], window: usize, output: &mut [f64]) {
    if window == 0 || window > data.len() || output.len() != data.len() {
        return;
    }
    if window == 1 {
        output.copy_from_slice(data);
        return;
    }

    // The window is not full until `window` bars are in, and a missing bar does
    // not fill it: skip a leading run of missing bars before reporting.
    let warm_at = first_finite(data).map_or(data.len(), |start| start + window - 1);

    // Contract for callers that hand over an uninitialized buffer: every slot
    // from `window - 1` on is written by this call. The block kernel honours it
    // directly; the cached kernels stop at `warm_at`, which a leading missing
    // run can push past `window - 1`, so that gap is filled here. It is at most
    // the leading missing run long, not a full pass over the series.
    if warm_at > window - 1 {
        let end = warm_at.min(output.len());
        output[window - 1..end].fill(f64::NAN);
    }

    if window <= EXTREMA_CACHE_LIMIT {
        // Fully finite input is the common case for price series, and it is the
        // regime where the cached-index kernel's rescan degenerates to a full
        // window scan on most bars. The block algorithm's cost is independent
        // of the data distribution, so it wins exactly where the cache loses.
        // The test is `is_finite`, not `!is_nan`: a leading non-finite run is
        // warm-up (see `warm_at` above), and the block kernel has no warm-up
        // concept — it would report an `inf` bar the cached kernel skips.
        if data.iter().all(|value| value.is_finite()) {
            van_herk_extreme_into::<WANT_MAX>(data, window, output);
        } else {
            rolling_extreme_cached::<WANT_MAX>(data, window, warm_at, output);
        }
    } else {
        rolling_extreme_ring::<WANT_MAX>(data, window, warm_at, output);
    }
}

/// `VecDeque` monotonic queue used for windows above [`EXTREMA_CACHE_LIMIT`],
/// where the guaranteed-one-removal-per-bar bound beats the cached index's
/// rescan cost. NaN bars are never queued, so a window with no finite bar reads
/// back as `NaN` — the same contract as the cached-index strategy.
fn rolling_extreme_ring<const WANT_MAX: bool>(
    data: &[f64],
    window: usize,
    warm_at: usize,
    output: &mut [f64],
) {
    let mut deque: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(window);

    for i in 0..data.len() {
        let value = data[i];
        if !value.is_nan() {
            push_extreme_candidate::<WANT_MAX>(&mut deque, data, i, value);
        }

        expire_extreme_candidates(&mut deque, window, i);

        if i >= warm_at {
            output[i] = match deque.front() {
                Some(&front) => data[front],
                // The window holds no finite bar.
                None => f64::NAN,
            };
        }
    }
}

/// Find minimum value in a rolling window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling minimum values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_min(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_min(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    rolling_min_into(
        data,
        window,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    );
    Ok(output)
}

/// Rolling median over a trailing window of `window` observations.
///
/// The single implementation behind the formula surface's `MEDIAN` and the
/// compiled-plan `CALL:MEDIAN` kernel. Missing values are dropped before the
/// median is taken, so a window that is entirely NaN yields NaN rather than
/// zero; bars before the first full window are NaN.
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling medians (first `window - 1` values are NaN)
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 3.0, 2.0, 5.0];
/// let result = statistics::rolling_median(&data, 3).unwrap();
/// assert!(result[0].is_nan() && result[1].is_nan());
/// assert_eq!(result[2], 2.0);
/// assert_eq!(result[3], 3.0);
/// ```
pub fn rolling_median(data: &[f64], window: usize) -> Result<Array1<f64>> {
    let mut output = Array1::from_elem(data.len(), f64::NAN);
    rolling_median_into(
        data,
        window,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Compute the rolling median directly into caller-owned output.
pub fn rolling_median_into(data: &[f64], window: usize, output: &mut [f64]) -> Result<()> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if output.len() != data.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as data".to_string(),
        });
    }

    output.fill(f64::NAN);
    let mut buffer: Vec<f64> = Vec::with_capacity(window);

    for i in 0..data.len() {
        if i + 1 < window {
            continue;
        }
        let start = i + 1 - window;
        buffer.clear();
        buffer.extend(data[start..=i].iter().copied().filter(|v| !v.is_nan()));
        if buffer.is_empty() {
            continue;
        }
        buffer.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let middle = buffer.len() / 2;
        output[i] = if buffer.len().is_multiple_of(2) {
            (buffer[middle - 1] + buffer[middle]) / 2.0
        } else {
            buffer[middle]
        };
    }

    Ok(())
}

/// Rolling range: the trailing maximum minus the trailing minimum.
///
/// The single implementation behind the formula surface's `ROLLING_RANGE` and
/// the compiled-plan `CALL:ROLLING_RANGE` kernel. It is defined in terms of
/// [`rolling_max`] and [`rolling_min`] so the three cannot disagree, and it is
/// deliberately distinct from the TDX `RANGE(X, A, B)` predicate.
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling ranges (NaN where either extremum is not finite)
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 4.0, 2.0, 5.0];
/// let result = statistics::rolling_range(&data, 3).unwrap();
/// assert!(result[0].is_nan() && result[1].is_nan());
/// assert_eq!(result[2], 3.0);
/// assert_eq!(result[3], 3.0);
/// ```
pub fn rolling_range(data: &[f64], window: usize) -> Result<Array1<f64>> {
    let mut output = Array1::from_elem(data.len(), f64::NAN);
    rolling_range_into(
        data,
        window,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Compute the rolling range directly into caller-owned output.
///
/// Expressed through [`rolling_max`] and [`rolling_min`] rather than a third
/// deque, so the three cannot disagree about NaN handling or tie-breaking.
pub fn rolling_range_into(data: &[f64], window: usize, output: &mut [f64]) -> Result<()> {
    let max = rolling_max(data, window)?;
    let min = rolling_min(data, window)?;
    if output.len() != data.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as data".to_string(),
        });
    }
    for index in 0..data.len() {
        output[index] = if max[index].is_finite() && min[index].is_finite() {
            max[index] - min[index]
        } else {
            f64::NAN
        };
    }
    Ok(())
}

/// Compute Kendall Tau rank correlation coefficient between two series.
///
/// Uses the O(n²) pairwise comparison algorithm. For each pair (i, j) with i < j,
/// count concordant (+1) vs discordant (-1) pairs.
/// tau = (concordant - discordant) / (n * (n-1) / 2)
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
///
/// # Returns
/// Kendall Tau coefficient in [-1, 1]
pub fn kendall_tau(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let n = x.len();
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 2".to_string(),
        });
    }

    let mut concordant: i64 = 0;
    let mut discordant: i64 = 0;

    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let x_diff = x[j] - x[i];
            let y_diff = y[j] - y[i];
            let product = x_diff * y_diff;
            if product > 0.0 {
                concordant += 1;
            } else if product < 0.0 {
                discordant += 1;
            }
        }
    }

    let pairs = (n * (n - 1)) as f64 / 2.0;
    if pairs == 0.0 {
        return Ok(0.0);
    }
    Ok((concordant - discordant) as f64 / pairs)
}

/// Compute Spearman rank correlation coefficient between two series.
///
/// Assigns canonical fractional ranks to each series, then computes Pearson
/// correlation on the ranks.
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
///
/// # Returns
/// Spearman rho coefficient in [-1, 1]
pub fn spearman_rank(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let n = x.len();
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 2".to_string(),
        });
    }

    let rank_x = fractional_ranks(x);
    let rank_y = fractional_ranks(y);

    // Pearson correlation on ranks
    let mean_rx: f64 = rank_x.iter().sum::<f64>() / n as f64;
    let mean_ry: f64 = rank_y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = rank_x[i] - mean_rx;
        let dy = rank_y[i] - mean_ry;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        return Ok(0.0);
    }
    Ok(cov / denom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// Deterministic pseudo-noise: enough wiggle to make a cached extreme
    /// leave the window on most bars, which is exactly the regime the Van
    /// Herk block scan must win and where it must still agree with the
    /// cached-index kernel bit for bit.
    fn noisy(len: usize) -> Vec<f64> {
        (0..len)
            .map(|i| {
                let t = i as f64;
                100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
            })
            .collect()
    }

    #[test]
    fn test_van_herk_extreme_matches_cached_index() {
        let data = noisy(2_000);
        for window in [2usize, 3, 7, 14, 30, 64, 511, 512] {
            let mut fast = vec![f64::NAN; data.len()];
            van_herk_extreme_into::<true>(&data, window, &mut fast);
            let mut reference = vec![f64::NAN; data.len()];
            rolling_extreme_cached::<true>(&data, window, window - 1, &mut reference);
            let mismatch = (0..data.len())
                .find(|&i| fast[i] != reference[i] && !(fast[i].is_nan() && reference[i].is_nan()));
            assert_eq!(
                mismatch,
                None,
                "max mismatch at window {window}: {:?}",
                mismatch.map(|i| (i, fast[i], reference[i]))
            );
            let mut fast_min = vec![f64::NAN; data.len()];
            van_herk_extreme_into::<false>(&data, window, &mut fast_min);
            let mut reference_min = vec![f64::NAN; data.len()];
            rolling_extreme_cached::<false>(&data, window, window - 1, &mut reference_min);
            let mismatch_min = (0..data.len()).find(|&i| {
                fast_min[i] != reference_min[i]
                    && !(fast_min[i].is_nan() && reference_min[i].is_nan())
            });
            assert_eq!(
                mismatch_min,
                None,
                "min mismatch at window {window}: {:?}",
                mismatch_min.map(|i| (i, fast_min[i], reference_min[i]))
            );
        }
    }

    /// A leading `inf` is a **value** in this family — `first_finite` skips only
    /// `NaN` (unlike `leading_warmup` in the moving-average layer, which skips
    /// every non-finite bar) — so such a series must not be dispatched to the
    /// block kernel, which has no warm-up concept at all. The guarantee under
    /// test: the dispatched result equals the cached kernel bit for bit, and
    /// the infinite bar is reported as soon as the window is full.
    #[test]
    fn test_rolling_extreme_leading_inf_uses_cached_kernel() {
        let base = noisy(600);
        let window = 30;
        for (fill, want_max) in [(f64::INFINITY, true), (f64::NEG_INFINITY, false)] {
            let mut data = vec![fill; 3];
            data.extend_from_slice(&base);
            let warm_at = first_finite(&data).map_or(data.len(), |start| start + window - 1);

            let mut dispatched = vec![f64::NAN; data.len()];
            let mut reference = vec![f64::NAN; data.len()];
            if want_max {
                fill_rolling_extreme::<true>(&data, window, &mut dispatched);
                rolling_extreme_cached::<true>(&data, window, warm_at, &mut reference);
            } else {
                fill_rolling_extreme::<false>(&data, window, &mut dispatched);
                rolling_extreme_cached::<false>(&data, window, warm_at, &mut reference);
            }
            let mismatch = (0..data.len()).find(|&i| {
                dispatched[i] != reference[i] && !(dispatched[i].is_nan() && reference[i].is_nan())
            });
            assert_eq!(
                mismatch,
                None,
                "want_max={want_max} fill={fill}: {:?}",
                mismatch.map(|i| (i, dispatched[i], reference[i]))
            );
            assert_eq!(dispatched[window - 1], fill, "want_max={want_max}");
        }
    }

    #[test]
    fn test_mean() {
        assert_relative_eq!(
            mean(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap(),
            3.0,
            epsilon = 1e-10
        );
        assert!(mean(&[]).is_err());
    }

    #[test]
    fn test_variance() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let var = variance(&data).unwrap();
        assert_relative_eq!(var, 4.571428571428571, epsilon = 1e-10);
        assert!(variance(&[1.0]).is_err());
    }

    #[test]
    fn test_std_dev() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&data).unwrap();
        assert_relative_eq!(sd, 4.571428571428571_f64.sqrt(), epsilon = 1e-10);
    }

    #[test]
    fn test_covariance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let cov = covariance(&x, &y).unwrap();
        assert_relative_eq!(cov, 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let corr = correlation(&x, &y).unwrap();
        assert_relative_eq!(corr, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_mean(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 4.0, epsilon = 1e-10);
    }

    #[test]
    fn rolling_variance_is_stable_for_large_baseline_values() {
        let data: Vec<f64> = (0..32).map(|index| 1.0e12 + index as f64).collect();
        let result = rolling_variance(&data, 3).unwrap();

        for value in result.iter().skip(2) {
            assert_relative_eq!(*value, 1.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_skewness() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let skew = skewness(&data).unwrap();
        assert_relative_eq!(skew, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kurtosis() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let kurt = kurtosis(&data).unwrap();
        // For uniform distribution, excess kurtosis should be negative
        assert!(kurt < 0.0);
    }

    #[test]
    fn test_rolling_minmax_visit_matches_naive_windows() {
        let high = [3.0, 5.0, 5.0, 2.0, 7.0, 6.0, 7.0, 1.0, 4.0];
        let low = [1.0, 2.0, 2.0, 0.0, 3.0, -1.0, 1.0, -1.0, 2.0];
        let window = 3;
        let mut visited = Vec::new();

        rolling_minmax_visit(&high, &low, window, |index, highest, lowest| {
            visited.push((index, highest, lowest));
        });

        assert_eq!(visited.len(), high.len() + 1 - window);
        for (offset, (index, highest, lowest)) in visited.into_iter().enumerate() {
            let expected_index = offset + window - 1;
            let start = expected_index + 1 - window;
            let expected_high = high[start..=expected_index]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let expected_low = low[start..=expected_index]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);

            assert_eq!(index, expected_index);
            assert_eq!(highest, expected_high);
            assert_eq!(lowest, expected_low);
        }
    }

    #[test]
    fn test_rolling_minmax_visit_window_one() {
        let high = [2.0, 4.0, 3.0];
        let low = [1.0, 2.0, 0.5];
        let mut visited = Vec::new();
        rolling_minmax_visit(&high, &low, 1, |index, highest, lowest| {
            visited.push((index, highest, lowest));
        });
        assert_eq!(visited, vec![(0, 2.0, 1.0), (1, 4.0, 2.0), (2, 3.0, 0.5)]);
    }

    #[test]
    fn test_rolling_max() {
        let data = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 5.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_min() {
        let data = vec![5.0, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_perfect_concordance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let tau = kendall_tau(&x, &y).unwrap();
        assert_relative_eq!(tau, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_perfect_discordance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let tau = kendall_tau(&x, &y).unwrap();
        assert_relative_eq!(tau, -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_invalid() {
        assert!(kendall_tau(&[1.0], &[2.0]).is_err());
        assert!(kendall_tau(&[1.0, 2.0], &[1.0]).is_err());
    }

    #[test]
    fn test_spearman_rank_perfect() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let rho = spearman_rank(&x, &y).unwrap();
        assert_relative_eq!(rho, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_spearman_rank_perfect_inverse() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![50.0, 40.0, 30.0, 20.0, 10.0];
        let rho = spearman_rank(&x, &y).unwrap();
        assert_relative_eq!(rho, -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_spearman_rank_invalid() {
        assert!(spearman_rank(&[1.0], &[2.0]).is_err());
        assert!(spearman_rank(&[1.0, 2.0], &[1.0]).is_err());
    }
}
