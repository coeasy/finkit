use serde::{Deserialize, Serialize};

use crate::config::DecimateStrategy;

/// Level of detail used for a visible chart window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LodLevel {
    /// Draw every source row.
    Raw,
    /// Preserve visible extrema while limiting points to the pixel budget.
    Balanced,
    /// Prefer a compact overview representation.
    Overview,
}

impl Default for LodLevel {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Configurable level-of-detail policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LodPolicy {
    /// Select a level from the visible bar density.
    Auto,
    /// Use one fixed level.
    Fixed(LodLevel),
    /// Use the existing decimation strategy explicitly.
    Strategy(DecimateStrategy),
}

impl Default for LodPolicy {
    fn default() -> Self {
        Self::Auto
    }
}

impl LodPolicy {
    pub fn level(self, bars: usize, pixel_width: u32) -> LodLevel {
        match self {
            Self::Auto => {
                if bars <= pixel_width as usize {
                    LodLevel::Raw
                } else if bars <= (pixel_width as usize).saturating_mul(8) {
                    LodLevel::Balanced
                } else {
                    LodLevel::Overview
                }
            }
            Self::Fixed(level) => level,
            Self::Strategy(DecimateStrategy::EveryNth) => LodLevel::Overview,
            Self::Strategy(_) => LodLevel::Balanced,
        }
    }
}

/// A source-index viewport independent from any renderer backend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Viewport {
    pub start: usize,
    pub end: usize,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub device_pixel_ratio: f32,
    pub overscan_bars: usize,
    pub follow_latest: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            pixel_width: 1200,
            pixel_height: 600,
            device_pixel_ratio: 1.0,
            overscan_bars: 0,
            follow_latest: true,
        }
    }
}

impl Viewport {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: end.max(start),
            follow_latest: false,
            ..Self::default()
        }
    }

    pub fn latest(bars: usize) -> Self {
        Self::new(0, bars.max(1)).with_follow_latest(true)
    }

    pub fn full() -> Self {
        Self {
            start: 0,
            end: 0,
            follow_latest: false,
            ..Self::default()
        }
    }

    pub fn with_pixels(mut self, width: u32, height: u32) -> Self {
        self.pixel_width = width.max(1);
        self.pixel_height = height.max(1);
        self
    }

    pub fn with_overscan(mut self, bars: usize) -> Self {
        self.overscan_bars = bars;
        self
    }

    pub fn with_follow_latest(mut self, follow: bool) -> Self {
        self.follow_latest = follow;
        self
    }

    pub fn is_full(self) -> bool {
        self.end == 0
    }

    pub fn resolve(self, total: usize) -> (usize, usize) {
        if total == 0 || self.is_full() {
            return (0, total);
        }
        if self.follow_latest {
            let width = self.end.saturating_sub(self.start).max(1).min(total);
            return (total.saturating_sub(width), total);
        }
        let end = self.end.min(total).max(1);
        let start = self.start.min(end.saturating_sub(1));
        (start, end)
    }

    pub fn render_range(self, total: usize) -> (usize, usize) {
        let (start, end) = self.resolve(total);
        (
            start.saturating_sub(self.overscan_bars),
            (end + self.overscan_bars).min(total),
        )
    }

    /// Returns the render range plus historical bars needed to warm up a
    /// rolling calculation. The warm-up prefix is never considered visible.
    pub fn warmup_range(self, total: usize, lookback: usize) -> (usize, usize) {
        let (start, end) = self.render_range(total);
        (start.saturating_sub(lookback), end)
    }

    pub fn visible_count(self, total: usize) -> usize {
        let (start, end) = self.resolve(total);
        end.saturating_sub(start)
    }

    pub fn contains(self, index: usize, total: usize) -> bool {
        let (start, end) = self.resolve(total);
        index >= start && index < end
    }

    pub fn index_at_pixel(self, x: f64, plot_x: f64, plot_width: f64, total: usize) -> usize {
        let (start, end) = self.resolve(total);
        if end <= start || plot_width <= 0.0 {
            return start;
        }
        let ratio = ((x - plot_x) / plot_width).clamp(0.0, 0.999_999);
        start + ((end - start) as f64 * ratio) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_latest_viewport_without_exceeding_data() {
        let viewport = Viewport::latest(20).with_follow_latest(true);
        assert_eq!(viewport.resolve(100), (80, 100));
        assert_eq!(viewport.resolve(7), (0, 7));
    }

    #[test]
    fn latest_viewport_follows_tail_by_default() {
        assert_eq!(Viewport::latest(20).resolve(100), (80, 100));
    }

    #[test]
    fn render_range_includes_overscan() {
        let viewport = Viewport::new(20, 40).with_overscan(5);
        assert_eq!(viewport.render_range(100), (15, 45));
        assert_eq!(viewport.warmup_range(100, 10), (5, 45));
    }

    #[test]
    fn auto_lod_scales_with_density() {
        assert_eq!(LodPolicy::Auto.level(100, 200), LodLevel::Raw);
        assert_eq!(LodPolicy::Auto.level(1000, 200), LodLevel::Balanced);
        assert_eq!(LodPolicy::Auto.level(10_000, 200), LodLevel::Overview);
    }
}
