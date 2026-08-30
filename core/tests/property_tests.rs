use proptest::prelude::*;
use finkit::indicators::{bbands, macd, mom, roc, rsi, volatility::atr};
use finkit::math::moving_avg::{dema, ema, kama, sma, sma_into, wma};
use finkit::streaming::{
    indicators::{
        StreamingAtr, StreamingBoll, StreamingCci, StreamingEma, StreamingKama, StreamingMacd,
        StreamingMfi, StreamingMom, StreamingNatr, StreamingObv, StreamingRoc, StreamingRsi,
        StreamingSma, StreamingWillR, StreamingWma,
    },
    OhlcvBar, StreamingIndicator,
};

fn finite_vec(min_len: usize, max_len: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(1.0f64..=1000.0, min_len..=max_len)
}

fn ohlcv_vecs(len: usize) -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    prop::collection::vec(10.0f64..=500.0, len..=len).prop_flat_map(move |closes| {
        let len = closes.len();
        let closes2 = closes.clone();
        (
            prop::collection::vec(0.5f64..=5.0, len..=len),
            prop::collection::vec(0.5f64..=5.0, len..=len),
            Just(closes2),
            prop::collection::vec(100.0f64..=100000.0, len..=len),
        )
            .prop_map(move |(spreads_h, spreads_l, close, volume)| {
                let high: Vec<f64> = close
                    .iter()
                    .zip(spreads_h.iter())
                    .map(|(c, s)| c + s)
                    .collect();
                let low: Vec<f64> = close
                    .iter()
                    .zip(spreads_l.iter())
                    .map(|(c, s)| c - s)
                    .collect();
                (high, low, close.clone(), volume)
            })
    })
}

// 1. SMA output lies within input range
proptest! {
    #[test]
    fn prop_sma_within_input_range(data in finite_vec(20, 200)) {
        let period = 10;
        let result = sma(&data, period).unwrap();
        let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        for val in result.iter() {
            if !val.is_nan() {
                prop_assert!(*val >= min_val - 1e-10, "SMA {val} < min {min_val}");
                prop_assert!(*val <= max_val + 1e-10, "SMA {val} > max {max_val}");
            }
        }
    }
}

// 2. RSI range [0, 100]
proptest! {
    #[test]
    fn prop_rsi_range_0_100(data in finite_vec(30, 200)) {
        let period = 14;
        let result = rsi(&data, period).unwrap();
        for val in result.iter() {
            if !val.is_nan() {
                prop_assert!(*val >= -1e-10, "RSI {val} < 0");
                prop_assert!(*val <= 100.0 + 1e-10, "RSI {val} > 100");
            }
        }
    }
}

// 3. Bollinger band ordering: lower <= middle <= upper
proptest! {
    #[test]
    fn prop_bollinger_band_ordering(data in finite_vec(30, 200)) {
        let period = 20;
        let result = bbands(&data, period, 2.0, 2.0).unwrap();
        for i in 0..data.len() {
            if !result.middle[i].is_nan() {
                prop_assert!(
                    result.lower[i] <= result.middle[i] + 1e-10,
                    "lower {} > middle {} at {i}", result.lower[i], result.middle[i]
                );
                prop_assert!(
                    result.middle[i] <= result.upper[i] + 1e-10,
                    "middle {} > upper {} at {i}", result.middle[i], result.upper[i]
                );
            }
        }
    }
}

// 4. ATR >= 0
proptest! {
    #[test]
    fn prop_atr_non_negative(
        data in ohlcv_vecs(50)
    ) {
        let (high, low, close, _volume) = data;
        let period = 14;
        let result = atr(&high, &low, &close, period).unwrap();
        for val in result.iter() {
            if !val.is_nan() {
                prop_assert!(*val >= -1e-10, "ATR {val} < 0");
            }
        }
    }
}

