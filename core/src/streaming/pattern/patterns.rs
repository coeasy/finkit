//! Streaming candlestick pattern recognition.
//!
//! Each pattern detector maintains a rolling window of recent candles
//! and outputs a signal on each new candle: 100 (bullish), -100 (bearish), or 0 (no pattern).

use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Candle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

impl Candle {
    fn body(&self) -> f64 { (self.close - self.open).abs() }
    fn upper_shadow(&self) -> f64 { self.high - self.open.max(self.close) }
    fn lower_shadow(&self) -> f64 { self.open.min(self.close) - self.low }
    fn range(&self) -> f64 { self.high - self.low }
    fn is_bullish(&self) -> bool { self.close > self.open }
    fn is_bearish(&self) -> bool { self.close < self.open }
    fn mid_body(&self) -> f64 { (self.open + self.close) / 2.0 }
}

macro_rules! pattern_struct {
    ($name:ident, $window:expr, $str_name:expr, $desc:expr) => {
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name {
            window: VecDeque<Candle>,
            count: usize,
            last_value: Option<i32>,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    window: VecDeque::with_capacity($window),
                    count: 0,
                    last_value: None,
                }
            }
        }

        impl Default for $name {
            fn default() -> Self { Self::new() }
        }

        impl IndicatorMeta for $name {
            fn name() -> &'static str { $str_name }
            fn category() -> &'static str { "pattern" }
            fn description() -> &'static str { $desc }
            fn warm_up_period(&self) -> usize { $window }
        }
    };
}

macro_rules! impl_streaming_pattern {
    ($name:ident, $window:expr, $detect:expr) => {
        impl<T: Ohlcv> StreamingIndicator<T, i32> for $name {
            #[inline]
            fn next(&mut self, input: T) -> Option<i32> {
                self.count += 1;
                let c = Candle {
                    open: input.open(),
                    high: input.high(),
                    low: input.low(),
                    close: input.close(),
                };
                if self.window.len() >= $window {
                    self.window.pop_front();
                }
                self.window.push_back(c);

                if self.window.len() < $window {
                    self.last_value = None;
                    return None;
                }

                let signal = $detect(&self.window);
                self.last_value = Some(signal);
                Some(signal)
            }

            fn reset(&mut self) {
                self.window.clear();
                self.count = 0;
                self.last_value = None;
            }

            fn is_ready(&self) -> bool {
                self.window.len() >= $window
            }

            fn count(&self) -> usize {
                self.count
            }

            fn value(&self) -> Option<i32> {
                self.last_value
            }
        }
    };
}

// --- CDL_DOJI ---
pattern_struct!(StreamingCdlDoji, 1, "CDL_DOJI", "Doji candlestick pattern");
impl_streaming_pattern!(StreamingCdlDoji, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    if c.range() > 1e-15 && c.body() <= c.range() * 0.05 { 100 } else { 0 }
});

// --- CDL_HAMMER ---
pattern_struct!(StreamingCdlHammer, 1, "CDL_HAMMER", "Hammer bullish reversal");
impl_streaming_pattern!(StreamingCdlHammer, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    let lower = c.lower_shadow();
    let upper = c.upper_shadow();
    if lower >= body * 2.0 && upper <= body * 0.3 && body > r * 0.1 { 100 } else { 0 }
});

// --- CDL_INVERTED_HAMMER ---
pattern_struct!(StreamingCdlInvertedHammer, 1, "CDL_INVERTED_HAMMER", "Inverted Hammer pattern");
impl_streaming_pattern!(StreamingCdlInvertedHammer, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    let upper = c.upper_shadow();
    let lower = c.lower_shadow();
    if upper >= body * 2.0 && lower <= body * 0.3 && body > r * 0.1 { 100 } else { 0 }
});

// --- CDL_ENGULFING ---
pattern_struct!(StreamingCdlEngulfing, 2, "CDL_ENGULFING", "Bullish/Bearish Engulfing");
impl_streaming_pattern!(StreamingCdlEngulfing, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    if curr.is_bullish() && prev.is_bearish()
        && curr.open <= prev.close && curr.close >= prev.open {
        100
    } else if curr.is_bearish() && prev.is_bullish()
        && curr.open >= prev.close && curr.close <= prev.open {
        -100
    } else {
        0
    }
});

