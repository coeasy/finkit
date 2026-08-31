use finkit::streaming::OhlcvBar;
use finkit::streaming::StreamingIndicator;
use napi_derive::napi;

// ============================================================================
// Category 1: f64 → f64 (single-period constructor)
// ============================================================================

macro_rules! napi_streaming_f64 {
    ($napi_name:ident, $core_mod:ident, $core_type:ident) => {
        #[napi]
        pub struct $napi_name {
            inner: finkit::streaming::indicators::$core_type,
        }

        #[napi]
        impl $napi_name {
            #[napi(constructor)]
            pub fn new(period: u32) -> Self {
                Self {
                    inner: finkit::streaming::indicators::$core_type::new(period as usize),
                }
            }

            #[napi]
            pub fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            #[napi]
            pub fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            #[napi]
            pub fn reset(&mut self) {
                self.inner.reset();
            }

            #[napi(getter)]
            pub fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            #[napi(getter)]
            pub fn count(&self) -> u32 {
                self.inner.count() as u32
            }
        }
    };
}

napi_streaming_f64!(NapiStreamingSma, sma, StreamingSma);
napi_streaming_f64!(NapiStreamingEma, ema, StreamingEma);
napi_streaming_f64!(NapiStreamingWma, wma, StreamingWma);
napi_streaming_f64!(NapiStreamingDema, dema, StreamingDema);
napi_streaming_f64!(NapiStreamingTema, tema, StreamingTema);
napi_streaming_f64!(NapiStreamingKama, kama, StreamingKama);
napi_streaming_f64!(NapiStreamingT3, t3, StreamingT3);
napi_streaming_f64!(NapiStreamingRsi, rsi, StreamingRsi);
napi_streaming_f64!(NapiStreamingMom, mom, StreamingMom);
napi_streaming_f64!(NapiStreamingRoc, roc, StreamingRoc);

// ============================================================================
// Category 2: f64 → struct (multi-output)
// ============================================================================

