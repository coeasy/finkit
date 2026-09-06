use finkit::indicators::{
    midpoint, midpoint_into, midprice, midprice_into, willr, willr_into,
};

fn assert_same(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected.iter()).enumerate() {
        if expected.is_nan() {
            assert!(actual.is_nan(), "index {index}: expected NaN, got {actual}");
        } else {
            assert!(
                (actual - expected).abs() <= 1e-12,
                "index {index}: expected {expected}, got {actual}"
            );
        }
    }
}

fn reference_midpoint(input: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; input.len()];
    for i in (period - 1)..input.len() {
        let start = i + 1 - period;
        let mut highest = input[start];
        let mut lowest = input[start];
        for &value in &input[(start + 1)..=i] {
            highest = highest.max(value);
            lowest = lowest.min(value);
        }
        output[i] = (highest + lowest) * 0.5;
    }
    output
}

fn reference_midprice(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; high.len()];
    for i in (period - 1)..high.len() {
        let start = i + 1 - period;
        let mut highest = high[start];
        let mut lowest = low[start];
        for j in (start + 1)..=i {
            highest = highest.max(high[j]);
            lowest = lowest.min(low[j]);
        }
        output[i] = (highest + lowest) * 0.5;
    }
    output
}

fn reference_willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; close.len()];
    for i in (period - 1)..close.len() {
        let start = i + 1 - period;
        let mut highest = high[start];
        let mut lowest = low[start];
        for j in (start + 1)..=i {
            highest = highest.max(high[j]);
            lowest = lowest.min(low[j]);
        }
        let range = highest - lowest;
        output[i] = if range > 1e-15 {
            (highest - close[i]) / range * -100.0
        } else {
            0.0
        };
    }
    output
}

#[test]
fn extrema_round6_owned_and_into_match_reference_across_ring_boundary() {
    let len = 640usize;
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);

    for i in 0..len {
        let wave = ((i * 37 + i * i * 3) % 211) as f64 * 0.03125;
        let base = 90.0 + wave + (i % 17) as f64 * 0.0078125;
        let h = base + 1.0 + (i % 5) as f64 * 0.0625;
        let l = base - 1.0 - (i % 7) as f64 * 0.046875;
        high.push(h);
        low.push(l);
        close.push(l + (h - l) * ((i % 11) as f64 / 10.0).min(1.0));
    }

    for period in [1usize, 14, 256, 257] {
        let expected_midpoint = reference_midpoint(&close, period);
        let owned_midpoint = midpoint(&close, period).unwrap();
        let mut into_midpoint = vec![0.0; len];
        midpoint_into(&close, period, &mut into_midpoint).unwrap();
        assert_same(owned_midpoint.as_slice().unwrap(), &expected_midpoint);
        assert_same(&into_midpoint, &expected_midpoint);

        let expected_midprice = reference_midprice(&high, &low, period);
        let owned_midprice = midprice(&high, &low, period).unwrap();
        let mut into_midprice = vec![0.0; len];
        midprice_into(&high, &low, period, &mut into_midprice).unwrap();
        assert_same(owned_midprice.as_slice().unwrap(), &expected_midprice);
        assert_same(&into_midprice, &expected_midprice);

        let expected_willr = reference_willr(&high, &low, &close, period);
        let owned_willr = willr(&high, &low, &close, period).unwrap();
        let mut into_willr = vec![0.0; len];
        willr_into(&high, &low, &close, period, &mut into_willr).unwrap();
        assert_same(owned_willr.as_slice().unwrap(), &expected_willr);
        assert_same(&into_willr, &expected_willr);
    }
}