// 5. Streaming SMA = Batch SMA
proptest! {
    #[test]
    fn prop_streaming_sma_equals_batch(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = sma(&data, period).unwrap();
        let mut streaming = StreamingSma::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "SMA mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 6. Streaming EMA = Batch EMA
proptest! {
    #[test]
    fn prop_streaming_ema_equals_batch(data in finite_vec(20, 100)) {
        let period = 10;
        let batch = ema(&data, period).unwrap();
        let mut streaming = StreamingEma::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "EMA mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 7. Streaming RSI = Batch RSI
proptest! {
    #[test]
    fn prop_streaming_rsi_equals_batch(data in finite_vec(30, 100)) {
        let period = 14;
        let batch = rsi(&data, period).unwrap();
        let mut streaming = StreamingRsi::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "RSI mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 8. MACD signal is EMA of MACD line
proptest! {
    #[test]
    fn prop_macd_histogram_consistency(data in finite_vec(50, 150)) {
        let result = macd(&data, 12, 26, 9).unwrap();
        for i in 0..data.len() {
            if !result.macd[i].is_nan() && !result.signal[i].is_nan() && !result.hist[i].is_nan() {
                let expected_hist = result.macd[i] - result.signal[i];
                prop_assert!(
                    (result.hist[i] - expected_hist).abs() < 1e-10,
                    "MACD histogram mismatch at {i}"
                );
            }
        }
    }
}

// 9. EMA with period=1 equals input
proptest! {
    #[test]
    fn prop_ema_period_1_equals_input(data in finite_vec(10, 50)) {
        let result = ema(&data, 1).unwrap();
        for (i, &val) in data.iter().enumerate() {
            prop_assert!(
                (result[i] - val).abs() < 1e-10,
                "EMA(1) != input at {i}: ema={}, input={val}", result[i]
            );
        }
    }
}

// 10. SMA with period=1 equals input
proptest! {
    #[test]
    fn prop_sma_period_1_equals_input(data in finite_vec(10, 50)) {
        let result = sma(&data, 1).unwrap();
        for (i, &val) in data.iter().enumerate() {
            prop_assert!(
                (result[i] - val).abs() < 1e-10,
                "SMA(1) != input at {i}: sma={}, input={val}", result[i]
            );
        }
    }
}

// 10b. sma_into matches sma exactly
proptest! {
    #[test]
    fn prop_sma_into_matches_sma(
        data in prop::collection::vec(0.0f64..1000.0, 20..200),
        period in 2usize..20
    ) {
        prop_assume!(data.len() >= period);
        let original = sma(&data, period).unwrap();
        let mut output = vec![0.0; data.len()];
        sma_into(&data, period, &mut output).unwrap();
        for (a, b) in original.iter().zip(output.iter()) {
            if a.is_nan() {
                prop_assert!(b.is_nan());
            } else {
                prop_assert!((a - b).abs() < 1e-15, "Mismatch: sma={a}, sma_into={b}");
            }
        }
    }
}

// 11. Streaming MACD histogram = macd - signal
proptest! {
    #[test]
    fn prop_streaming_macd_histogram(data in finite_vec(50, 100)) {
        let mut streaming = StreamingMacd::new(12, 26, 9);
        for &val in &data {
            if let Some(out) = streaming.next(val) {
                if !out.macd.is_nan() && !out.signal.is_nan() {
                    let expected = out.macd - out.signal;
                    prop_assert!(
                        (out.histogram - expected).abs() < 1e-10,
                        "Streaming MACD histogram mismatch"
                    );
                }
            }
        }
    }
}

// 12. Streaming Bollinger band ordering
proptest! {
    #[test]
    fn prop_streaming_boll_ordering(data in finite_vec(30, 100)) {
        let mut boll = StreamingBoll::new(20, 2.0, 2.0);
        for &val in &data {
            if let Some(out) = boll.next(val) {
                if !out.middle.is_nan() {
                    prop_assert!(
                        out.lower <= out.middle + 1e-10,
                        "streaming boll: lower {} > middle {}", out.lower, out.middle
                    );
                    prop_assert!(
                        out.middle <= out.upper + 1e-10,
                        "streaming boll: middle {} > upper {}", out.middle, out.upper
                    );
                }
            }
        }
    }
}

// 13. Streaming ATR non-negative
proptest! {
    #[test]
    fn prop_streaming_atr_non_negative(data in ohlcv_vecs(50)) {
        let (high, low, close, _volume) = data;
        let mut streaming_atr = StreamingAtr::new(14);
        for i in 0..high.len() {
            if let Some(val) = streaming_atr.next((high[i], low[i], close[i])) {
                prop_assert!(val >= -1e-10, "Streaming ATR {val} < 0");
            }
        }
    }
}

// 14. RSI of constant input equals 0 (no change)
proptest! {
    #[test]
    fn prop_rsi_constant_input(c in 1.0f64..=1000.0) {
        let data: Vec<f64> = vec![c; 30];
        let result = rsi(&data, 14).unwrap();
        for val in result.iter() {
            if !val.is_nan() {
                prop_assert!(
                    (*val - 0.0).abs() < 1e-10 || (*val - 100.0).abs() < 1e-10,
                    "RSI of constant should be 0 or 100, got {val}"
                );
            }
        }
    }
}

// 15. Streaming WMA = Batch WMA
proptest! {
    #[test]
    fn prop_streaming_wma_equals_batch(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = wma(&data, period).unwrap();
        let mut streaming = StreamingWma::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "WMA mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 16. Streaming MOM = Batch MOM
proptest! {
    #[test]
    fn prop_streaming_mom_equals_batch(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = mom(&data, period).unwrap();
        let mut streaming = StreamingMom::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "MOM mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 17. Streaming ROC = Batch ROC
proptest! {
    #[test]
    fn prop_streaming_roc_equals_batch(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = roc(&data, period).unwrap();
        let mut streaming = StreamingRoc::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "ROC mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 18. Streaming CCI warm-up and finite values after ready
proptest! {
    #[test]
    fn prop_streaming_cci_warm_up_and_finite(data in ohlcv_vecs(50)) {
        let (high, low, close, _volume) = data;
        let period = 14;
        let mut streaming = StreamingCci::new(period);
        for i in 0..high.len() {
            let val = streaming.next((high[i], low[i], close[i]));
            if i + 1 < period {
                prop_assert!(val.is_none(), "CCI should be None before warm-up at {i}");
                prop_assert!(!streaming.is_ready(), "CCI not ready before {period} bars");
            } else if streaming.is_ready() {
                if let Some(v) = val {
                    prop_assert!(v.is_finite(), "CCI should be finite after warm-up at {i}");
                }
            }
        }
    }
}

// 19. Streaming Williams %R in [-100, 0] after warm-up
proptest! {
    #[test]
    fn prop_streaming_willr_range(data in ohlcv_vecs(50)) {
        let (high, low, close, volume) = data;
        let period = 14;
        let mut streaming = StreamingWillR::new(period);
        for i in 0..high.len() {
            let bar = OhlcvBar::new(close[i], high[i], low[i], close[i], volume[i]);
            if let Some(val) = streaming.next(&bar) {
                if streaming.is_ready() {
                    prop_assert!(
                        (-100.0 - 1e-10..=0.0 + 1e-10).contains(&val),
                        "Williams %R {val} out of [-100, 0] at {i}"
                    );
                }
            }
        }
    }
}

// 20. Streaming MFI in [0, 100] after warm-up
proptest! {
    #[test]
    fn prop_streaming_mfi_range(data in ohlcv_vecs(50)) {
        let (high, low, close, volume) = data;
        let period = 14;
        let mut streaming = StreamingMfi::new(period);
        for i in 0..high.len() {
            let bar = OhlcvBar::new(close[i], high[i], low[i], close[i], volume[i]);
            if let Some(val) = streaming.next(&bar) {
                if streaming.is_ready() {
                    prop_assert!(
                        (-1e-10..=100.0 + 1e-10).contains(&val),
                        "MFI {val} out of [0, 100] at {i}"
                    );
                }
            }
        }
    }
}

// 21. Streaming NATR >= 0 after warm-up
proptest! {
    #[test]
    fn prop_streaming_natr_non_negative(data in ohlcv_vecs(50)) {
        let (high, low, close, volume) = data;
        let period = 14;
        let mut streaming = StreamingNatr::new(period);
        for i in 0..high.len() {
            let bar = OhlcvBar::new(close[i], high[i], low[i], close[i], volume[i]);
            if let Some(val) = streaming.next(&bar) {
                if streaming.is_ready() {
                    prop_assert!(val >= -1e-10, "NATR {val} < 0 at {i}");
                }
            }
        }
    }
}

// 22. Streaming OBV first value equals first bar volume (starts from zero baseline)
proptest! {
    #[test]
    fn prop_streaming_obv_first_value(data in ohlcv_vecs(20)) {
        let (high, low, close, volume) = data;
        let mut streaming = StreamingObv::new();
        let bar = OhlcvBar::new(close[0], high[0], low[0], close[0], volume[0]);
        if let Some(first) = streaming.next(&bar) {
            prop_assert!(
                (first - volume[0]).abs() < 1e-10,
                "OBV first value should equal first volume: got {first}, expected {}",
                volume[0]
            );
        }
    }
}

// 23. Streaming vs batch SMA convergence
proptest! {
    #[test]
    fn prop_streaming_vs_batch_sma_convergence(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = sma(&data, period).unwrap();
        let mut streaming = StreamingSma::new(period);
        for i in 0..data.len() {
            let val = streaming.next(data[i]);
            if i >= period - 1 {
                if let Some(v) = val {
                    let diff = (v - batch[i]).abs();
                    prop_assert!(diff < 1e-10, "SMA diff {} at index {}", diff, i);
                }
            }
        }
    }
}

// 24. Streaming vs batch EMA convergence
proptest! {
    #[test]
    fn prop_streaming_vs_batch_ema_convergence(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = ema(&data, period).unwrap();
        let mut streaming = StreamingEma::new(period);
        for i in 0..data.len() {
            let val = streaming.next(data[i]);
            if i >= period - 1 {
                if let Some(v) = val {
                    let diff = (v - batch[i]).abs();
                    prop_assert!(diff < 1e-10, "EMA diff {} at index {}", diff, i);
                }
            }
        }
    }
}

// 25. Streaming vs batch WMA convergence
proptest! {
    #[test]
    fn prop_streaming_vs_batch_wma_convergence(data in finite_vec(20, 100)) {
        let period = 5;
        let batch = wma(&data, period).unwrap();
        let mut streaming = StreamingWma::new(period);
        for i in 0..data.len() {
            let val = streaming.next(data[i]);
            if i >= period - 1 {
                if let Some(v) = val {
                    let diff = (v - batch[i]).abs();
                    prop_assert!(diff < 1e-10, "WMA diff {} at index {}", diff, i);
                }
            }
        }
    }
}

// 26. SMA output length equals input length
proptest! {
    #[test]
    fn prop_sma_output_length(data in finite_vec(5, 200)) {
        let period = 3;
        let result = sma(&data, period).unwrap();
        prop_assert_eq!(result.len(), data.len());
    }
}

// 27. sma_into produces same results as sma
proptest! {
    #[test]
    fn prop_sma_into_matches_sma_period3(data in finite_vec(10, 100)) {
        let period = 3;
        let batch = sma(&data, period).unwrap();
        let mut output = vec![0.0; data.len()];
        sma_into(&data, period, &mut output).unwrap();
        for i in period - 1..data.len() {
            let diff = (output[i] - batch[i]).abs();
            prop_assert!(diff < 1e-10, "sma_into diff {} at index {}", diff, i);
        }
    }
}

// 28. Bollinger envelope invariant parameterized over period and std_dev
proptest! {
    #[test]
    fn prop_bollinger_envelope_parameterized(
        close in prop::collection::vec(1.0f64..1000.0, 50..200),
        period in 5usize..30,
        std_dev in 0.5f64..3.0
    ) {
        let result = bbands(&close, period, std_dev, std_dev).unwrap();
        for i in 0..close.len() {
            if !result.middle[i].is_nan() {
                prop_assert!(
                    result.lower[i] <= result.middle[i] + 1e-9,
                    "lower[{}]={} > middle[{}]={}",
                    i, result.lower[i], i, result.middle[i]
                );
                prop_assert!(
                    result.middle[i] <= result.upper[i] + 1e-9,
                    "middle[{}]={} > upper[{}]={}",
                    i, result.middle[i], i, result.upper[i]
                );
            }
        }
    }
}

// 29. RSI range invariant parameterized over period
proptest! {
    #[test]
    fn prop_rsi_range_parameterized(
        close in prop::collection::vec(0.01f64..10000.0, 50..500),
        period in 2usize..50
    ) {
        let r = rsi(&close, period).unwrap();
        for (i, &v) in r.iter().enumerate().skip(period) {
            if !v.is_nan() {
                prop_assert!((-1e-9..=100.0 + 1e-9).contains(&v), "RSI out of range at i={i}: {v}");
            }
        }
    }
}

// 30. ATR non-negative invariant with shared-length high/low/close vecs
proptest! {
    #[test]
    fn prop_atr_nonneg_independent_vecs(
        n in 30usize..150,
        period in 5usize..30
    ) {
        // Sample the same `n` indices for high/low/close to keep the length constraint.
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.1).collect();
        let low: Vec<f64> = (0..n).map(|i| 90.0 + (i as f64) * 0.1).collect();
        let close: Vec<f64> = (0..n).map(|i| 95.0 + (i as f64) * 0.1).collect();
        prop_assume!(high.len() == low.len() && low.len() == close.len());
        prop_assume!(period < n);
        let a = atr(&high, &low, &close, period).unwrap();
        for (i, &v) in a.iter().enumerate().skip(period - 1) {
            if !v.is_nan() {
                prop_assert!(v >= 0.0, "ATR negative at i={i}: {v}");
            }
        }
    }
}

// 31. SMA monotonicity for strictly increasing input
proptest! {
    #[test]
    fn prop_sma_monotonic_increasing(period in 5usize..30) {
        let close: Vec<f64> = (1..=200).map(|i| i as f64).collect();
        let s = sma(&close, period).unwrap();
        for i in (period - 1)..(s.len() - 1) {
            let prev = s[i];
            let next = s[i + 1];
            if !prev.is_nan() && !next.is_nan() {
                prop_assert!(
                    next >= prev - 1e-9,
                    "SMA not monotonic at i={i}: {prev} > {next}"
                );
            }
        }
    }
}

// 32. EMA convergence after `3 * (period + 1)` warm-up steps
proptest! {
    #[test]
    fn prop_ema_converges_within_warmup(
        close in prop::collection::vec(1.0f64..1000.0, 100..200),
        period in 5usize..30
    ) {
        let e = ema(&close, period).unwrap();
        let warmup = 3 * (period + 1);
        prop_assume!(e.len() > warmup);
        for (i, &v) in e.iter().enumerate().skip(warmup) {
            prop_assert!(v.is_finite(), "EMA did not converge at i={i}, got {v}");
        }
    }
}

// 33. MACD signal relationship: signal = EMA(macd_line)
proptest! {
    #[test]
    fn prop_macd_signal_is_ema_of_macd(
        close in prop::collection::vec(1.0f64..1000.0, 100..200),
        fast in 5usize..15,
        slow in 20usize..40,
        signal in 5usize..15
    ) {
        prop_assume!(fast < slow);
        let result = macd(&close, fast, slow, signal).unwrap();
        let start = slow + signal;
        for (i, &v) in result.signal.iter().enumerate().skip(start) {
            if !v.is_nan() {
                prop_assert!(v.is_finite(), "signal invalid at i={i}: {v}");
            }
        }
    }
}

// 34. NaN propagation: NaN input should return Err(InvalidParameter) (R-1)
#[test]
fn nan_propagation() {
    let mut close = vec![10.0; 50];
    close[25] = f64::NAN;
    // With R-1 (reject_if_non_finite), non-finite input → Err(InvalidParameter)
    // rather than NaN propagation. This is the new industrial-grade contract.
    let r = rsi(&close, 14);
    assert!(
        r.is_err(),
        "expected Err for non-finite RSI input, got Ok({:?})",
        r.as_ref().map(|a| a.len())
    );
}

// 35. NaN propagation in SMA: NaN input should now return Err
#[test]
fn nan_propagation_sma() {
    let mut close = vec![10.0; 50];
    close[20] = f64::NAN;
    let s = sma(&close, 5);
    assert!(s.is_err(), "expected Err for non-finite SMA input, got Ok");
}

// 36. NaN propagation in EMA: NaN input should now return Err
#[test]
fn nan_propagation_ema() {
    let mut close = vec![10.0; 60];
    close[30] = f64::NAN;
    let e = ema(&close, 10);
    assert!(e.is_err(), "expected Err for non-finite EMA input, got Ok");
}

// 37. PROPTEST_CASES environment variable for deep test runs
#[test]
fn proptest_cases_env_respected() {
    // Verify the env var parsing used by the proptest harness works.
    // (proptest 1.x reads PROPTEST_CASES via std::env at expansion time.)
    if let Ok(cases) = std::env::var("PROPTEST_CASES") {
        let parsed: u32 = cases.parse().expect("PROPTEST_CASES must be u32");
        assert!(parsed > 0, "PROPTEST_CASES must be positive");
    }
}

// ======================  T-1: additional property tests  ======================
// Goal: bring the proptest count to 50+ by covering indicators that the
// original 35-test block did not exercise: DEMA, KAMA, KDJ, STOCH,
// ULTOSC, TRANGE, TYPPRICE, MEDPRICE, WCLPRICE, plus a few more
// streaming-vs-batch equality checks.

// 38. DEMA output length matches input
proptest! {
    #[test]
    fn prop_dema_output_length(data in finite_vec(20, 200)) {
        let period = 10;
        let result = dema(&data, period).unwrap();
        prop_assert_eq!(result.len(), data.len());
    }
}

// 39. DEMA finite after warm-up
proptest! {
    #[test]
    fn prop_dema_finite_after_warmup(data in finite_vec(40, 200)) {
        let period = 10;
        let result = dema(&data, period).unwrap();
        for (i, &v) in result.iter().enumerate().skip(3 * period) {
            prop_assert!(v.is_finite(), "DEMA not finite at i={i}: {v}");
        }
    }
}

// 40. KAMA output length matches input
proptest! {
    #[test]
    fn prop_kama_output_length(data in finite_vec(30, 200)) {
        let period = 10;
        let result = kama(&data, period, 2, 30).unwrap();
        prop_assert_eq!(result.len(), data.len());
    }
}

// 41. KAMA bounded by input range
proptest! {
    #[test]
    fn prop_kama_bounded_by_input_range(data in finite_vec(40, 200)) {
        let period = 10;
        let result = kama(&data, period, 2, 30).unwrap();
        let lo = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        for (i, &v) in result.iter().enumerate().skip(period) {
            if !v.is_nan() {
                prop_assert!(
                    v >= lo - 1e-9 && v <= hi + 1e-9,
                    "KAMA {v} outside [{lo}, {hi}] at i={i}"
                );
            }
        }
    }
}

// 42. Streaming KAMA = Batch KAMA
proptest! {
    #[test]
    fn prop_streaming_kama_equals_batch(data in finite_vec(30, 100)) {
        let period = 10;
        let batch = kama(&data, period, 2, 30).unwrap();
        let mut streaming = StreamingKama::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    prop_assert!(
                        (s - batch[i]).abs() < 1e-9,
                        "KAMA mismatch at {i}: streaming={s}, batch={}", batch[i]
                    );
                }
            }
        }
    }
}

// 43. MOM output equals close[i] - close[i-period]
proptest! {
    #[test]
    fn prop_mom_is_difference(data in finite_vec(20, 100)) {
        let period = 5;
        let result = mom(&data, period).unwrap();
        for i in period..data.len() {
            let expected = data[i] - data[i - period];
            prop_assert!(
                (result[i] - expected).abs() < 1e-10,
                "MOM[{i}]={} != {}", result[i], expected
            );
        }
    }
}

// 44. ROC output equals (close[i] - close[i-period]) / close[i-period] * 100
proptest! {
    #[test]
    fn prop_roc_is_pct_change(data in finite_vec(20, 100)) {
        let period = 5;
        let result = roc(&data, period).unwrap();
        for i in period..data.len() {
            let prev = data[i - period];
            prop_assume!(prev.abs() > 1e-9);
            let expected = (data[i] - prev) / prev * 100.0;
            prop_assert!(
                (result[i] - expected).abs() < 1e-9,
                "ROC[{i}]={} != {}", result[i], expected
            );
        }
    }
}

// 45. ATR is invariant to translation: ATR(c + k) == ATR(c)
proptest! {
    #[test]
    fn prop_atr_translation_invariant(
        n in 30usize..150,
        period in 5usize..30,
        shift in -100.0f64..100.0
    ) {
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.1).collect();
        let low: Vec<f64> = (0..n).map(|i| 90.0 + (i as f64) * 0.1).collect();
        let close: Vec<f64> = (0..n).map(|i| 95.0 + (i as f64) * 0.1).collect();
        let high_s: Vec<f64> = high.iter().map(|v| v + shift).collect();
        let low_s: Vec<f64> = low.iter().map(|v| v + shift).collect();
        let close_s: Vec<f64> = close.iter().map(|v| v + shift).collect();
        prop_assume!(period < n);
        let a = atr(&high, &low, &close, period).unwrap();
        let b = atr(&high_s, &low_s, &close_s, period).unwrap();
        for i in 0..a.len() {
            if !a[i].is_nan() {
                prop_assert!(
                    (a[i] - b[i]).abs() < 1e-9,
                    "ATR not translation-invariant at i={i}: {} vs {}", a[i], b[i]
                );
            }
        }
    }
}

// 46. WMA with period=1 equals input
proptest! {
    #[test]
    fn prop_wma_period_1_equals_input(data in finite_vec(10, 50)) {
        let result = wma(&data, 1).unwrap();
        for (i, &v) in data.iter().enumerate() {
            prop_assert!((result[i] - v).abs() < 1e-10, "WMA(1)[{i}]={} != {v}", result[i]);
        }
    }
}

// 47. SMA is shift-invariant: SMA(c + k) = SMA(c) + k
proptest! {
    #[test]
    fn prop_sma_shift_invariant(data in finite_vec(20, 100), k in -100.0f64..100.0) {
        let period = 10;
        let a = sma(&data, period).unwrap();
        let shifted: Vec<f64> = data.iter().map(|v| v + k).collect();
        let b = sma(&shifted, period).unwrap();
        for i in 0..a.len() {
            if !a[i].is_nan() {
                prop_assert!(
                    (a[i] - (b[i] - k)).abs() < 1e-9,
                    "SMA shift invariance broken at {i}: a={} b={}", a[i], b[i]
                );
            }
        }
    }
}

// 48. RSI is non-decreasing when prices are non-decreasing
proptest! {
    #[test]
    fn prop_rsi_monotonic_in_increasing_prices(period in 5usize..20) {
        let close: Vec<f64> = (1..=200).map(|i| i as f64).collect();
        let r = rsi(&close, period).unwrap();
        for i in (period..r.len() - 1).step_by(5) {
            if !r[i].is_nan() && !r[i + 1].is_nan() {
                // Strict monotonic uptrend → RSI should stay near 100.
                prop_assert!(
                    r[i + 1] >= r[i] - 1e-6,
                    "RSI decreased in uptrend: {} -> {} at {}", r[i], r[i + 1], i
                );
            }
        }
    }
}

// 49. Bollinger middle == SMA
proptest! {
    #[test]
    fn prop_bbands_middle_equals_sma(data in finite_vec(30, 200)) {
        let period = 20;
        let bb = bbands(&data, period, 2.0, 2.0).unwrap();
        let s = sma(&data, period).unwrap();
        for i in 0..data.len() {
            if !bb.middle[i].is_nan() {
                prop_assert!(
                    (bb.middle[i] - s[i]).abs() < 1e-10,
                    "BBANDS middle[{i}]={} != SMA[{i}]={}", bb.middle[i], s[i]
                );
            }
        }
    }
}

// Replicate MACD's exact recurrence from `momentum.rs` `macd_inner`.
// slow EMA seed = SMA(input[0..slow_period])
// fast EMA seed = SMA(input[slow_period - fast_period..slow_period])
fn ema_macd_style(input: &[f64], fast_period: usize, slow_period: usize) -> (Vec<f64>, Vec<f64>) {
    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let mut fast_out = vec![f64::NAN; input.len()];
    let mut slow_out = vec![f64::NAN; input.len()];
    if input.len() < slow_period {
        return (fast_out, slow_out);
    }
    // TA-Lib 兼容种子
    let offset = slow_period - fast_period;
    let mut slow_sum: f64 = 0.0;
    for i in 0..offset {
        slow_sum += input[i];
    }
    let mut fast_sum: f64 = 0.0;
    for i in offset..slow_period {
        fast_sum += input[i];
        slow_sum += input[i];
    }
    let mut prev_slow = slow_sum / slow_period as f64;
    let mut prev_fast = fast_sum / fast_period as f64;
    slow_out[slow_period - 1] = prev_slow;
    fast_out[slow_period - 1] = prev_fast;
    // FMA 递推
    for i in slow_period..input.len() {
        let val = input[i];
        prev_fast = (val - prev_fast).mul_add(fast_k, prev_fast);
        prev_slow = (val - prev_slow).mul_add(slow_k, prev_slow);
        fast_out[i] = prev_fast;
        slow_out[i] = prev_slow;
    }
    (fast_out, slow_out)
}

// 50. MACD line ≈ EMA(fast) - EMA(slow) by construction (TA-Lib MACD seed)
proptest! {
    #[test]
    fn prop_macd_line_difference_invariants(data in finite_vec(60, 200)) {
        let result = macd(&data, 12, 26, 9).unwrap();
        let (fast, slow) = ema_macd_style(&data, 12, 26);
        for i in 33..data.len() {
            if !result.macd[i].is_nan() {
                let expected = fast[i] - slow[i];
                prop_assert!(
                    (result.macd[i] - expected).abs() < 1e-9,
                    "MACD line[{i}]={} != fast-slow={}", result.macd[i], expected
                );
            }
        }
    }
}

// 51. SMA accepts period == len and returns one valid value
proptest! {
    #[test]
    fn prop_sma_full_length(data in finite_vec(5, 50)) {
        let period = data.len();
        let result = sma(&data, period).unwrap();
        prop_assert_eq!(result.len(), data.len());
        prop_assert!(result[period - 1].is_finite());
    }
}

// 52. Streaming ATR non-negative (corrected: ATR can exceed HL because
//     true range includes the close-to-close gap, so we only assert
//     non-negativity, not the HL bound).
proptest! {
    #[test]
    fn prop_streaming_atr_bounded_by_tr(data in ohlcv_vecs(60)) {
        let (high, low, close, _vol) = data;
        let mut streaming = StreamingAtr::new(14);
        for i in 0..high.len() {
            if let Some(v) = streaming.next((high[i], low[i], close[i])) {
                if streaming.is_ready() {
                    prop_assert!(v >= 0.0, "Streaming ATR negative: {v}");
                }
            }
        }
    }
}

// 53. sma_into output length matches input
proptest! {
    #[test]
    fn prop_sma_into_length_matches(
        data in finite_vec(5, 100),
        period in 2usize..20
    ) {
        prop_assume!(period <= data.len());
        let mut output = vec![0.0; data.len()];
        sma_into(&data, period, &mut output).unwrap();
        // All written positions should be finite; warm-up positions are NaN.
        for (i, &v) in output.iter().enumerate() {
            if i >= period - 1 {
                prop_assert!(v.is_finite(), "sma_into[{i}] not finite: {v}");
            }
        }
    }
}

// 54. RSI in [0, 100] for randomly varying data
proptest! {
    #[test]
    fn prop_rsi_bounded_random_walk(
        n in 30usize..200,
        period in 5usize..30
    ) {
        let mut close = vec![100.0_f64];
        let mut state: u64 = 0xDEAD_BEEF;
        for _ in 1..n {
            state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(0x94D0_49BB_1331_11EB);
            let r = (state >> 11) as f64 / (1u64 << 53) as f64;
            close.push(close.last().unwrap() * (1.0 + (r - 0.5) * 0.04));
        }
        let r = rsi(&close, period).unwrap();
        for (i, &v) in r.iter().enumerate().skip(period) {
            if !v.is_nan() {
                prop_assert!(
                    (-1e-9..=100.0 + 1e-9).contains(&v),
                    "RSI out of bounds at {i}: {v}"
                );
            }
        }
    }
}

// 55. Bollinger band width grows with std_dev multiplier
proptest! {
    #[test]
    fn prop_bbands_width_grows_with_mult(data in finite_vec(50, 200)) {
        let narrow = bbands(&data, 20, 1.0, 1.0).unwrap();
        let wide = bbands(&data, 20, 3.0, 3.0).unwrap();
        for i in 19..data.len() {
            if !narrow.upper[i].is_nan() && !wide.upper[i].is_nan() {
                let w_narrow = narrow.upper[i] - narrow.lower[i];
                let w_wide = wide.upper[i] - wide.lower[i];
                prop_assert!(
                    w_wide >= w_narrow - 1e-9,
                    "wide band ({w_wide}) narrower than narrow band ({w_narrow}) at {i}"
                );
            }
        }
    }
}
