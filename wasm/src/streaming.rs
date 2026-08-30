use wasm_bindgen::prelude::*;
use finkit::streaming::StreamingIndicator;
use finkit::streaming::indicators::*;
use finkit::streaming::OhlcvBar;

macro_rules! wasm_streaming_f64 {
    ($name:ident, $core:ty, $doc:expr) => {
        #[doc = $doc]
        #[wasm_bindgen]
        pub struct $name { inner: $core }

        #[wasm_bindgen]
        impl $name {
            #[wasm_bindgen(constructor)]
            pub fn new(period: usize) -> Self { Self { inner: <$core>::new(period) } }
            pub fn update(&mut self, value: f64) -> f64 {
                StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
            }
            pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
            #[wasm_bindgen(js_name = "isReady")]
            pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
            pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
        }
    };
}

wasm_streaming_f64!(WasmStreamingSma, StreamingSma, "Streaming SMA");
wasm_streaming_f64!(WasmStreamingEma, StreamingEma, "Streaming EMA");
wasm_streaming_f64!(WasmStreamingWma, StreamingWma, "Streaming WMA");
wasm_streaming_f64!(WasmStreamingDema, StreamingDema, "Streaming DEMA");
wasm_streaming_f64!(WasmStreamingTema, StreamingTema, "Streaming TEMA");
wasm_streaming_f64!(WasmStreamingRsi, StreamingRsi, "Streaming RSI");
wasm_streaming_f64!(WasmStreamingMom, StreamingMom, "Streaming MOM");
wasm_streaming_f64!(WasmStreamingRoc, StreamingRoc, "Streaming ROC");
wasm_streaming_f64!(WasmStreamingKama, StreamingKama, "Streaming KAMA");
wasm_streaming_f64!(WasmStreamingT3, StreamingT3, "Streaming T3");
wasm_streaming_f64!(WasmStreamingHma, StreamingHma, "Streaming HMA");
wasm_streaming_f64!(WasmStreamingZlema, StreamingZlema, "Streaming ZLEMA");
wasm_streaming_f64!(WasmStreamingZscore, StreamingZscore, "Streaming Z-Score");
wasm_streaming_f64!(WasmStreamingStdDev, StreamingStdDev, "Streaming StdDev");
wasm_streaming_f64!(WasmStreamingVar, StreamingVar, "Streaming Variance");
wasm_streaming_f64!(WasmStreamingLinReg, StreamingLinReg, "Streaming Linear Regression");
wasm_streaming_f64!(WasmStreamingTsf, StreamingTsf, "Streaming Time Series Forecast");
wasm_streaming_f64!(WasmStreamingCmo, StreamingCmo, "Streaming CMO");
// PPO / TSI / APO / Coppock take 2-3 period args in their constructors, so they
// need bespoke wrappers instead of the wasm_streaming_f64! single-period macro.
wasm_streaming_f64!(WasmStreamingPsy, StreamingPsy, "Streaming Psychology Line");
wasm_streaming_f64!(WasmStreamingMcGinley, StreamingMcGinley, "Streaming McGinley Dynamic");
wasm_streaming_f64!(WasmStreamingEfficiencyRatio, StreamingEfficiencyRatio, "Streaming Efficiency Ratio");
wasm_streaming_f64!(WasmStreamingTrix, StreamingTrix, "Streaming TRIX");
wasm_streaming_f64!(WasmStreamingVolumeMomentum, StreamingVolumeMomentum, "Streaming Volume Momentum");
wasm_streaming_f64!(WasmStreamingVolumeRoc, StreamingVolumeRoc, "Streaming Volume ROC");

macro_rules! wasm_streaming_hlc_tuple {
    ($name:ident, $core:ty, $doc:expr) => {
        #[doc = $doc]
        #[wasm_bindgen]
        pub struct $name { inner: $core }

        #[wasm_bindgen]
        impl $name {
            #[wasm_bindgen(constructor)]
            pub fn new(period: usize) -> Self { Self { inner: <$core>::new(period) } }
            pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
                StreamingIndicator::next(&mut self.inner, (high, low, close)).unwrap_or(f64::NAN)
            }
            pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
            #[wasm_bindgen(js_name = "isReady")]
            pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
            pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
        }
    };
}

