use crate::config::DecimateStrategy;
use crate::data::KlineData;

pub struct DecimatedKline {
    pub indices: Vec<usize>,
    pub dates: Vec<String>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
    pub original_len: usize,
}

impl DecimatedKline {
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

#[allow(clippy::needless_range_loop)]
pub fn lttb(data: &[f64], threshold: usize) -> Vec<usize> {
    let n = data.len();
    if n <= threshold {
        return (0..n).collect();
    }
    if threshold < 3 {
        let mut result = Vec::with_capacity(threshold);
        for i in 0..threshold.min(n) {
            result.push(i);
        }
        return result;
    }

    let mut selected = Vec::with_capacity(threshold);
    selected.push(0);

    let bucket_size = (n - 2) as f64 / (threshold - 2) as f64;

    let mut prev_index: usize = 0;
    for i in 0..(threshold - 2) {
        let avg_start = ((i + 1) as f64 * bucket_size + 1.0) as usize;
        let avg_end = ((i + 2) as f64 * bucket_size + 1.0).min(n as f64) as usize;

        let avg_x: f64 =
            (avg_start..avg_end).map(|j| j as f64).sum::<f64>() / (avg_end - avg_start) as f64;
        let avg_y: f64 =
            (avg_start..avg_end).map(|j| data[j]).sum::<f64>() / (avg_end - avg_start) as f64;

        let range_start = ((i as f64) * bucket_size + 1.0) as usize;
        let range_end = (((i + 1) as f64) * bucket_size + 1.0) as usize;
        let range_end = range_end.min(n);

        let prev_x = prev_index as f64;
        let prev_y = data[prev_index];

        let mut max_area = f64::NEG_INFINITY;
        let mut max_index = range_start;

        for j in range_start..range_end {
            let area = ((prev_x - avg_x) * (data[j] - prev_y)
                - (prev_x - j as f64) * (avg_y - prev_y))
                .abs();
            if area > max_area {
                max_area = area;
                max_index = j;
            }
        }

        selected.push(max_index);
        prev_index = max_index;
    }

    selected.push(n - 1);
    selected
}

pub fn min_max(highs: &[f64], lows: &[f64], threshold: usize) -> Vec<usize> {
    let n = highs.len();
    if n <= threshold {
        return (0..n).collect();
    }
    if threshold < 2 {
        return (0..threshold.min(n)).collect();
    }

    let num_buckets = threshold / 2;
    let bucket_size = n as f64 / num_buckets as f64;
    let mut selected = Vec::with_capacity(num_buckets * 2);

    for i in 0..num_buckets {
        let start = (i as f64 * bucket_size) as usize;
        let end = ((i + 1) as f64 * bucket_size).ceil() as usize;
        let end = end.min(n);

        if start >= end {
            continue;
        }

        let mut max_idx = start;
        let mut min_idx = start;
        for j in start..end {
            if highs[j] > highs[max_idx] {
                max_idx = j;
            }
            if lows[j] < lows[min_idx] {
                min_idx = j;
            }
        }

        if max_idx == min_idx {
            selected.push(max_idx);
        } else if max_idx < min_idx {
            selected.push(max_idx);
            selected.push(min_idx);
        } else {
            selected.push(min_idx);
            selected.push(max_idx);
        }
    }

    if !selected.contains(&0) {
        selected.insert(0, 0);
    }
    if !selected.contains(&(n - 1)) {
        selected.push(n - 1);
    }

    selected.sort();
    selected.dedup();
    selected
}

pub fn every_nth(len: usize, threshold: usize) -> Vec<usize> {
    if len <= threshold {
        return (0..len).collect();
    }
    if threshold < 2 {
        return (0..threshold.min(len)).collect();
    }

    let step = (len - 1) as f64 / (threshold - 1) as f64;
    let mut selected = Vec::with_capacity(threshold);

    for i in 0..(threshold - 1) {
        let idx = (i as f64 * step).round() as usize;
        if selected.last().is_none_or(|&last| idx > last) {
            selected.push(idx);
        }
    }

    if selected.last() != Some(&(len - 1)) {
        selected.push(len - 1);
    }

    selected
}

pub fn decimate(
    data: &KlineData,
    strategy: &DecimateStrategy,
    canvas_width: u32,
) -> DecimatedKline {
    let n = data.len();
    let threshold = canvas_width as usize;

    if n == 0 {
        return DecimatedKline {
            indices: Vec::new(),
            dates: Vec::new(),
            opens: Vec::new(),
            highs: Vec::new(),
            lows: Vec::new(),
            closes: Vec::new(),
            volumes: Vec::new(),
            original_len: 0,
        };
    }

    if threshold == 0 || n <= threshold {
        return DecimatedKline {
            indices: (0..n).collect(),
            dates: data.dates.clone(),
            opens: data.opens.clone(),
            highs: data.highs.clone(),
            lows: data.lows.clone(),
            closes: data.closes.clone(),
            volumes: data.volumes.clone(),
            original_len: n,
        };
    }

    let indices = match strategy {
        DecimateStrategy::Auto => {
            if n > canvas_width as usize * 2 {
                lttb(data.closes(), threshold)
            } else {
                (0..n).collect()
            }
        }
        DecimateStrategy::LTTB => lttb(data.closes(), threshold),
        DecimateStrategy::MinMax => min_max(data.highs(), data.lows(), threshold),
        DecimateStrategy::EveryNth => every_nth(n, threshold),
    };

    let mut dates = Vec::with_capacity(indices.len());
    let mut opens = Vec::with_capacity(indices.len());
    let mut highs = Vec::with_capacity(indices.len());
    let mut lows = Vec::with_capacity(indices.len());
    let mut closes = Vec::with_capacity(indices.len());
    let mut volumes = Vec::with_capacity(indices.len());

    for &idx in &indices {
        dates.push(data.dates[idx].clone());
        opens.push(data.opens[idx]);
        highs.push(data.highs[idx]);
        lows.push(data.lows[idx]);
        closes.push(data.closes[idx]);
        volumes.push(data.volumes[idx]);
    }

    DecimatedKline {
        indices,
        dates,
        opens,
        highs,
        lows,
        closes,
        volumes,
        original_len: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_data(len: usize) -> KlineData {
        let mut data = KlineData::new(
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
        );
        for i in 0..len {
            let v = i as f64;
            data.push(
                format!("2024-01-{:02}", i % 28 + 1),
                v,
                v + 2.0,
                v - 1.0,
                v + 1.0,
                v * 100.0,
            );
        }
        data
    }

    fn make_sine_data(len: usize) -> KlineData {
        let mut data = KlineData::new(
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
        );
        for i in 0..len {
            let v = (i as f64 * 0.1).sin() * 50.0 + 100.0;
            data.push(
                format!("2024-01-{:02}", i % 28 + 1),
                v,
                v + 2.0,
                v - 2.0,
                v + 1.0,
                v * 100.0,
            );
        }
        data
    }

    fn make_extreme_data() -> KlineData {
        let mut data = KlineData::new(
            Vec::with_capacity(10),
            Vec::with_capacity(10),
            Vec::with_capacity(10),
            Vec::with_capacity(10),
            Vec::with_capacity(10),
            Vec::with_capacity(10),
        );
        let closes = vec![10.0, 20.0, 5.0, 30.0, 15.0, 25.0, 8.0, 35.0, 12.0, 18.0];
        let highs = vec![12.0, 22.0, 7.0, 33.0, 17.0, 27.0, 10.0, 38.0, 14.0, 20.0];
        let lows = vec![8.0, 18.0, 3.0, 28.0, 13.0, 23.0, 6.0, 32.0, 10.0, 16.0];
        for i in 0..10 {
            data.push(
                format!("2024-01-{:02}", i + 1),
                closes[i],
                highs[i],
                lows[i],
                closes[i],
                1000.0,
            );
        }
        data
    }

    #[test]
    fn test_lttb_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let indices = lttb(&data, 20);
        assert_eq!(indices.len(), 20);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[indices.len() - 1], 99);
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_lttb_preserves_extremes() {
        let mut data = vec![10.0; 100];
        data[25] = 100.0;
        data[75] = 0.0;
        let indices = lttb(&data, 20);
        assert!(indices.contains(&25));
        assert!(indices.contains(&75));
    }

    #[test]
    fn test_lttb_small_data() {
        let data: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let indices = lttb(&data, 10);
        assert_eq!(indices.len(), 5);
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_lttb_threshold_equals_len() {
        let data: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let indices = lttb(&data, 20);
        assert_eq!(indices.len(), 20);
    }

    #[test]
    fn test_lttb_sine_wave() {
        let data: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
        let indices = lttb(&data, 50);
        assert_eq!(indices.len(), 50);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[49], 999);
    }

    #[test]
    fn test_min_max_basic() {
        let highs: Vec<f64> = (0..100).map(|i| i as f64 + 2.0).collect();
        let lows: Vec<f64> = (0..100).map(|i| i as f64 - 1.0).collect();
        let indices = min_max(&highs, &lows, 20);
        assert!(indices.len() >= 20);
        assert_eq!(indices[0], 0);
        assert!(indices.contains(&99));
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_min_max_preserves_extremes() {
        let mut highs = vec![20.0; 100];
        let mut lows = vec![10.0; 100];
        highs[30] = 100.0;
        lows[70] = 0.0;
        let indices = min_max(&highs, &lows, 20);
        assert!(indices.contains(&30));
        assert!(indices.contains(&70));
    }

    #[test]
    fn test_min_max_small_data() {
        let highs = vec![5.0, 6.0, 7.0];
        let lows = vec![1.0, 2.0, 3.0];
        let indices = min_max(&highs, &lows, 10);
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_min_max_dedup() {
        let highs = vec![10.0, 20.0, 15.0, 25.0];
        let lows = vec![5.0, 10.0, 8.0, 12.0];
        let indices = min_max(&highs, &lows, 4);
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
        let unique: std::collections::HashSet<usize> = indices.iter().copied().collect();
        assert_eq!(unique.len(), indices.len());
    }

    #[test]
    fn test_every_nth_basic() {
        let indices = every_nth(100, 10);
        assert_eq!(indices.len(), 10);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[indices.len() - 1], 99);
    }

    #[test]
    fn test_every_nth_preserves_endpoints() {
        let indices = every_nth(1000, 50);
        assert_eq!(indices[0], 0);
        assert_eq!(*indices.last().expect("finkit-visualization: unexpected None/Err in visualization/src/decimate.rs (A5 governance)"), 999);
    }

    #[test]
    fn test_every_nth_sorted() {
        let indices = every_nth(100, 20);
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_every_nth_small_data() {
        let indices = every_nth(5, 10);
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_every_nth_two_points() {
        let indices = every_nth(100, 2);
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[1], 99);
    }

    #[test]
    fn test_decimate_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let result = decimate(&data, &DecimateStrategy::LTTB, 100);
        assert!(result.is_empty());
        assert_eq!(result.original_len, 0);
    }

    #[test]
    fn test_decimate_data_less_than_threshold() {
        let data = make_test_data(50);
        let result = decimate(&data, &DecimateStrategy::LTTB, 100);
        assert_eq!(result.len(), 50);
        assert_eq!(result.original_len, 50);
        assert_eq!(result.indices, (0..50).collect::<Vec<usize>>());
    }

    #[test]
    fn test_decimate_zero_canvas_width() {
        let data = make_test_data(50);
        let result = decimate(&data, &DecimateStrategy::LTTB, 0);
        assert_eq!(result.len(), 50);
        assert_eq!(result.original_len, 50);
    }

    #[test]
    fn test_decimate_lttb_strategy() {
        let data = make_test_data(1000);
        let result = decimate(&data, &DecimateStrategy::LTTB, 100);
        assert!(result.len() <= 102);
        assert!(result.len() >= 98);
        assert_eq!(result.indices[0], 0);
        assert_eq!(*result.indices.last().expect("finkit-visualization: unexpected None/Err in visualization/src/decimate.rs (A5 governance)"), 999);
        assert_eq!(result.original_len, 1000);
    }

    #[test]
    fn test_decimate_minmax_strategy() {
        let data = make_extreme_data();
        let result = decimate(&data, &DecimateStrategy::MinMax, 6);
        assert!(result.len() >= 4);
        assert_eq!(result.original_len, 10);
        assert!(result.highs.contains(&38.0));
        assert!(result.lows.contains(&3.0));
    }

    #[test]
    fn test_decimate_every_nth_strategy() {
        let data = make_test_data(1000);
        let result = decimate(&data, &DecimateStrategy::EveryNth, 100);
        assert_eq!(result.len(), 100);
        assert_eq!(result.indices[0], 0);
        assert_eq!(*result.indices.last().expect("finkit-visualization: unexpected None/Err in visualization/src/decimate.rs (A5 governance)"), 999);
        assert_eq!(result.original_len, 1000);
    }

    #[test]
    fn test_decimate_auto_large_data() {
        let data = make_test_data(1000);
        let result = decimate(&data, &DecimateStrategy::Auto, 100);
        assert!(result.len() <= 102);
        assert!(result.len() >= 98);
        assert!(result.len() < 1000);
    }

    #[test]
    fn test_decimate_auto_small_data() {
        let data = make_test_data(150);
        let result = decimate(&data, &DecimateStrategy::Auto, 100);
        assert_eq!(result.len(), 150);
    }

    #[test]
    fn test_decimated_kline_fields() {
        let data = make_test_data(1000);
        let result = decimate(&data, &DecimateStrategy::LTTB, 50);
        assert_eq!(result.dates.len(), result.indices.len());
        assert_eq!(result.opens.len(), result.indices.len());
        assert_eq!(result.highs.len(), result.indices.len());
        assert_eq!(result.lows.len(), result.indices.len());
        assert_eq!(result.closes.len(), result.indices.len());
        assert_eq!(result.volumes.len(), result.indices.len());
        assert_eq!(result.original_len, 1000);

        for &idx in &result.indices {
            assert!(idx < 1000);
        }
    }

    #[test]
    fn test_decimate_sine_wave_lttb() {
        let data = make_sine_data(1000);
        let result = decimate(&data, &DecimateStrategy::LTTB, 100);
        assert_eq!(result.indices[0], 0);
        assert_eq!(*result.indices.last().expect("finkit-visualization: unexpected None/Err in visualization/src/decimate.rs (A5 governance)"), 999);
        assert!(result.len() <= 102);
    }

    #[test]
    fn test_lttb_constant_data() {
        let data = vec![5.0; 100];
        let indices = lttb(&data, 10);
        assert_eq!(indices.len(), 10);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[indices.len() - 1], 99);
    }

    #[test]
    fn test_min_max_same_high_low_index() {
        let highs = vec![10.0, 20.0, 15.0];
        let lows = vec![5.0, 10.0, 3.0];
        let indices = min_max(&highs, &lows, 6);
        assert!(indices.contains(&1));
        assert!(indices.contains(&2));
    }

    #[test]
    fn test_every_nth_exact_division() {
        let indices = every_nth(100, 10);
        assert_eq!(indices.len(), 10);
        assert_eq!(*indices.last().expect("finkit-visualization: unexpected None/Err in visualization/src/decimate.rs (A5 governance)"), 99);
    }

    #[test]
    fn test_decimate_auto_boundary() {
        let data = make_test_data(201);
        let result = decimate(&data, &DecimateStrategy::Auto, 100);
        assert!(result.len() < 201);

        let data2 = make_test_data(200);
        let result2 = decimate(&data2, &DecimateStrategy::Auto, 100);
        assert_eq!(result2.len(), 200);
    }

    #[test]
    fn test_performance_large_dataset() {
        let len = 1_000_000;
        let mut data = KlineData::new(
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
            Vec::with_capacity(len),
        );
        for i in 0..len {
            let v = (i as f64 * 0.0001).sin() * 50.0 + 100.0;
            data.push(
                format!("{}", i),
                v,
                v + 1.0,
                v - 1.0,
                v + 0.5,
                (i % 10000) as f64,
            );
        }

        let start = std::time::Instant::now();
        let result = decimate(&data, &DecimateStrategy::LTTB, 1200);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "LTTB took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        assert!(result.len() <= 1202);
        assert_eq!(result.original_len, 1_000_000);

        let start = std::time::Instant::now();
        let _result = decimate(&data, &DecimateStrategy::MinMax, 1200);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "MinMax took {}ms, expected < 100ms",
            elapsed.as_millis()
        );

        let start = std::time::Instant::now();
        let result = decimate(&data, &DecimateStrategy::EveryNth, 1200);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "EveryNth took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        assert_eq!(result.len(), 1200);
    }
}
