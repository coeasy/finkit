use super::traits::Ohlcv;

/// A concrete OHLCV bar holding owned data.
///
/// # Example
///
/// ```
/// use finkit::streaming::{Ohlcv, OhlcvBar};
///
/// let bar = OhlcvBar::new(100.0, 110.0, 95.0, 105.0, 50000.0);
/// assert_eq!(bar.open(), 100.0);
/// assert_eq!(bar.high(), 110.0);
/// assert_eq!(bar.low(), 95.0);
/// assert_eq!(bar.close(), 105.0);
/// assert_eq!(bar.volume(), 50000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OhlcvBar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub time: i64,
}

impl OhlcvBar {
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
            time: 0,
        }
    }

    pub fn new_with_time(open: f64, high: f64, low: f64, close: f64, volume: f64, timestamp: i64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
            time: timestamp,
        }
    }
}

impl Ohlcv for OhlcvBar {
    #[inline]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline]
    fn volume(&self) -> f64 {
        self.volume
    }
    #[inline]
    fn open_time(&self) -> i64 {
        self.time
    }
}

impl Ohlcv for (f64, f64, f64, f64, f64) {
    fn open(&self) -> f64 {
        self.0
    }
    fn high(&self) -> f64 {
        self.1
    }
    fn low(&self) -> f64 {
        self.2
    }
    fn close(&self) -> f64 {
        self.3
    }
    fn volume(&self) -> f64 {
        self.4
    }
}

impl Ohlcv for [f64; 5] {
    fn open(&self) -> f64 {
        self[0]
    }
    fn high(&self) -> f64 {
        self[1]
    }
    fn low(&self) -> f64 {
        self[2]
    }
    fn close(&self) -> f64 {
        self[3]
    }
    fn volume(&self) -> f64 {
        self[4]
    }
}

impl Ohlcv for &dyn Ohlcv {
    fn open(&self) -> f64 {
        (*self).open()
    }
    fn high(&self) -> f64 {
        (*self).high()
    }
    fn low(&self) -> f64 {
        (*self).low()
    }
    fn close(&self) -> f64 {
        (*self).close()
    }
    fn volume(&self) -> f64 {
        (*self).volume()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohlcv_bar_basic() {
        let bar = OhlcvBar::new(100.0, 110.0, 90.0, 105.0, 50000.0);
        assert_eq!(bar.open(), 100.0);
        assert_eq!(bar.high(), 110.0);
        assert_eq!(bar.low(), 90.0);
        assert_eq!(bar.close(), 105.0);
        assert_eq!(bar.volume(), 50000.0);
    }

    #[test]
    fn test_ohlcv_derived_prices() {
        let bar = OhlcvBar::new(100.0, 120.0, 80.0, 110.0, 1000.0);
        let tp = (120.0 + 80.0 + 110.0) / 3.0;
        assert!((bar.typical_price() - tp).abs() < 1e-10);
        assert!((bar.median_price() - 100.0).abs() < 1e-10);
        let wc = (120.0 + 80.0 + 2.0 * 110.0) / 4.0;
        assert!((bar.weighted_close() - wc).abs() < 1e-10);
    }

    #[test]
    fn test_ohlcv_true_range() {
        let bar = OhlcvBar::new(100.0, 110.0, 90.0, 105.0, 1000.0);
        assert!((bar.true_range(95.0) - 20.0).abs() < 1e-10);
        assert!((bar.true_range(115.0) - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_ohlcv_tuple() {
        let bar = (100.0, 110.0, 90.0, 105.0, 50000.0);
        assert_eq!(bar.open(), 100.0);
        assert_eq!(bar.close(), 105.0);
    }

    #[test]
    fn test_ohlcv_array() {
        let bar = [100.0, 110.0, 90.0, 105.0, 50000.0];
        assert_eq!(bar.open(), 100.0);
        assert_eq!(bar.volume(), 50000.0);
    }

    #[test]
    fn test_ohlcv_bar_copy() {
        let bar = OhlcvBar::new(1.0, 2.0, 0.5, 1.5, 100.0);
        let bar2 = bar;
        assert_eq!(bar.close(), bar2.close());
    }

    #[test]
    fn test_ohlcv_bar_new_with_time() {
        let bar = OhlcvBar::new_with_time(100.0, 110.0, 90.0, 105.0, 50000.0, 1700000000);
        assert_eq!(bar.open(), 100.0);
        assert_eq!(bar.close(), 105.0);
        assert_eq!(bar.open_time(), 1700000000);
    }

    #[test]
    fn test_ohlcv_bar_default_time() {
        let bar = OhlcvBar::new(100.0, 110.0, 90.0, 105.0, 50000.0);
        assert_eq!(bar.open_time(), 0);
    }
}