// --- CDL_MORNINGSTAR ---
pattern_struct!(StreamingCdlMorningStar, 3, "CDL_MORNINGSTAR", "Morning Star bullish reversal");
impl_streaming_pattern!(StreamingCdlMorningStar, 3, |w: &VecDeque<Candle>| {
    let first = &w[0];
    let second = &w[1];
    let third = &w[2];
    if first.is_bearish() && first.body() > first.range() * 0.3
        && second.body() < first.body() * 0.3
        && third.is_bullish() && third.close > first.mid_body() {
        100
    } else {
        0
    }
});

// --- CDL_EVENINGSTAR ---
pattern_struct!(StreamingCdlEveningStar, 3, "CDL_EVENINGSTAR", "Evening Star bearish reversal");
impl_streaming_pattern!(StreamingCdlEveningStar, 3, |w: &VecDeque<Candle>| {
    let first = &w[0];
    let second = &w[1];
    let third = &w[2];
    if first.is_bullish() && first.body() > first.range() * 0.3
        && second.body() < first.body() * 0.3
        && third.is_bearish() && third.close < first.mid_body() {
        -100
    } else {
        0
    }
});

// --- CDL_HARAMI ---
pattern_struct!(StreamingCdlHarami, 2, "CDL_HARAMI", "Harami pattern");
impl_streaming_pattern!(StreamingCdlHarami, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    let prev_top = prev.open.max(prev.close);
    let prev_bot = prev.open.min(prev.close);
    let curr_top = curr.open.max(curr.close);
    let curr_bot = curr.open.min(curr.close);
    if curr_top <= prev_top && curr_bot >= prev_bot && prev.body() > curr.body() {
        if prev.is_bearish() && curr.is_bullish() { 100 }
        else if prev.is_bullish() && curr.is_bearish() { -100 }
        else { 0 }
    } else {
        0
    }
});

// --- CDL_PIERCING ---
pattern_struct!(StreamingCdlPiercing, 2, "CDL_PIERCING", "Piercing Line bullish reversal");
impl_streaming_pattern!(StreamingCdlPiercing, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    if prev.is_bearish() && curr.is_bullish()
        && curr.open < prev.close
        && curr.close > prev.mid_body()
        && curr.close < prev.open {
        100
    } else {
        0
    }
});

// --- CDL_DARKCLOUDCOVER ---
pattern_struct!(StreamingCdlDarkCloudCover, 2, "CDL_DARKCLOUDCOVER", "Dark Cloud Cover bearish reversal");
impl_streaming_pattern!(StreamingCdlDarkCloudCover, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    if prev.is_bullish() && curr.is_bearish()
        && curr.open > prev.close
        && curr.close < prev.mid_body()
        && curr.close > prev.open {
        -100
    } else {
        0
    }
});

// --- CDL_SPINNING_TOP ---
pattern_struct!(StreamingCdlSpinningTop, 1, "CDL_SPINNING_TOP", "Spinning Top indecision");
impl_streaming_pattern!(StreamingCdlSpinningTop, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    let upper = c.upper_shadow();
    let lower = c.lower_shadow();
    if body < r * 0.3 && upper > body && lower > body { 100 } else { 0 }
});

// --- CDL_MARUBOZU ---
pattern_struct!(StreamingCdlMarubozu, 1, "CDL_MARUBOZU", "Marubozu (full body, no wicks)");
impl_streaming_pattern!(StreamingCdlMarubozu, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    if body >= r * 0.95 {
        if c.is_bullish() { 100 } else { -100 }
    } else {
        0
    }
});

// --- CDL_HANGINGMAN ---
pattern_struct!(StreamingCdlHangingMan, 1, "CDL_HANGINGMAN", "Hanging Man bearish reversal");
impl_streaming_pattern!(StreamingCdlHangingMan, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    let lower = c.lower_shadow();
    let upper = c.upper_shadow();
    if lower >= body * 2.0 && upper <= body * 0.3 && body > r * 0.1 && c.is_bearish() {
        -100
    } else {
        0
    }
});

// --- CDL_SHOOTINGSTAR ---
pattern_struct!(StreamingCdlShootingStar, 1, "CDL_SHOOTINGSTAR", "Shooting Star bearish reversal");
impl_streaming_pattern!(StreamingCdlShootingStar, 1, |w: &VecDeque<Candle>| {
    let c = &w[0];
    let r = c.range();
    if r < 1e-15 { return 0; }
    let body = c.body();
    let upper = c.upper_shadow();
    let lower = c.lower_shadow();
    if upper >= body * 2.0 && lower <= body * 0.3 && body > r * 0.1 && c.is_bearish() {
        -100
    } else {
        0
    }
});

