//! Target encoding for categorical features in ML pipelines.
//!
//! Provides leakage-safe encodings (rolling, smoothed) and cross-validation
//! encodings (leave-one-out, k-fold).

/// Rolling target encoding: mean of `target` for the same category in the past
/// `window` bars only (excludes the current bar).
pub fn rolling_target_encode(categories: &[usize], target: &[f64], window: usize) -> Vec<f64> {
    let n = categories.len().min(target.len());
    let mut out = vec![f64::NAN; n];

    if n == 0 || window == 0 {
        return out;
    }

    for i in 0..n {
        let cat = categories[i];
        let start = i.saturating_sub(window);
        let mut sum = 0.0;
        let mut count = 0usize;
        let mut global_sum = 0.0;
        let mut global_count = 0usize;

        for j in start..i {
            global_sum += target[j];
            global_count += 1;
            if categories[j] == cat {
                sum += target[j];
                count += 1;
            }
        }

        out[i] = if count > 0 {
            sum / count as f64
        } else if global_count > 0 {
            global_sum / global_count as f64
        } else {
            f64::NAN
        };
    }

    out
}

/// Bayesian smoothed target encoding using only past observations.
///
/// `encoded = (count * category_mean + prior_weight * global_mean) / (count + prior_weight)`
pub fn smoothed_target_encode(categories: &[usize], target: &[f64], prior_weight: f64) -> Vec<f64> {
    let n = categories.len().min(target.len());
    let mut out = vec![f64::NAN; n];

    if n == 0 {
        return out;
    }

    let mut global_sum = 0.0;
    let max_cat = categories.iter().copied().max().unwrap_or(0);
    let mut cat_sum = vec![0.0; max_cat + 1];
    let mut cat_count = vec![0usize; max_cat + 1];

    for (global_count, i) in (0..n).enumerate() {
        let cat = categories[i];
        if global_count > 0 {
            let global_mean = global_sum / global_count as f64;
            let count = cat_count[cat];
            out[i] = if count > 0 {
                let cat_mean = cat_sum[cat] / count as f64;
                (count as f64 * cat_mean + prior_weight * global_mean)
                    / (count as f64 + prior_weight)
            } else {
                global_mean
            };
        }

        global_sum += target[i];
        if cat >= cat_sum.len() {
            cat_sum.resize(cat + 1, 0.0);
            cat_count.resize(cat + 1, 0);
        }
        cat_sum[cat] += target[i];
        cat_count[cat] += 1;
    }

    out
}

/// Leave-one-out target encoding using full-sample category statistics.
///
/// For bar `i`, the encoding is the mean of `target` over all bars with the
/// same category except bar `i`.
pub fn loo_encode(categories: &[usize], target: &[f64]) -> Vec<f64> {
    let n = categories.len().min(target.len());
    let mut out = vec![f64::NAN; n];

    if n == 0 {
        return out;
    }

    let max_cat = categories.iter().copied().max().unwrap_or(0);
    let mut cat_sum = vec![0.0; max_cat + 1];
    let mut cat_count = vec![0usize; max_cat + 1];
    let mut global_sum = 0.0;

    for i in 0..n {
        let cat = categories[i];
        global_sum += target[i];
        if cat >= cat_sum.len() {
            cat_sum.resize(cat + 1, 0.0);
            cat_count.resize(cat + 1, 0);
        }
        cat_sum[cat] += target[i];
        cat_count[cat] += 1;
    }

    let global_mean = global_sum / n as f64;

    for i in 0..n {
        let cat = categories[i];
        let count = cat_count[cat];
        if count > 1 {
            out[i] = (cat_sum[cat] - target[i]) / (count - 1) as f64;
        } else {
            let other_sum = global_sum - target[i];
            let other_count = n - 1;
            out[i] = if other_count > 0 {
                other_sum / other_count as f64
            } else {
                global_mean
            };
        }
    }

    out
}

