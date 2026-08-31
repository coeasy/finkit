use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::{impl_indicator_meta, impl_standard_methods};

/// Output for VWAP Bands
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VwapBandsOutput {
    pub vwap: f64,
    pub upper: f64,
    pub lower: f64,
}

/// Streaming VWAP Bands
///
/// VWAP with upper/lower bands based on rolling standard deviation of typical prices.
///
/// Upper = VWAP + nb_dev * StdDev(TP, period)
/// Lower = VWAP - nb_dev * StdDev(TP, period)
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVwapBands {
    period: usize,
    nb_dev: f64,
    cumulative_tp_vol: f64,
    cumulative_vol: f64,
    tp_ring: Vec<f64>,
    tp_ring_idx: usize,
    rolling_sum: f64,
    rolling_m2: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVwapBands {
    pub fn new(period: usize, nb_dev: f64) -> Self {
        Self {
            period,
            nb_dev,
            cumulative_tp_vol: 0.0,
            cumulative_vol: 0.0,
            tp_ring: vec![0.0; period],
            tp_ring_idx: 0,
            rolling_sum: 0.0,
            rolling_m2: 0.0,
            count: 0,
            last_value: None,
        }
    }

    pub fn next_bar(&mut self, bar: &dyn Ohlcv) -> Option<VwapBandsOutput> {
        self.count += 1;
        let tp = (bar.high() + bar.low() + bar.close()) / 3.0;

        self.cumulative_tp_vol += tp * bar.volume();
        self.cumulative_vol += bar.volume();

        if self.count <= self.period {
            self.tp_ring[self.tp_ring_idx] = tp;
            self.tp_ring_idx = (self.tp_ring_idx + 1) % self.period;
            let old_sum = self.rolling_sum;
            self.rolling_sum += tp;
            if self.count > 1 {
                let old_mean = old_sum / (self.count - 1) as f64;
                let new_mean = self.rolling_sum / self.count as f64;
                self.rolling_m2 += (tp - old_mean) * (tp - new_mean);
            }
        } else {
            let old_tp = self.tp_ring[self.tp_ring_idx];
            let old_mean = self.rolling_sum / self.period as f64;
            self.tp_ring[self.tp_ring_idx] = tp;
            self.tp_ring_idx = (self.tp_ring_idx + 1) % self.period;
            self.rolling_sum += tp - old_tp;
            let new_mean = self.rolling_sum / self.period as f64;
            self.rolling_m2 +=
                (tp - new_mean) * (tp - old_mean) - (old_tp - new_mean) * (old_tp - old_mean);
            if self.rolling_m2 < 0.0 {
                self.rolling_m2 = 0.0;
            }
        }

        if self.cumulative_vol.abs() <= 1e-15 || self.count < self.period {
            self.last_value = None;
            return None;
        }

        let vwap = self.cumulative_tp_vol / self.cumulative_vol;
        let variance = self.rolling_m2 / self.period as f64;
        let std_dev = variance.sqrt();

        let upper = vwap + self.nb_dev * std_dev;
        let lower = vwap - self.nb_dev * std_dev;

        self.last_value = Some(vwap);
        Some(VwapBandsOutput { vwap, upper, lower })
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingVwapBands {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.next_bar(bar).map(|o| o.vwap)
    }

    fn reset(&mut self) {
        self.cumulative_tp_vol = 0.0;
        self.cumulative_vol = 0.0;
        self.tp_ring.iter_mut().for_each(|x| *x = 0.0);
        self.tp_ring_idx = 0;
        self.rolling_sum = 0.0;
        self.rolling_m2 = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingVwapBands,
    "VWAPBands",
    "volume",
    "VWAP Bands (upper/lower deviation bands)"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_vwap_bands_basic() {
        let mut vb = StreamingVwapBands::new(3, 2.0);
        let bars: Vec<OhlcvBar> = (0..10)
            .map(|i| {
                let base = 50.0 + i as f64;
                OhlcvBar::new(0.0, base + 1.0, base - 1.0, base, 100.0)
            })
            .collect();

        for bar in bars.iter().take(2) {
            assert!(vb.next_bar(bar as &dyn Ohlcv).is_none());
        }
        let out = vb.next_bar(&bars[2] as &dyn Ohlcv).unwrap();
        assert!(out.upper > out.vwap);
        assert!(out.lower < out.vwap);
    }

    #[test]
    fn test_streaming_vwap_bands_meta() {
        assert_eq!(StreamingVwapBands::name(), "VWAPBands");
        assert_eq!(StreamingVwapBands::category(), "volume");
    }

    #[test]
    fn test_streaming_vwap_bands_reset() {
        let mut vb = StreamingVwapBands::new(3, 2.0);
        let bar = OhlcvBar::new(0.0, 12.0, 8.0, 10.0, 100.0);
        for _ in 0..5 {
            vb.next(&bar as &dyn Ohlcv);
        }
        assert!(vb.is_ready());
        vb.reset();
        assert!(!vb.is_ready());
        assert_eq!(vb.count(), 0);
    }

    #[test]
    fn test_streaming_vwap_bands_bands_width() {
        let mut vb = StreamingVwapBands::new(5, 1.0);
        let bars: Vec<OhlcvBar> = vec![
            OhlcvBar::new(0.0, 52.0, 48.0, 50.0, 100.0),
            OhlcvBar::new(0.0, 53.0, 49.0, 51.0, 110.0),
            OhlcvBar::new(0.0, 54.0, 50.0, 52.0, 120.0),
            OhlcvBar::new(0.0, 55.0, 51.0, 53.0, 130.0),
            OhlcvBar::new(0.0, 56.0, 52.0, 54.0, 140.0),
        ];

        let mut last_out = None;
        for bar in &bars {
            last_out = vb.next_bar(bar as &dyn Ohlcv);
        }
        let out = last_out.unwrap();
        assert!(out.upper - out.lower > 0.0);
        assert!((out.upper - out.vwap - (out.vwap - out.lower)).abs() < 1e-10);
    }
}