wasm_streaming_hlc_tuple!(WasmStreamingAtr, StreamingAtr, "Streaming ATR");
// NATR takes `&dyn Ohlcv` instead of a tuple, so it needs a bespoke wrapper.
wasm_streaming_hlc_tuple!(WasmStreamingAdx, StreamingAdx, "Streaming ADX");
wasm_streaming_hlc_tuple!(WasmStreamingDx, StreamingDx, "Streaming DX");
wasm_streaming_hlc_tuple!(WasmStreamingAdxr, StreamingAdxr, "Streaming ADXR");
wasm_streaming_hlc_tuple!(WasmStreamingMinusDi, StreamingMinusDi, "Streaming Minus DI");
wasm_streaming_hlc_tuple!(WasmStreamingPlusDi, StreamingPlusDi, "Streaming Plus DI");
wasm_streaming_hlc_tuple!(WasmStreamingChop, StreamingChop, "Streaming Chop");

// --- Bespoke wrappers for indicators whose constructors differ from the macro. ---

#[wasm_bindgen]
pub struct WasmStreamingNatr { inner: StreamingNatr }

#[wasm_bindgen]
impl WasmStreamingNatr {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingNatr::new(period) } }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, 0.0);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingPpo { inner: StreamingPpo }

#[wasm_bindgen]
impl WasmStreamingPpo {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self { inner: StreamingPpo::new(fast_period, slow_period) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingApo { inner: StreamingApo }

#[wasm_bindgen]
impl WasmStreamingApo {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self { inner: StreamingApo::new(fast_period, slow_period) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingTsi { inner: StreamingTsi }

#[wasm_bindgen]
impl WasmStreamingTsi {
    #[wasm_bindgen(constructor)]
    pub fn new(long_period: usize, short_period: usize) -> Self {
        Self { inner: StreamingTsi::new(long_period, short_period) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingCoppock { inner: StreamingCoppock }

#[wasm_bindgen]
impl WasmStreamingCoppock {
    #[wasm_bindgen(constructor)]
    pub fn new(wma_period: usize, long_roc: usize, short_roc: usize) -> Self {
        Self { inner: StreamingCoppock::new(wma_period, long_roc, short_roc) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmAroonOutput { pub aroon_up: f64, pub aroon_down: f64 }

#[wasm_bindgen]
pub struct WasmStreamingAroon { inner: StreamingAroon }

#[wasm_bindgen]
impl WasmStreamingAroon {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingAroon::new(period) } }
    pub fn update(&mut self, high: f64, low: f64) -> WasmAroonOutput {
        match StreamingIndicator::next(&mut self.inner, (high, low)) {
            Some(out) => WasmAroonOutput { aroon_up: out.aroon_up, aroon_down: out.aroon_down },
            None => WasmAroonOutput { aroon_up: f64::NAN, aroon_down: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingAroonOsc { inner: StreamingAroonOsc }

#[wasm_bindgen]
impl WasmStreamingAroonOsc {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingAroonOsc::new(period) } }
    pub fn update(&mut self, high: f64, low: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, (high, low)).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmBollOutput { pub upper: f64, pub middle: f64, pub lower: f64 }

#[wasm_bindgen]
pub struct WasmStreamingBoll { inner: StreamingBoll }

#[wasm_bindgen]
impl WasmStreamingBoll {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize, nb_dev_up: f64, nb_dev_dn: f64) -> Self { Self { inner: StreamingBoll::new(period, nb_dev_up, nb_dev_dn) } }
    pub fn update(&mut self, value: f64) -> WasmBollOutput {
        match StreamingIndicator::next(&mut self.inner, value) {
            Some(out) => WasmBollOutput { upper: out.upper, middle: out.middle, lower: out.lower },
            None => WasmBollOutput { upper: f64::NAN, middle: f64::NAN, lower: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmMacdOutput { pub macd: f64, pub signal: f64 }

#[wasm_bindgen]
pub struct WasmStreamingMacd { inner: StreamingMacd }

#[wasm_bindgen]
impl WasmStreamingMacd {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self { inner: StreamingMacd::new(fast_period, slow_period, signal_period) }
    }
    pub fn update(&mut self, value: f64) -> WasmMacdOutput {
        match StreamingIndicator::next(&mut self.inner, value) {
            Some(out) => WasmMacdOutput { macd: out.macd, signal: out.signal },
            None => WasmMacdOutput { macd: f64::NAN, signal: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmStochOutput { pub k: f64, pub d: f64 }

#[wasm_bindgen]
pub struct WasmStreamingStoch { inner: StreamingStoch }

#[wasm_bindgen]
impl WasmStreamingStoch {
    #[wasm_bindgen(constructor)]
    pub fn new(k_period: usize, k_slow: usize, d_period: usize) -> Self { Self { inner: StreamingStoch::new(k_period, k_slow, d_period) } }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> WasmStochOutput {
        match StreamingIndicator::next(&mut self.inner, (high, low, close)) {
            Some(out) => WasmStochOutput { k: out.k, d: out.d },
            None => WasmStochOutput { k: f64::NAN, d: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmSuperTrendOutput { pub supertrend: f64, pub direction: i32 }

#[wasm_bindgen]
pub struct WasmStreamingSuperTrend { inner: StreamingSuperTrend }

#[wasm_bindgen]
impl WasmStreamingSuperTrend {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self { inner: StreamingSuperTrend::new(period, multiplier) }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> WasmSuperTrendOutput {
        let bar = OhlcvBar::new(0.0, high, low, close, 0.0);
        match self.inner.next(&bar) {
            Some(out) => WasmSuperTrendOutput { supertrend: out.supertrend, direction: out.direction },
            None => WasmSuperTrendOutput { supertrend: f64::NAN, direction: 0 },
        }
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingObv { inner: StreamingObv }

#[wasm_bindgen]
impl WasmStreamingObv {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingObv::new() } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingVwap { inner: StreamingVwap }

#[wasm_bindgen]
impl WasmStreamingVwap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingVwap::new() } }
    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingJma { inner: StreamingJma }

#[wasm_bindgen]
impl WasmStreamingJma {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize, phase: f64, power: f64) -> Self {
        Self { inner: StreamingJma::new(period, phase, power) }
    }
    pub fn update(&mut self, value: f64) -> f64 { StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN) }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingQStick { inner: StreamingQStick }

#[wasm_bindgen]
impl WasmStreamingQStick {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingQStick::new(period) } }
    pub fn update(&mut self, open: f64, close: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, (open, close)).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmKstOutput { pub kst: f64, pub signal: f64 }

#[wasm_bindgen]
pub struct WasmStreamingKst { inner: StreamingKst }

#[wasm_bindgen]
impl WasmStreamingKst {
    #[wasm_bindgen(constructor)]
    pub fn new(roc1: usize, roc2: usize, roc3: usize, roc4: usize, sma1: usize, sma2: usize, sma3: usize, sma4: usize, sig_period: usize) -> Self {
        Self { inner: StreamingKst::new(roc1, roc2, roc3, roc4, sma1, sma2, sma3, sma4, sig_period) }
    }
    pub fn update(&mut self, value: f64) -> WasmKstOutput {
        match StreamingIndicator::next(&mut self.inner, value) {
            Some(out) => WasmKstOutput { kst: out.kst, signal: out.signal },
            None => WasmKstOutput { kst: f64::NAN, signal: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingStc { inner: StreamingStc }

#[wasm_bindgen]
impl WasmStreamingStc {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize, cycle: usize) -> Self {
        Self { inner: StreamingStc::new(fast_period, slow_period, cycle) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmVortexOutput { pub vi_plus: f64, pub vi_minus: f64 }

#[wasm_bindgen]
pub struct WasmStreamingVortex { inner: StreamingVortex }

#[wasm_bindgen]
impl WasmStreamingVortex {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingVortex::new(period) } }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> WasmVortexOutput {
        match StreamingIndicator::next(&mut self.inner, (high, low, close)) {
            Some(out) => WasmVortexOutput { vi_plus: out.vi_plus, vi_minus: out.vi_minus },
            None => WasmVortexOutput { vi_plus: f64::NAN, vi_minus: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmKeltnerOutput { pub upper: f64, pub middle: f64, pub lower: f64 }

#[wasm_bindgen]
pub struct WasmStreamingKeltner { inner: StreamingKeltner }

#[wasm_bindgen]
impl WasmStreamingKeltner {
    #[wasm_bindgen(constructor)]
    pub fn new(ema_period: usize, atr_period: usize, multiplier: f64) -> Self {
        Self { inner: StreamingKeltner::new(ema_period, atr_period, multiplier) }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> WasmKeltnerOutput {
        let bar = OhlcvBar::new(0.0, high, low, close, 0.0);
        match self.inner.next(&bar) {
            Some(out) => WasmKeltnerOutput { upper: out.upper, middle: out.middle, lower: out.lower },
            None => WasmKeltnerOutput { upper: f64::NAN, middle: f64::NAN, lower: f64::NAN },
        }
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmRviOutput { pub rvi: f64, pub signal: f64 }

#[wasm_bindgen]
pub struct WasmStreamingRvi { inner: StreamingRvi }

#[wasm_bindgen]
impl WasmStreamingRvi {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingRvi::new(period) } }
    pub fn update(&mut self, high: f64, low: f64, close: f64, open: f64) -> WasmRviOutput {
        let bar = OhlcvBar::new(open, high, low, close, 0.0);
        match self.inner.next(&bar) {
            Some(out) => WasmRviOutput { rvi: out.rvi, signal: out.signal },
            None => WasmRviOutput { rvi: f64::NAN, signal: f64::NAN },
        }
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingStochRsi { inner: StreamingStochRsi }

#[wasm_bindgen]
impl WasmStreamingStochRsi {
    #[wasm_bindgen(constructor)]
    pub fn new(rsi_period: usize, stoch_period: usize, k_smooth: usize, d_smooth: usize) -> Self {
        Self { inner: StreamingStochRsi::new(rsi_period, stoch_period, k_smooth, d_smooth) }
    }
    pub fn update(&mut self, value: f64) -> f64 {
        StreamingIndicator::next(&mut self.inner, value).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmStochFOutput { pub k: f64, pub d: f64 }

#[wasm_bindgen]
pub struct WasmStreamingStochF { inner: StreamingStochF }

#[wasm_bindgen]
impl WasmStreamingStochF {
    #[wasm_bindgen(constructor)]
    pub fn new(k_period: usize, d_period: usize) -> Self {
        Self { inner: StreamingStochF::new(k_period, d_period) }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> WasmStochFOutput {
        match StreamingIndicator::next(&mut self.inner, (high, low, close)) {
            Some(out) => WasmStochFOutput { k: out.k, d: out.d },
            None => WasmStochFOutput { k: f64::NAN, d: f64::NAN },
        }
    }
    pub fn reset(&mut self) { StreamingIndicator::reset(&mut self.inner); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { StreamingIndicator::is_ready(&self.inner) }
    pub fn count(&self) -> usize { StreamingIndicator::count(&self.inner) }
}

#[wasm_bindgen]
pub struct WasmStreamingForceIndex { inner: StreamingForceIndex }

#[wasm_bindgen]
impl WasmStreamingForceIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingForceIndex::new(period) } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen(getter_with_clone)]
pub struct WasmKvoOutput { pub kvo: f64, pub signal: f64 }

#[wasm_bindgen]
pub struct WasmStreamingKvo { inner: StreamingKvo }

#[wasm_bindgen]
impl WasmStreamingKvo {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self { inner: StreamingKvo::new(fast_period, slow_period, signal_period) }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> WasmKvoOutput {
        let bar = OhlcvBar::new(0.0, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => WasmKvoOutput { kvo: out.kvo, signal: out.signal },
            None => WasmKvoOutput { kvo: f64::NAN, signal: f64::NAN },
        }
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingNvi { inner: StreamingNvi }

#[wasm_bindgen]
impl WasmStreamingNvi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingNvi::new() } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingPvi { inner: StreamingPvi }

#[wasm_bindgen]
impl WasmStreamingPvi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingPvi::new() } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingPvt { inner: StreamingPvt }

#[wasm_bindgen]
impl WasmStreamingPvt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingPvt::new() } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingAnchoredVwap { inner: StreamingAnchoredVwap }

#[wasm_bindgen]
impl WasmStreamingAnchoredVwap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: StreamingAnchoredVwap::new() }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingVwma { inner: StreamingVwma }

#[wasm_bindgen]
impl WasmStreamingVwma {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Self { Self { inner: StreamingVwma::new(period) } }
    pub fn update(&mut self, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingAvgPrice { inner: StreamingAvgPrice }

#[wasm_bindgen]
impl WasmStreamingAvgPrice {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingAvgPrice::new() } }
    pub fn update(&mut self, open: f64, high: f64, low: f64, close: f64) -> f64 {
        let bar = OhlcvBar::new(open, high, low, close, 0.0);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingMedPrice { inner: StreamingMedPrice }

#[wasm_bindgen]
impl WasmStreamingMedPrice {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingMedPrice::new() } }
    pub fn update(&mut self, high: f64, low: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, 0.0, 0.0);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingTypPrice { inner: StreamingTypPrice }

#[wasm_bindgen]
impl WasmStreamingTypPrice {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingTypPrice::new() } }
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, 0.0);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingAd { inner: StreamingAd }

#[wasm_bindgen]
impl WasmStreamingAd {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: StreamingAd::new() } }
    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}

#[wasm_bindgen]
pub struct WasmStreamingAdosc { inner: StreamingAdosc }

#[wasm_bindgen]
impl WasmStreamingAdosc {
    #[wasm_bindgen(constructor)]
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self { inner: StreamingAdosc::new(fast_period, slow_period) }
    }
    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(0.0, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }
    pub fn reset(&mut self) { self.inner.reset(); }
    #[wasm_bindgen(js_name = "isReady")]
    pub fn is_ready(&self) -> bool { self.inner.is_ready() }
    pub fn count(&self) -> usize { self.inner.count() }
}