// --- CDL_3WHITESOLDIERS ---
pattern_struct!(StreamingCdl3WhiteSoldiers, 3, "CDL_3WHITESOLDIERS", "Three White Soldiers bullish");
impl_streaming_pattern!(StreamingCdl3WhiteSoldiers, 3, |w: &VecDeque<Candle>| {
    let (a, b, c) = (&w[0], &w[1], &w[2]);
    if a.is_bullish() && b.is_bullish() && c.is_bullish()
        && b.close > a.close && c.close > b.close
        && b.open > a.open && b.open < a.close
        && c.open > b.open && c.open < b.close
        && a.upper_shadow() < a.body() * 0.3
        && b.upper_shadow() < b.body() * 0.3
        && c.upper_shadow() < c.body() * 0.3 {
        100
    } else {
        0
    }
});

// --- CDL_3BLACKCROWS ---
pattern_struct!(StreamingCdl3BlackCrows, 3, "CDL_3BLACKCROWS", "Three Black Crows bearish");
impl_streaming_pattern!(StreamingCdl3BlackCrows, 3, |w: &VecDeque<Candle>| {
    let (a, b, c) = (&w[0], &w[1], &w[2]);
    if a.is_bearish() && b.is_bearish() && c.is_bearish()
        && b.close < a.close && c.close < b.close
        && b.open < a.open && b.open > a.close
        && c.open < b.open && c.open > b.close
        && a.lower_shadow() < a.body() * 0.3
        && b.lower_shadow() < b.body() * 0.3
        && c.lower_shadow() < c.body() * 0.3 {
        -100
    } else {
        0
    }
});

// --- CDL_DOJISTAR ---
pattern_struct!(StreamingCdlDojiStar, 2, "CDL_DOJISTAR", "Doji Star pattern");
impl_streaming_pattern!(StreamingCdlDojiStar, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    let is_doji = curr.range() > 1e-15 && curr.body() <= curr.range() * 0.05;
    if !is_doji { return 0; }
    if prev.is_bullish() && prev.body() > prev.range() * 0.3 && curr.low > prev.close {
        -100
    } else if prev.is_bearish() && prev.body() > prev.range() * 0.3 && curr.high < prev.close {
        100
    } else {
        0
    }
});

// --- CDL_ABANDONEDBABY ---
pattern_struct!(StreamingCdlAbandonedBaby, 3, "CDL_ABANDONEDBABY", "Abandoned Baby reversal");
impl_streaming_pattern!(StreamingCdlAbandonedBaby, 3, |w: &VecDeque<Candle>| {
    let (first, second, third) = (&w[0], &w[1], &w[2]);
    let is_doji = second.range() > 1e-15 && second.body() <= second.range() * 0.05;
    if !is_doji { return 0; }
    // Bullish: bearish, gap-down doji, gap-up bullish
    if first.is_bearish() && second.high < first.low && third.low > second.high && third.is_bullish() {
        100
    }
    // Bearish: bullish, gap-up doji, gap-down bearish
    else if first.is_bullish() && second.low > first.high && third.high < second.low && third.is_bearish() {
        -100
    } else {
        0
    }
});

// --- CDL_TRISTAR ---
pattern_struct!(StreamingCdlTristar, 3, "CDL_TRISTAR", "Tri-Star pattern");
impl_streaming_pattern!(StreamingCdlTristar, 3, |w: &VecDeque<Candle>| {
    let (a, b, c) = (&w[0], &w[1], &w[2]);
    let is_doji = |candle: &Candle| candle.range() > 1e-15 && candle.body() <= candle.range() * 0.05;
    if !is_doji(a) || !is_doji(b) || !is_doji(c) { return 0; }
    // Bullish: middle doji gaps down
    if b.high < a.low && c.low > b.high { 100 }
    // Bearish: middle doji gaps up
    else if b.low > a.high && c.high < b.low { -100 }
    else { 0 }
});

// --- CDL_KICKING ---
pattern_struct!(StreamingCdlKicking, 2, "CDL_KICKING", "Kicking pattern");
impl_streaming_pattern!(StreamingCdlKicking, 2, |w: &VecDeque<Candle>| {
    let prev = &w[0];
    let curr = &w[1];
    let prev_maru = prev.body() >= prev.range() * 0.95 && prev.range() > 1e-15;
    let curr_maru = curr.body() >= curr.range() * 0.95 && curr.range() > 1e-15;
    if !prev_maru || !curr_maru { return 0; }
    // Bullish kicking: bearish marubozu then bullish marubozu with gap up
    if prev.is_bearish() && curr.is_bullish() && curr.open > prev.open {
        100
    }
    // Bearish kicking: bullish marubozu then bearish marubozu with gap down
    else if prev.is_bullish() && curr.is_bearish() && curr.open < prev.open {
        -100
    } else {
        0
    }
});