/// K-fold target encoding: each bar is encoded from category means in other folds.
pub fn kfold_encode(categories: &[usize], target: &[f64], k: usize) -> Vec<f64> {
    let n = categories.len().min(target.len());
    let mut out = vec![f64::NAN; n];

    if n == 0 || k == 0 {
        return out;
    }

    let k = k.min(n).max(1);
    let fold_of: Vec<usize> = (0..n).map(|i| i * k / n).collect();

    let max_cat = categories.iter().copied().max().unwrap_or(0);
    let mut fold_cat_sum = vec![vec![0.0; max_cat + 1]; k];
    let mut fold_cat_count = vec![vec![0usize; max_cat + 1]; k];
    let mut cat_sum = vec![0.0; max_cat + 1];
    let mut cat_count = vec![0usize; max_cat + 1];
    let mut global_sum = 0.0;

    for i in 0..n {
        let cat = categories[i];
        let f = fold_of[i];
        global_sum += target[i];
        if cat >= cat_sum.len() {
            cat_sum.resize(cat + 1, 0.0);
            cat_count.resize(cat + 1, 0);
            for fold_stats in fold_cat_sum.iter_mut().zip(fold_cat_count.iter_mut()) {
                fold_stats.0.resize(cat + 1, 0.0);
                fold_stats.1.resize(cat + 1, 0);
            }
        }
        cat_sum[cat] += target[i];
        cat_count[cat] += 1;
        fold_cat_sum[f][cat] += target[i];
        fold_cat_count[f][cat] += 1;
    }

    let global_mean = global_sum / n as f64;

    for i in 0..n {
        let cat = categories[i];
        let f = fold_of[i];
        let total_count = cat_count[cat];
        let holdout_count = fold_cat_count[f][cat];
        let other_count = total_count.saturating_sub(holdout_count);

        out[i] = if other_count > 0 {
            (cat_sum[cat] - fold_cat_sum[f][cat]) / other_count as f64
        } else {
            let other_global_count = n.saturating_sub(fold_cat_count[f].iter().sum::<usize>());
            if other_global_count > 0 {
                let holdout_sum: f64 = fold_cat_sum[f].iter().sum();
                (global_sum - holdout_sum) / other_global_count as f64
            } else {
                global_mean
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_target_encode_no_leakage() {
        let categories = vec![0, 0, 1, 1, 0];
        let target = vec![1.0, 2.0, 10.0, 20.0, 100.0];
        let encoded = rolling_target_encode(&categories, &target, 3);

        assert!(encoded[0].is_nan());

        // Bar 4: past window is bars 1..3; category 0 mean = 2.0 only (bar 1)
        assert!((encoded[4] - 2.0).abs() < 1e-10);

        // Changing future target must not affect past encodings
        let mut target2 = target.clone();
        target2[4] = 999.0;
        let encoded2 = rolling_target_encode(&categories, &target2, 3);
        assert!((encoded2[3] - encoded[3]).abs() < 1e-10);
    }

    #[test]
    fn test_smoothed_target_encode_prior_weight() {
        let categories = vec![0, 0, 0, 0];
        let target = vec![0.0, 0.0, 0.0, 100.0];

        let low_prior = smoothed_target_encode(&categories, &target, 0.1);
        let high_prior = smoothed_target_encode(&categories, &target, 100.0);

        // At bar 3, category 0 has past mean 0; global past mean is 0
        assert!((low_prior[3] - 0.0).abs() < 1e-6);
        // High prior pulls harder toward global mean (0)
        assert!(high_prior[3].abs() <= low_prior[3].abs() + 1e-6);

        // When category mean differs from global past mean, higher prior pulls toward global mean
        let categories2 = vec![0, 0, 1, 0];
        let target2 = vec![10.0, 10.0, 0.0, 100.0];
        let low = smoothed_target_encode(&categories2, &target2, 0.01);
        let high = smoothed_target_encode(&categories2, &target2, 100.0);
        let global_past = (10.0 + 10.0 + 0.0) / 3.0;
        assert!((high[3] - global_past).abs() < (low[3] - global_past).abs());
    }

    #[test]
    fn test_loo_encode_basic() {
        let categories = vec![0, 0, 1, 1];
        let target = vec![1.0, 3.0, 10.0, 20.0];
        let encoded = loo_encode(&categories, &target);

        assert!((encoded[0] - 3.0).abs() < 1e-10);
        assert!((encoded[1] - 1.0).abs() < 1e-10);
        assert!((encoded[2] - 20.0).abs() < 1e-10);
        assert!((encoded[3] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_kfold_encode_basic() {
        let categories = vec![0, 0, 1, 1];
        let target = vec![1.0, 3.0, 10.0, 20.0];
        let encoded = kfold_encode(&categories, &target, 2);

        // n=4, k=2: fold 0 = indices 0,1; fold 1 = indices 2,3
        // Bar 0 (fold 0, cat 0): encoded from fold 1 only — no cat 0 → global of fold 1
        assert!((encoded[0] - 15.0).abs() < 1e-10);
        // Bar 2 (fold 1, cat 1): encoded from fold 0 only — cat 1 mean = (1+3)/2? no cat 1 in fold 0
        // fold 0 has cats [0,0] targets [1,3] → global mean 2.0 for cat 1 fallback
        assert!((encoded[2] - 2.0).abs() < 1e-10);
        // Must not equal in-fold category means (would leak holdout labels)
        let in_fold_mean_cat0 = (1.0 + 3.0) / 2.0;
        assert!((encoded[0] - in_fold_mean_cat0).abs() > 1e-10);
        assert!((encoded[2] - 10.0).abs() > 1e-10);
    }
}
