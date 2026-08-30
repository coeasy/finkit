use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DmaOutput {
    pub dma: f64,
    pub ama: f64,
}

/// Streaming DMA (Different of Moving Averages 平行线差).
///
/// DMA = SMA(short) - SMA(long), AMA = SMA(DMA, ama_period).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDma {
    short_period: usize,
    long_period: usize,
    ama_period: usize,
    sma_short: StreamingSma,
    sma_long: StreamingSma,
    sma_ama: StreamingSma,
    count: usize,
    last_value: Option<DmaOutput>,
}

impl StreamingDma {
    pub fn new(short_period: usize, long_period: usize, ama_period: usize) -> Self {
        Self {
            short_period,
            long_period,
            ama_period,
            sma_short: StreamingSma::new(short_period),
            sma_long: StreamingSma::new(long_period),
            sma_ama: StreamingSma::new(ama_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, DmaOutput> for StreamingDma {
    #[inline]
    fn next(&mut self, input: f64) -> Option<DmaOutput> {
        self.count += 1;

        let short = self.sma_short.next(input);
        let long = self.sma_long.next(input);
        let (Some(short), Some(long)) = (short, long) else {
            self.last_value = None;
            return None;
        };

        let dma = short - long;
        let Some(ama) = self.sma_ama.next(dma) else {
            self.last_value = None;
            return None;
        };

        let result = Some(DmaOutput { dma, ama });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma_short.reset();
        self.sma_long.reset();
        self.sma_ama.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sma_short.is_ready() && self.sma_long.is_ready() && self.sma_ama.is_ready()
    }

        impl_standard_methods!(output = DmaOutput);


}

impl IndicatorMeta for StreamingDma {
    fn name() -> &'static str {
        "DMA"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Different of Moving Averages (平行线差)"
    }

    fn warm_up_period(&self) -> usize {
        self.short_period.max(self.long_period) + self.ama_period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_dma_basic() {
        let mut dma = StreamingDma::new(3, 5, 3);
        for i in 1..=10 {
            dma.next(i as f64 * 10.0);
        }
        assert!(dma.is_ready());
        let out = dma.value().unwrap();
        assert!(out.dma.is_finite());
        assert!(out.ama.is_finite());
    }

    #[test]
    fn test_streaming_dma_reset() {
        let mut dma = StreamingDma::new(3, 5, 3);
        for i in 0..20 {
            dma.next(i as f64 + 1.0);
        }
        assert!(dma.is_ready());
        dma.reset();
        assert!(!dma.is_ready());
        assert_eq!(dma.count(), 0);
    }

    #[test]
    fn test_streaming_dma_meta() {
        let dma = StreamingDma::new(10, 50, 10);
        assert_eq!(StreamingDma::name(), "DMA");
        assert_eq!(StreamingDma::category(), "overlap");
        assert_eq!(dma.warm_up_period(), 59);
    }
}
