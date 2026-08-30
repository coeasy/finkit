use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EneOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Streaming ENE (Envelope 轨道线).
///
/// Middle = SMA(close, period), Upper = Middle * (1 + k1/100), Lower = Middle * (1 - k2/100).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingEne {
    period: usize,
    k1: f64,
    k2: f64,
    sma: StreamingSma,
    count: usize,
    last_value: Option<EneOutput>,
}

impl StreamingEne {
    pub fn new(period: usize, k1: f64, k2: f64) -> Self {
        Self {
            period,
            k1,
            k2,
            sma: StreamingSma::new(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, EneOutput> for StreamingEne {
    #[inline]
    fn next(&mut self, input: f64) -> Option<EneOutput> {
        self.count += 1;

        let Some(middle) = self.sma.next(input) else {
            self.last_value = None;
            return None;
        };

        let result = Some(EneOutput {
            upper: middle * (1.0 + self.k1 / 100.0),
            middle,
            lower: middle * (1.0 - self.k2 / 100.0),
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sma.is_ready()
    }

        impl_standard_methods!(output = EneOutput);


}

impl IndicatorMeta for StreamingEne {
    fn name() -> &'static str {
        "ENE"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Envelope (轨道线)"
    }

    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ene_basic() {
        let mut ene = StreamingEne::new(3, 11.0, 9.0);
        assert_eq!(ene.next(10.0), None);
        assert_eq!(ene.next(20.0), None);
        let out = ene.next(30.0).unwrap();
        assert!((out.middle - 20.0).abs() < 1e-10);
        assert!((out.upper - 20.0 * 1.11).abs() < 1e-10);
        assert!((out.lower - 20.0 * 0.91).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_ene_reset() {
        let mut ene = StreamingEne::new(5, 11.0, 9.0);
        for i in 0..10 {
            ene.next(i as f64 + 1.0);
        }
        assert!(ene.is_ready());
        ene.reset();
        assert!(!ene.is_ready());
        assert_eq!(ene.count(), 0);
    }

    #[test]
    fn test_streaming_ene_meta() {
        let ene = StreamingEne::new(20, 11.0, 9.0);
        assert_eq!(StreamingEne::name(), "ENE");
        assert_eq!(StreamingEne::category(), "overlap");
        assert_eq!(ene.warm_up_period(), 20);
    }
}