#[napi(object)]
pub struct MacdResult {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

#[napi]
pub struct NapiStreamingMacd {
    inner: finkit::streaming::indicators::StreamingMacd,
}

#[napi]
impl NapiStreamingMacd {
    #[napi(constructor)]
    pub fn new(
        fast_period: Option<u32>,
        slow_period: Option<u32>,
        signal_period: Option<u32>,
    ) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingMacd::new(
                fast_period.unwrap_or(12) as usize,
                slow_period.unwrap_or(26) as usize,
                signal_period.unwrap_or(9) as usize,
            ),
        }
    }

    #[napi]
    pub fn update(&mut self, value: f64) -> MacdResult {
        match self.inner.next(value) {
            Some(out) => MacdResult {
                macd: out.macd,
                signal: out.signal,
                histogram: out.histogram,
            },
            None => MacdResult {
                macd: f64::NAN,
                signal: f64::NAN,
                histogram: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

#[napi(object)]
pub struct BollResult {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[napi]
pub struct NapiStreamingBoll {
    inner: finkit::streaming::indicators::StreamingBoll,
}

#[napi]
impl NapiStreamingBoll {
    #[napi(constructor)]
    pub fn new(period: Option<u32>, nb_dev_up: Option<f64>, nb_dev_dn: Option<f64>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingBoll::new(
                period.unwrap_or(20) as usize,
                nb_dev_up.unwrap_or(2.0),
                nb_dev_dn.unwrap_or(2.0),
            ),
        }
    }

    #[napi]
    pub fn update(&mut self, value: f64) -> BollResult {
        match self.inner.next(value) {
            Some(out) => BollResult {
                upper: out.upper,
                middle: out.middle,
                lower: out.lower,
            },
            None => BollResult {
                upper: f64::NAN,
                middle: f64::NAN,
                lower: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

// ============================================================================
// Category 3: (high, low, close) → f64
// ============================================================================

macro_rules! napi_streaming_hlc {
    ($napi_name:ident, $core_mod:ident, $core_type:ident) => {
        #[napi]
        pub struct $napi_name {
            inner: finkit::streaming::indicators::$core_type,
        }

        #[napi]
        impl $napi_name {
            #[napi(constructor)]
            pub fn new(period: u32) -> Self {
                Self {
                    inner: finkit::streaming::indicators::$core_type::new(period as usize),
                }
            }

            #[napi]
            pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
                self.inner.next((high, low, close)).unwrap_or(f64::NAN)
            }

            #[napi]
            pub fn reset(&mut self) {
                self.inner.reset();
            }

            #[napi(getter)]
            pub fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            #[napi(getter)]
            pub fn count(&self) -> u32 {
                self.inner.count() as u32
            }
        }
    };
}

napi_streaming_hlc!(NapiStreamingAtr, atr, StreamingAtr);
napi_streaming_hlc!(NapiStreamingAdx, adx, StreamingAdx);
napi_streaming_hlc!(NapiStreamingCci, cci, StreamingCci);

// ============================================================================
// Category 4: (high, low, close) → struct
// ============================================================================

#[napi(object)]
pub struct StochResult {
    pub k: f64,
    pub d: f64,
}

#[napi]
pub struct NapiStreamingStoch {
    inner: finkit::streaming::indicators::StreamingStoch,
}

#[napi]
impl NapiStreamingStoch {
    #[napi(constructor)]
    pub fn new(k_period: Option<u32>, k_slow: Option<u32>, d_period: Option<u32>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingStoch::new(
                k_period.unwrap_or(14) as usize,
                k_slow.unwrap_or(3) as usize,
                d_period.unwrap_or(3) as usize,
            ),
        }
    }

    #[napi]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> StochResult {
        match self.inner.next((high, low, close)) {
            Some(out) => StochResult { k: out.k, d: out.d },
            None => StochResult {
                k: f64::NAN,
                d: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

// ============================================================================
// Category 5: (high, low) → struct
// ============================================================================

#[napi(object)]
pub struct AroonResult {
    pub up: f64,
    pub down: f64,
}

#[napi]
pub struct NapiStreamingAroon {
    inner: finkit::streaming::indicators::StreamingAroon,
}

#[napi]
impl NapiStreamingAroon {
    #[napi(constructor)]
    pub fn new(period: Option<u32>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingAroon::new(period.unwrap_or(14) as usize),
        }
    }

    #[napi]
    pub fn update(&mut self, high: f64, low: f64) -> AroonResult {
        match self.inner.next((high, low)) {
            Some(out) => AroonResult {
                up: out.aroon_up,
                down: out.aroon_down,
            },
            None => AroonResult {
                up: f64::NAN,
                down: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

// ============================================================================
// Category 6: OHLCV → f64
// ============================================================================

#[napi]
pub struct NapiStreamingObv {
    inner: finkit::streaming::indicators::StreamingObv,
}

#[napi]
impl NapiStreamingObv {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingObv::new(),
        }
    }

    #[napi]
    pub fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

#[napi]
pub struct NapiStreamingVwap {
    inner: finkit::streaming::indicators::StreamingVwap,
}

#[napi]
impl NapiStreamingVwap {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingVwap::new(),
        }
    }

    #[napi]
    pub fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

macro_rules! napi_streaming_ohlcv {
    ($napi_name:ident, $core_mod:ident, $core_type:ident) => {
        #[napi]
        pub struct $napi_name {
            inner: finkit::streaming::indicators::$core_type,
        }

        #[napi]
        impl $napi_name {
            #[napi(constructor)]
            pub fn new(period: u32) -> Self {
                Self {
                    inner: finkit::streaming::indicators::$core_type::new(period as usize),
                }
            }

            #[napi]
            pub fn update(
                &mut self,
                open: f64,
                high: f64,
                low: f64,
                close: f64,
                volume: f64,
            ) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            #[napi]
            pub fn reset(&mut self) {
                self.inner.reset();
            }

            #[napi(getter)]
            pub fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            #[napi(getter)]
            pub fn count(&self) -> u32 {
                self.inner.count() as u32
            }
        }
    };
}

napi_streaming_ohlcv!(NapiStreamingWillr, willr, StreamingWillR);
napi_streaming_ohlcv!(NapiStreamingMfi, mfi, StreamingMfi);
napi_streaming_ohlcv!(NapiStreamingNatr, natr, StreamingNatr);

#[napi]
pub struct NapiStreamingTrange {
    inner: finkit::streaming::indicators::StreamingTrange,
}

#[napi]
impl NapiStreamingTrange {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingTrange::new(),
        }
    }

    #[napi]
    pub fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _ = (open, volume);
        self.inner.next((high, low, close)).unwrap_or(f64::NAN)
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

// ============================================================================
// Category 7: OHLCV → struct
// ============================================================================

#[napi(object)]
pub struct DonchianResult {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[napi]
pub struct NapiStreamingDonchian {
    inner: finkit::streaming::indicators::StreamingDonchian,
}

#[napi]
impl NapiStreamingDonchian {
    #[napi(constructor)]
    pub fn new(period: Option<u32>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingDonchian::new(
                period.unwrap_or(20) as usize
            ),
        }
    }

    #[napi]
    pub fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> DonchianResult {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => DonchianResult {
                upper: out.upper,
                middle: out.middle,
                lower: out.lower,
            },
            None => DonchianResult {
                upper: f64::NAN,
                middle: f64::NAN,
                lower: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

#[napi(object)]
pub struct IchimokuResult {
    pub tenkan: f64,
    pub kijun: f64,
    pub senkou_a: f64,
    pub senkou_b: f64,
    pub chikou: f64,
}

#[napi]
pub struct NapiStreamingIchimoku {
    inner: finkit::streaming::indicators::StreamingIchimoku,
}

#[napi]
impl NapiStreamingIchimoku {
    #[napi(constructor)]
    pub fn new(tenkan: Option<u32>, kijun: Option<u32>, senkou_b: Option<u32>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingIchimoku::new(
                tenkan.unwrap_or(9) as usize,
                kijun.unwrap_or(26) as usize,
                senkou_b.unwrap_or(52) as usize,
            ),
        }
    }

    #[napi]
    pub fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IchimokuResult {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => IchimokuResult {
                tenkan: out.tenkan,
                kijun: out.kijun,
                senkou_a: out.senkou_a,
                senkou_b: out.senkou_b,
                chikou: out.chikou,
            },
            None => IchimokuResult {
                tenkan: f64::NAN,
                kijun: f64::NAN,
                senkou_a: f64::NAN,
                senkou_b: f64::NAN,
                chikou: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

#[napi(object)]
pub struct SuperTrendResult {
    pub supertrend: f64,
    pub direction: i32,
}

#[napi]
pub struct NapiStreamingSupertrend {
    inner: finkit::streaming::indicators::StreamingSuperTrend,
}

#[napi]
impl NapiStreamingSupertrend {
    #[napi(constructor)]
    pub fn new(period: Option<u32>, multiplier: Option<f64>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingSuperTrend::new(
                period.unwrap_or(10) as usize,
                multiplier.unwrap_or(3.0),
            ),
        }
    }

    #[napi]
    pub fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> SuperTrendResult {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => SuperTrendResult {
                supertrend: out.supertrend,
                direction: out.direction,
            },
            None => SuperTrendResult {
                supertrend: f64::NAN,
                direction: 0,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}

#[napi(object)]
pub struct KeltnerResult {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[napi]
pub struct NapiStreamingKeltner {
    inner: finkit::streaming::indicators::StreamingKeltner,
}

#[napi]
impl NapiStreamingKeltner {
    #[napi(constructor)]
    pub fn new(ema_period: Option<u32>, atr_period: Option<u32>, multiplier: Option<f64>) -> Self {
        Self {
            inner: finkit::streaming::indicators::StreamingKeltner::new(
                ema_period.unwrap_or(20) as usize,
                atr_period.unwrap_or(10) as usize,
                multiplier.unwrap_or(2.0),
            ),
        }
    }

    #[napi]
    pub fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> KeltnerResult {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => KeltnerResult {
                upper: out.upper,
                middle: out.middle,
                lower: out.lower,
            },
            None => KeltnerResult {
                upper: f64::NAN,
                middle: f64::NAN,
                lower: f64::NAN,
            },
        }
    }

    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[napi(getter)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    #[napi(getter)]
    pub fn count(&self) -> u32 {
        self.inner.count() as u32
    }
}
