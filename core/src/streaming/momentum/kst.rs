use crate::streaming::momentum::roc::StreamingRoc;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KstOutput {
    pub kst: f64,
    pub signal: f64,
}

/// Streaming Know Sure Thing (KST).
///
/// KST = w1*SMA(ROC(roc1),sma1) + w2*SMA(ROC(roc2),sma2) + w3*SMA(ROC(roc3),sma3) + w4*SMA(ROC(roc4),sma4)
/// Signal = SMA(KST, signal_period)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingKst {
    roc1: StreamingRoc,
    roc2: StreamingRoc,
    roc3: StreamingRoc,
    roc4: StreamingRoc,
    sma1: StreamingSma,
    sma2: StreamingSma,
    sma3: StreamingSma,
    sma4: StreamingSma,
    signal_sma: StreamingSma,
    roc_periods: [usize; 4],
    sma_periods: [usize; 4],
    signal_period: usize,
    count: usize,
    last_value: Option<KstOutput>,
}

impl StreamingKst {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        roc1: usize, roc2: usize, roc3: usize, roc4: usize,
        sma1: usize, sma2: usize, sma3: usize, sma4: usize,
        signal_period: usize,
    ) -> Self {
        Self {
            roc1: StreamingRoc::new(roc1),
            roc2: StreamingRoc::new(roc2),
            roc3: StreamingRoc::new(roc3),
            roc4: StreamingRoc::new(roc4),
            sma1: StreamingSma::new(sma1),
            sma2: StreamingSma::new(sma2),
            sma3: StreamingSma::new(sma3),
            sma4: StreamingSma::new(sma4),
            signal_sma: StreamingSma::new(signal_period),
            roc_periods: [roc1, roc2, roc3, roc4],
            sma_periods: [sma1, sma2, sma3, sma4],
            signal_period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, KstOutput> for StreamingKst {
    #[inline]
    fn next(&mut self, input: f64) -> Option<KstOutput> {
        self.count += 1;

        let r1 = self.roc1.next(input);
        let r2 = self.roc2.next(input);
        let r3 = self.roc3.next(input);
        let r4 = self.roc4.next(input);

        let s1 = self.sma1.next(r1.map(|v| if v.is_nan() { 0.0 } else { v }).unwrap_or(0.0));
        let s2 = self.sma2.next(r2.map(|v| if v.is_nan() { 0.0 } else { v }).unwrap_or(0.0));
        let s3 = self.sma3.next(r3.map(|v| if v.is_nan() { 0.0 } else { v }).unwrap_or(0.0));
        let s4 = self.sma4.next(r4.map(|v| if v.is_nan() { 0.0 } else { v }).unwrap_or(0.0));

        let all_ready = r1.is_some() && r2.is_some() && r3.is_some() && r4.is_some()
            && s1.is_some() && s2.is_some() && s3.is_some() && s4.is_some();

        if !all_ready {
            self.last_value = None;
            return None;
        }

        let kst_val = s1.unwrap() * 1.0 + s2.unwrap() * 2.0 + s3.unwrap() * 3.0 + s4.unwrap() * 4.0;
        let sig = self.signal_sma.next(kst_val);

        match sig {
            Some(signal) => {
                let result = Some(KstOutput { kst: kst_val, signal });
                self.last_value = result;
                result
            }
            None => {
                self.last_value = None;
                None
            }
        }
    }

    fn reset(&mut self) {
        self.roc1.reset();
        self.roc2.reset();
        self.roc3.reset();
        self.roc4.reset();
        self.sma1.reset();
        self.sma2.reset();
        self.sma3.reset();
        self.sma4.reset();
        self.signal_sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.signal_sma.is_ready()
    }

        impl_standard_methods!(output = KstOutput);


}

impl IndicatorMeta for StreamingKst {
    fn name() -> &'static str { "KST" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Know Sure Thing" }
    fn warm_up_period(&self) -> usize {
        let max_roc_sma = self.roc_periods.iter()
            .zip(self.sma_periods.iter())
            .map(|(&r, &s)| r + s)
            .max()
            .unwrap_or(0);
        max_roc_sma + self.signal_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_kst_basic() {
        let mut kst = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 20.0)
            .collect();
        let mut last = None;
        for &v in &data {
            last = kst.next(v);
        }
        assert!(last.is_some());
        assert!(kst.is_ready());
    }

    #[test]
    fn test_streaming_kst_meta() {
        let _kst = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
        assert_eq!(StreamingKst::name(), "KST");
        assert_eq!(StreamingKst::category(), "momentum");
    }

    #[test]
    fn test_streaming_kst_reset() {
        let mut kst = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
        for i in 0..100 {
            kst.next(i as f64 + 1.0);
        }
        assert!(kst.is_ready());
        kst.reset();
        assert!(!kst.is_ready());
        assert_eq!(kst.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..200)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 20.0)
            .collect();

        let batch = crate::indicators::momentum_ext::kst(
            &data, 10, 15, 20, 30, 10, 10, 10, 15, 9,
        ).unwrap();

        let mut streaming = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
        for (i, &val) in data.iter().enumerate() {
            if let Some(out) = streaming.next(val) {
                if !batch.kst[i].is_nan() {
                    assert!(
                        (out.kst - batch.kst[i]).abs() < 1e-6,
                        "KST mismatch at {i}: streaming={}, batch={}",
                        out.kst, batch.kst[i]
                    );
                }
                if !batch.signal[i].is_nan() {
                    assert!(
                        (out.signal - batch.signal[i]).abs() < 1e-6,
                        "Signal mismatch at {i}: streaming={}, batch={}",
                        out.signal, batch.signal[i]
                    );
                }
            }
        }
    }
}