// --- CDL_TASUKIGAP ---
pattern_struct!(StreamingCdlTasukiGap, 3, "CDL_TASUKIGAP", "Tasuki Gap continuation");
impl_streaming_pattern!(StreamingCdlTasukiGap, 3, |w: &VecDeque<Candle>| {
    let (first, second, third) = (&w[0], &w[1], &w[2]);
    // Upside Tasuki Gap
    if first.is_bullish() && second.is_bullish()
        && second.open > first.close  // gap up
        && third.is_bearish()
        && third.open > second.open && third.open < second.close
        && third.close > first.close && third.close < second.open {
        100
    }
    // Downside Tasuki Gap
    else if first.is_bearish() && second.is_bearish()
        && second.open < first.close  // gap down
        && third.is_bullish()
        && third.open < second.open && third.open > second.close
        && third.close < first.close && third.close > second.open {
        -100
    } else {
        0
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;
    use crate::streaming::traits::StreamingIndicator;

    #[test]
    fn test_cdl_doji() {
        let mut d = StreamingCdlDoji::new();
        // Doji: open ≈ close, has range
        let bar = OhlcvBar::new(100.0, 105.0, 95.0, 100.0, 1000.0);
        assert_eq!(d.next(bar), Some(100));
    }

    #[test]
    fn test_cdl_engulfing_bullish() {
        let mut e = StreamingCdlEngulfing::new();
        let bar1 = OhlcvBar::new(105.0, 106.0, 99.0, 100.0, 1000.0); // bearish
        let bar2 = OhlcvBar::new(99.0, 108.0, 98.0, 107.0, 1000.0);  // bullish engulfing
        e.next(bar1);
        assert_eq!(e.next(bar2), Some(100));
    }

    #[test]
    fn test_cdl_engulfing_bearish() {
        let mut e = StreamingCdlEngulfing::new();
        let bar1 = OhlcvBar::new(100.0, 106.0, 99.0, 105.0, 1000.0); // bullish
        let bar2 = OhlcvBar::new(106.0, 107.0, 98.0, 99.0, 1000.0);  // bearish engulfing
        e.next(bar1);
        assert_eq!(e.next(bar2), Some(-100));
    }

    #[test]
    fn test_cdl_hammer() {
        let mut h = StreamingCdlHammer::new();
        // body=2, range=10, lower=98-90=8, upper=100-100=0
        // lower>=body*2 (8>=4 ✓), upper<=body*0.3 (0<=0.6 ✓), body>range*0.1 (2>1 ✓)
        let bar = OhlcvBar::new(98.0, 100.0, 90.0, 100.0, 1000.0);
        let val = h.next(bar);
        assert_eq!(val, Some(100));
    }

    #[test]
    fn test_cdl_marubozu_bullish() {
        let mut m = StreamingCdlMarubozu::new();
        let bar = OhlcvBar::new(100.0, 110.0, 100.0, 110.0, 1000.0);
        assert_eq!(m.next(bar), Some(100));
    }

    #[test]
    fn test_cdl_marubozu_bearish() {
        let mut m = StreamingCdlMarubozu::new();
        let bar = OhlcvBar::new(110.0, 110.0, 100.0, 100.0, 1000.0);
        assert_eq!(m.next(bar), Some(-100));
    }

    #[test]
    fn test_cdl_pattern_reset() {
        let mut d = StreamingCdlDoji::new();
        let bar = OhlcvBar::new(100.0, 105.0, 95.0, 100.0, 1000.0);
        <StreamingCdlDoji as StreamingIndicator<OhlcvBar, i32>>::next(&mut d, bar);
        assert!(<StreamingCdlDoji as StreamingIndicator<OhlcvBar, i32>>::is_ready(&d));
        <StreamingCdlDoji as StreamingIndicator<OhlcvBar, i32>>::reset(&mut d);
        assert!(!<StreamingCdlDoji as StreamingIndicator<OhlcvBar, i32>>::is_ready(&d));
        assert_eq!(<StreamingCdlDoji as StreamingIndicator<OhlcvBar, i32>>::value(&d), None);
    }
}
