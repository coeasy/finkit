use crate::error::IndicatorError;

/// Trait for building streaming indicators via the builder pattern.
///
/// Each streaming indicator provides a type-safe builder with named setters
/// for its parameters. Invalid parameters produce
/// [`IndicatorError::InvalidParameter`] at build time.
///
/// # Example
///
/// ```
/// use finkit::streaming::builder::{IndicatorBuilder, Builder};
/// use finkit::streaming::indicators::StreamingSma;
/// use finkit::streaming::IndicatorMeta;
///
/// let sma = StreamingSma::builder()
///     .period(20)
///     .build()
///     .unwrap();
/// assert!(sma.warm_up_period() == 20);
/// ```
pub trait IndicatorBuilder: Sized {
    /// The concrete builder type.
    type Builder: Builder<Output = Self>;

    /// Create a new builder with default (unset) parameters.
    fn builder() -> Self::Builder;
}

/// A builder that can produce an indicator or an error.
pub trait Builder {
    type Output;

    /// Consume the builder and produce the indicator, validating parameters.
    fn build(self) -> Result<Self::Output, IndicatorError>;
}

// ---------------------------------------------------------------------------
// Macros to generate builders for common constructor patterns
// ---------------------------------------------------------------------------

/// Builder for indicators with `new(period: usize)`.
macro_rules! impl_builder_single_period {
    ($indicator:ty, $builder_param:ident) => {
        #[derive(Default, Debug)]
        pub struct $builder_param {
            period: Option<usize>,
        }

        impl $builder_param {
            pub fn period(mut self, v: usize) -> Self {
                self.period = Some(v);
                self
            }
        }

        impl Builder for $builder_param {
            type Output = $indicator;

            fn build(self) -> Result<Self::Output, IndicatorError> {
                let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: "period".into(),
                    reason: "must be set".into(),
                })?;
                if period == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: "period".into(),
                        reason: "> 0".into(),
                    });
                }
                Ok(<$indicator>::new(period))
            }
        }

        impl IndicatorBuilder for $indicator {
            type Builder = $builder_param;
            fn builder() -> Self::Builder {
                $builder_param::default()
            }
        }
    };
}

/// Builder for indicators with no parameters: `new()`.
macro_rules! impl_builder_no_params {
    ($indicator:ty, $builder_param:ident) => {
        #[derive(Default, Debug)]
        pub struct $builder_param;

        impl Builder for $builder_param {
            type Output = $indicator;

            fn build(self) -> Result<Self::Output, IndicatorError> {
                Ok(<$indicator>::new())
            }
        }

        impl IndicatorBuilder for $indicator {
            type Builder = $builder_param;
            fn builder() -> Self::Builder {
                $builder_param
            }
        }
    };
}

/// Builder for indicators with `new(fast_period, slow_period)` (two usize).
macro_rules! impl_builder_two_periods {
    ($indicator:ty, $builder_param:ident, $p1:ident, $p2:ident) => {
        #[derive(Default, Debug)]
        pub struct $builder_param {
            $p1: Option<usize>,
            $p2: Option<usize>,
        }

        impl $builder_param {
            pub fn $p1(mut self, v: usize) -> Self {
                self.$p1 = Some(v);
                self
            }
            pub fn $p2(mut self, v: usize) -> Self {
                self.$p2 = Some(v);
                self
            }
        }

        impl Builder for $builder_param {
            type Output = $indicator;

            fn build(self) -> Result<Self::Output, IndicatorError> {
                let p1 = self.$p1.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($p1).into(),
                    reason: "must be set".into(),
                })?;
                let p2 = self.$p2.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($p2).into(),
                    reason: "must be set".into(),
                })?;
                if p1 == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: stringify!($p1).into(),
                        reason: "> 0".into(),
                    });
                }
                if p2 == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: stringify!($p2).into(),
                        reason: "> 0".into(),
                    });
                }
                Ok(<$indicator>::new(p1, p2))
            }
        }

        impl IndicatorBuilder for $indicator {
            type Builder = $builder_param;
            fn builder() -> Self::Builder {
                $builder_param::default()
            }
        }
    };
}

/// Builder for indicators with `new(p1, p2, p3)` (three usize).
macro_rules! impl_builder_three_periods {
    ($indicator:ty, $builder_param:ident, $p1:ident, $p2:ident, $p3:ident) => {
        #[derive(Default, Debug)]
        pub struct $builder_param {
            $p1: Option<usize>,
            $p2: Option<usize>,
            $p3: Option<usize>,
        }

        impl $builder_param {
            pub fn $p1(mut self, v: usize) -> Self {
                self.$p1 = Some(v);
                self
            }
            pub fn $p2(mut self, v: usize) -> Self {
                self.$p2 = Some(v);
                self
            }
            pub fn $p3(mut self, v: usize) -> Self {
                self.$p3 = Some(v);
                self
            }
        }

        impl Builder for $builder_param {
            type Output = $indicator;

            fn build(self) -> Result<Self::Output, IndicatorError> {
                let p1 = self.$p1.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($p1).into(),
                    reason: "must be set".into(),
                })?;
                let p2 = self.$p2.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($p2).into(),
                    reason: "must be set".into(),
                })?;
                let p3 = self.$p3.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($p3).into(),
                    reason: "must be set".into(),
                })?;
                if p1 == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: stringify!($p1).into(),
                        reason: "> 0".into(),
                    });
                }
                if p2 == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: stringify!($p2).into(),
                        reason: "> 0".into(),
                    });
                }
                if p3 == 0 {
                    return Err(IndicatorError::InvalidParameter {
                        param: stringify!($p3).into(),
                        reason: "> 0".into(),
                    });
                }
                Ok(<$indicator>::new(p1, p2, p3))
            }
        }

        impl IndicatorBuilder for $indicator {
            type Builder = $builder_param;
            fn builder() -> Self::Builder {
                $builder_param::default()
            }
        }
    };
}

// ---------------------------------------------------------------------------
// 0 params (9 indicators)
// ---------------------------------------------------------------------------

use super::indicators::StreamingAvgPrice;
use super::indicators::StreamingMedPrice;
use super::indicators::StreamingNvi;
use super::indicators::StreamingObv;
use super::indicators::StreamingPvi;
use super::indicators::StreamingPvt;
use super::indicators::StreamingTrange;
use super::indicators::StreamingTypPrice;
use super::indicators::StreamingVwap;

impl_builder_no_params!(StreamingAd, AdBuilder);
impl_builder_no_params!(StreamingAnchoredVwap, AnchoredVwapBuilder);
impl_builder_no_params!(StreamingAvgPrice, AvgPriceBuilder);
impl_builder_no_params!(StreamingHtDcPeriod, HtDcPeriodBuilder);
impl_builder_no_params!(StreamingHtDcPhase, HtDcPhaseBuilder);
impl_builder_no_params!(StreamingHtSine, HtSineBuilder);
impl_builder_no_params!(StreamingHtTrendline, HtTrendlineBuilder);
impl_builder_no_params!(StreamingHtTrendMode, HtTrendModeBuilder);
impl_builder_no_params!(StreamingMedPrice, MedPriceBuilder);
impl_builder_no_params!(StreamingNvi, NviBuilder);
impl_builder_no_params!(StreamingObv, ObvBuilder);
impl_builder_no_params!(StreamingPvi, PviBuilder);
impl_builder_no_params!(StreamingPvt, PvtBuilder);
impl_builder_no_params!(StreamingTrange, TrangeBuilder);
impl_builder_no_params!(StreamingTypPrice, TypPriceBuilder);
impl_builder_no_params!(StreamingVwap, VwapBuilder);

// ---------------------------------------------------------------------------
// 1 param — period: usize (39 indicators)
// ---------------------------------------------------------------------------

use super::indicators::StreamingAdx;
use super::indicators::StreamingAdxr;
use super::indicators::StreamingAr;
use super::indicators::StreamingAroon;
use super::indicators::StreamingAroonOsc;
use super::indicators::StreamingAtr;
use super::indicators::StreamingBias;
use super::indicators::StreamingBr;
use super::indicators::StreamingCci;
use super::indicators::StreamingChop;
use super::indicators::StreamingCmo;
use super::indicators::StreamingCmf;
use super::indicators::StreamingCr;
use super::indicators::StreamingDema;
use super::indicators::StreamingDonchian;
use super::indicators::StreamingDpo;
use super::indicators::StreamingDx;
use super::indicators::StreamingEma;
use super::indicators::StreamingEom;
use super::indicators::StreamingFisher;
use super::indicators::StreamingForceIndex;
use super::indicators::StreamingHma;
use super::indicators::StreamingKama;
use super::indicators::StreamingLinReg;
use super::indicators::StreamingMcGinley;
use super::indicators::StreamingMfi;
use super::indicators::StreamingMinusDi;
use super::indicators::StreamingMom;
use super::indicators::StreamingNatr;
use super::indicators::StreamingPlusDi;
use super::indicators::StreamingPsy;
use super::indicators::StreamingRoc;
use super::indicators::StreamingRsi;
use super::indicators::StreamingRvi;
use super::indicators::StreamingSma;
use super::indicators::StreamingT3;
use super::indicators::StreamingTema;
use super::indicators::StreamingTrix;
use super::indicators::StreamingUlcerIndex;
use super::indicators::StreamingVr;
use super::indicators::StreamingVwma;
use super::indicators::StreamingWillR;
use super::indicators::StreamingWma;
use super::indicators::StreamingZlema;
use super::indicators::StreamingZscore;

impl_builder_single_period!(StreamingAdx, AdxBuilder);
impl_builder_single_period!(StreamingAdxr, AdxrBuilder);
impl_builder_single_period!(StreamingAr, ArBuilder);
impl_builder_single_period!(StreamingAroon, AroonBuilder);
impl_builder_single_period!(StreamingAroonOsc, AroonOscBuilder);
impl_builder_single_period!(StreamingAtr, AtrBuilder);
impl_builder_single_period!(StreamingBias, BiasBuilder);
impl_builder_single_period!(StreamingBr, BrBuilder);
impl_builder_single_period!(StreamingCci, CciBuilder);
impl_builder_single_period!(StreamingCmo, CmoBuilder);
impl_builder_single_period!(StreamingChop, ChopBuilder);
impl_builder_single_period!(StreamingCmf, CmfBuilder);
impl_builder_single_period!(StreamingCr, CrBuilder);
impl_builder_single_period!(StreamingDema, DemaBuilder);
impl_builder_single_period!(StreamingDonchian, DonchianBuilder);
impl_builder_single_period!(StreamingDpo, DpoBuilder);
impl_builder_single_period!(StreamingDx, DxBuilder);
impl_builder_single_period!(StreamingEma, EmaBuilder);
impl_builder_single_period!(StreamingEom, EomBuilder);
impl_builder_single_period!(StreamingFisher, FisherBuilder);
impl_builder_single_period!(StreamingForceIndex, ForceIndexBuilder);
impl_builder_single_period!(StreamingHma, HmaBuilder);
impl_builder_single_period!(StreamingKama, KamaBuilder);
impl_builder_single_period!(StreamingBeta, BetaBuilder);
impl_builder_single_period!(StreamingCorrel, CorrelBuilder);
impl_builder_single_period!(StreamingLinReg, LinRegBuilder);
impl_builder_single_period!(StreamingLinRegAngle, LinRegAngleBuilder);
impl_builder_single_period!(StreamingLinRegIntercept, LinRegInterceptBuilder);
impl_builder_single_period!(StreamingLinRegSlope, LinRegSlopeBuilder);
impl_builder_single_period!(StreamingStdDev, StdDevBuilder);
impl_builder_single_period!(StreamingTsf, TsfBuilder);
impl_builder_single_period!(StreamingVar, VarBuilder);
impl_builder_single_period!(StreamingMcGinley, McGinleyBuilder);
impl_builder_single_period!(StreamingMfi, MfiBuilder);
impl_builder_single_period!(StreamingMinusDi, MinusDiBuilder);
impl_builder_single_period!(StreamingMom, MomBuilder);
impl_builder_single_period!(StreamingNatr, NatrBuilder);
impl_builder_single_period!(StreamingPlusDi, PlusDiBuilder);
impl_builder_single_period!(StreamingPsy, PsyBuilder);
impl_builder_single_period!(StreamingRoc, RocBuilder);
impl_builder_single_period!(StreamingRsi, RsiBuilder);
impl_builder_single_period!(StreamingRvi, RviBuilder);
// SmaBuilder has a manual implementation to support price_source()
// impl_builder_single_period!(StreamingSma, SmaBuilder); -- replaced below
impl_builder_single_period!(StreamingT3, T3Builder);
impl_builder_single_period!(StreamingTema, TemaBuilder);
impl_builder_single_period!(StreamingTrix, TrixBuilder);
impl_builder_single_period!(StreamingUlcerIndex, UlcerIndexBuilder);
impl_builder_single_period!(StreamingVr, VrBuilder);
impl_builder_single_period!(StreamingVwma, VwmaBuilder);
impl_builder_single_period!(StreamingWillR, WillRBuilder);
impl_builder_single_period!(StreamingWma, WmaBuilder);
impl_builder_single_period!(StreamingZlema, ZlemaBuilder);
impl_builder_single_period!(StreamingZscore, ZscoreBuilder);

// ---------------------------------------------------------------------------
// 2 params — two usize periods (6 indicators)
// ---------------------------------------------------------------------------

use super::indicators::StreamingAd;
use super::indicators::StreamingAdosc;
use super::indicators::StreamingAnchoredVwap;
use super::indicators::StreamingHtDcPeriod;
use super::indicators::StreamingHtDcPhase;
use super::indicators::StreamingHtSine;
use super::indicators::StreamingHtTrendline;
use super::indicators::StreamingHtTrendMode;
use super::indicators::StreamingAo;
use super::indicators::StreamingApo;
use super::indicators::StreamingBeta;
use super::indicators::StreamingCorrel;
use super::indicators::StreamingElderRay;
use super::indicators::StreamingLinRegAngle;
use super::indicators::StreamingLinRegIntercept;
use super::indicators::StreamingLinRegSlope;
use super::indicators::StreamingStdDev;
use super::indicators::StreamingTsf;
use super::indicators::StreamingVar;
use super::indicators::StreamingExpma;
use super::indicators::StreamingPpo;
use super::indicators::StreamingStochF;
use super::indicators::StreamingStochRsi;
use super::indicators::StreamingUltOsc;
use super::indicators::StreamingVwapBands;
use super::indicators::StreamingMassIndex;
use super::indicators::StreamingTsi;
use super::indicators::StreamingVidya;

impl_builder_two_periods!(StreamingAdosc, AdoscBuilder, fast_period, slow_period);
impl_builder_two_periods!(StreamingAo, AoBuilder, fast_period, slow_period);
impl_builder_two_periods!(StreamingApo, ApoBuilder, fast_period, slow_period);
impl_builder_single_period!(StreamingElderRay, ElderRayBuilder);
impl_builder_two_periods!(StreamingExpma, ExpmaBuilder, short_period, long_period);
impl_builder_two_periods!(StreamingPpo, PpoBuilder, fast_period, slow_period);
impl_builder_two_periods!(StreamingMassIndex, MassIndexBuilder, period, ema_period);
impl_builder_two_periods!(StreamingTsi, TsiBuilder, long_period, short_period);
impl_builder_two_periods!(StreamingVidya, VidyaBuilder, period, cmo_period);

// ---------------------------------------------------------------------------
// 2 params — special: SAR (acceleration: f64, maximum: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingSar;

#[derive(Default)]
pub struct SarBuilder {
    acceleration: Option<f64>,
    maximum: Option<f64>,
}

impl SarBuilder {
    pub fn acceleration(mut self, v: f64) -> Self {
        self.acceleration = Some(v);
        self
    }
    pub fn maximum(mut self, v: f64) -> Self {
        self.maximum = Some(v);
        self
    }
}

impl Builder for SarBuilder {
    type Output = StreamingSar;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let acceleration = self.acceleration.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "acceleration".into(),
            reason: "must be set".into(),
        })?;
        let maximum = self.maximum.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "maximum".into(),
            reason: "must be set".into(),
        })?;
        if acceleration <= 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "acceleration".into(),
                reason: "> 0.0".into(),
            });
        }
        if maximum <= 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "maximum".into(),
                reason: "> 0.0".into(),
            });
        }
        Ok(StreamingSar::new(acceleration, maximum))
    }
}

impl IndicatorBuilder for StreamingSar {
    type Builder = SarBuilder;
    fn builder() -> Self::Builder {
        SarBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 2 params — special: SuperTrend (period: usize, multiplier: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingSuperTrend;

#[derive(Default)]
pub struct SuperTrendBuilder {
    period: Option<usize>,
    multiplier: Option<f64>,
}

impl SuperTrendBuilder {
    pub fn period(mut self, v: usize) -> Self {
        self.period = Some(v);
        self
    }
    pub fn multiplier(mut self, v: f64) -> Self {
        self.multiplier = Some(v);
        self
    }
}

impl Builder for SuperTrendBuilder {
    type Output = StreamingSuperTrend;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        let multiplier = self.multiplier.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "multiplier".into(),
            reason: "must be set".into(),
        })?;
        if period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: "> 0".into(),
            });
        }
        if multiplier <= 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "multiplier".into(),
                reason: "> 0.0".into(),
            });
        }
        Ok(StreamingSuperTrend::new(period, multiplier))
    }
}

impl IndicatorBuilder for StreamingSuperTrend {
    type Builder = SuperTrendBuilder;
    fn builder() -> Self::Builder {
        SuperTrendBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 3 params — three usize (6 indicators)
// ---------------------------------------------------------------------------

use super::indicators::StreamingCoppock;
use super::indicators::StreamingDma;
use super::indicators::StreamingIchimoku;
use super::indicators::StreamingKdj;
use super::indicators::StreamingKvo;
use super::indicators::StreamingMacd;
use super::indicators::StreamingStc;
use super::indicators::StreamingStoch;

impl_builder_three_periods!(StreamingCoppock, CoppockBuilder, wma_period, long_roc, short_roc);
impl_builder_three_periods!(StreamingDma, DmaBuilder, short_period, long_period, ama_period);
impl_builder_three_periods!(StreamingIchimoku, IchimokuBuilder, tenkan, kijun, senkou_b);
impl_builder_three_periods!(StreamingKdj, KdjBuilder, n, m1, m2);
impl_builder_three_periods!(StreamingKvo, KvoBuilder, fast_period, slow_period, signal_period);
impl_builder_three_periods!(StreamingMacd, MacdBuilder, fast_period, slow_period, signal_period);
impl_builder_three_periods!(StreamingStc, StcBuilder, fast_period, slow_period, cycle);
impl_builder_three_periods!(StreamingStoch, StochBuilder, k_period, k_slow, d_period);

// ---------------------------------------------------------------------------
// 3 params — special: ALMA (period: usize, sigma: f64, offset: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingAlma;

#[derive(Default)]
pub struct AlmaBuilder {
    period: Option<usize>,
    sigma: Option<f64>,
    offset: Option<f64>,
}

impl AlmaBuilder {
    pub fn period(mut self, v: usize) -> Self {
        self.period = Some(v);
        self
    }
    pub fn sigma(mut self, v: f64) -> Self {
        self.sigma = Some(v);
        self
    }
    pub fn offset(mut self, v: f64) -> Self {
        self.offset = Some(v);
        self
    }
}

impl Builder for AlmaBuilder {
    type Output = StreamingAlma;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        let sigma = self.sigma.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "sigma".into(),
            reason: "must be set".into(),
        })?;
        let offset = self.offset.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "offset".into(),
            reason: "must be set".into(),
        })?;
        if period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: "> 0".into(),
            });
        }
        if sigma <= 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "sigma".into(),
                reason: "> 0.0".into(),
            });
        }
        if !(0.0..=1.0).contains(&offset) {
            return Err(IndicatorError::InvalidParameter {
                param: "offset".into(),
                reason: "in range [0.0, 1.0]".into(),
            });
        }
        Ok(StreamingAlma::new(period, sigma, offset))
    }
}

impl IndicatorBuilder for StreamingAlma {
    type Builder = AlmaBuilder;
    fn builder() -> Self::Builder {
        AlmaBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 3 params — special: BOLL (period: usize, nb_dev_up: f64, nb_dev_dn: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingBoll;

#[derive(Default)]
pub struct BollBuilder {
    period: Option<usize>,
    nb_dev_up: Option<f64>,
    nb_dev_dn: Option<f64>,
}

impl BollBuilder {
    pub fn period(mut self, v: usize) -> Self {
        self.period = Some(v);
        self
    }
    pub fn nb_dev_up(mut self, v: f64) -> Self {
        self.nb_dev_up = Some(v);
        self
    }
    pub fn nb_dev_dn(mut self, v: f64) -> Self {
        self.nb_dev_dn = Some(v);
        self
    }
}

impl Builder for BollBuilder {
    type Output = StreamingBoll;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        let nb_dev_up = self.nb_dev_up.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "nb_dev_up".into(),
            reason: "must be set".into(),
        })?;
        let nb_dev_dn = self.nb_dev_dn.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "nb_dev_dn".into(),
            reason: "must be set".into(),
        })?;
        if period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: "> 0".into(),
            });
        }
        if nb_dev_up < 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "nb_dev_up".into(),
                reason: ">= 0.0".into(),
            });
        }
        if nb_dev_dn < 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "nb_dev_dn".into(),
                reason: ">= 0.0".into(),
            });
        }
        Ok(StreamingBoll::new(period, nb_dev_up, nb_dev_dn))
    }
}

impl IndicatorBuilder for StreamingBoll {
    type Builder = BollBuilder;
    fn builder() -> Self::Builder {
        BollBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 3 params — special: ENE (period: usize, k1: f64, k2: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingEne;

#[derive(Default)]
pub struct EneBuilder {
    period: Option<usize>,
    k1: Option<f64>,
    k2: Option<f64>,
}

impl EneBuilder {
    pub fn period(mut self, v: usize) -> Self {
        self.period = Some(v);
        self
    }
    pub fn k1(mut self, v: f64) -> Self {
        self.k1 = Some(v);
        self
    }
    pub fn k2(mut self, v: f64) -> Self {
        self.k2 = Some(v);
        self
    }
}

impl Builder for EneBuilder {
    type Output = StreamingEne;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        let k1 = self.k1.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "k1".into(),
            reason: "must be set".into(),
        })?;
        let k2 = self.k2.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "k2".into(),
            reason: "must be set".into(),
        })?;
        if period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: "> 0".into(),
            });
        }
        Ok(StreamingEne::new(period, k1, k2))
    }
}

impl IndicatorBuilder for StreamingEne {
    type Builder = EneBuilder;
    fn builder() -> Self::Builder {
        EneBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 3 params — special: Keltner (ema_period: usize, atr_period: usize, multiplier: f64)
// ---------------------------------------------------------------------------

use super::indicators::StreamingKeltner;

#[derive(Default)]
pub struct KeltnerBuilder {
    ema_period: Option<usize>,
    atr_period: Option<usize>,
    multiplier: Option<f64>,
}

impl KeltnerBuilder {
    pub fn ema_period(mut self, v: usize) -> Self {
        self.ema_period = Some(v);
        self
    }
    pub fn atr_period(mut self, v: usize) -> Self {
        self.atr_period = Some(v);
        self
    }
    pub fn multiplier(mut self, v: f64) -> Self {
        self.multiplier = Some(v);
        self
    }
}

impl Builder for KeltnerBuilder {
    type Output = StreamingKeltner;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let ema_period = self.ema_period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "ema_period".into(),
            reason: "must be set".into(),
        })?;
        let atr_period = self.atr_period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "atr_period".into(),
            reason: "must be set".into(),
        })?;
        let multiplier = self.multiplier.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "multiplier".into(),
            reason: "must be set".into(),
        })?;
        if ema_period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "ema_period".into(),
                reason: "> 0".into(),
            });
        }
        if atr_period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "atr_period".into(),
                reason: "> 0".into(),
            });
        }
        if multiplier <= 0.0 {
            return Err(IndicatorError::InvalidParameter {
                param: "multiplier".into(),
                reason: "> 0.0".into(),
            });
        }
        Ok(StreamingKeltner::new(ema_period, atr_period, multiplier))
    }
}

impl IndicatorBuilder for StreamingKeltner {
    type Builder = KeltnerBuilder;
    fn builder() -> Self::Builder {
        KeltnerBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// 9 params — KST
// ---------------------------------------------------------------------------

use super::indicators::StreamingKst;

#[derive(Default)]
pub struct KstBuilder {
    roc1: Option<usize>,
    roc2: Option<usize>,
    roc3: Option<usize>,
    roc4: Option<usize>,
    sma1: Option<usize>,
    sma2: Option<usize>,
    sma3: Option<usize>,
    sma4: Option<usize>,
    signal_period: Option<usize>,
}

impl KstBuilder {
    pub fn roc1(mut self, v: usize) -> Self { self.roc1 = Some(v); self }
    pub fn roc2(mut self, v: usize) -> Self { self.roc2 = Some(v); self }
    pub fn roc3(mut self, v: usize) -> Self { self.roc3 = Some(v); self }
    pub fn roc4(mut self, v: usize) -> Self { self.roc4 = Some(v); self }
    pub fn sma1(mut self, v: usize) -> Self { self.sma1 = Some(v); self }
    pub fn sma2(mut self, v: usize) -> Self { self.sma2 = Some(v); self }
    pub fn sma3(mut self, v: usize) -> Self { self.sma3 = Some(v); self }
    pub fn sma4(mut self, v: usize) -> Self { self.sma4 = Some(v); self }
    pub fn signal_period(mut self, v: usize) -> Self { self.signal_period = Some(v); self }
}

impl Builder for KstBuilder {
    type Output = StreamingKst;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        macro_rules! require_usize {
            ($field:ident) => {
                self.$field.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($field).into(),
                    reason: "must be set".into(),
                }).and_then(|v| {
                    if v == 0 {
                        Err(IndicatorError::InvalidParameter {
                            param: stringify!($field).into(),
                            reason: "> 0".into(),
                        })
                    } else {
                        Ok(v)
                    }
                })?
            };
        }
        let roc1 = require_usize!(roc1);
        let roc2 = require_usize!(roc2);
        let roc3 = require_usize!(roc3);
        let roc4 = require_usize!(roc4);
        let sma1 = require_usize!(sma1);
        let sma2 = require_usize!(sma2);
        let sma3 = require_usize!(sma3);
        let sma4 = require_usize!(sma4);
        let signal_period = require_usize!(signal_period);
        Ok(StreamingKst::new(roc1, roc2, roc3, roc4, sma1, sma2, sma3, sma4, signal_period))
    }
}

impl IndicatorBuilder for StreamingKst {
    type Builder = KstBuilder;
    fn builder() -> Self::Builder {
        KstBuilder::default()
    }
}

// ---------------------------------------------------------------------------
// STOCHF (fastk_period, fastd_period)
// ---------------------------------------------------------------------------

impl_builder_two_periods!(StreamingStochF, StochFBuilder, fastk_period, fastd_period);

// ---------------------------------------------------------------------------
// ULTOSC (period1, period2, period3)
// ---------------------------------------------------------------------------

impl_builder_three_periods!(StreamingUltOsc, UltOscBuilder, period1, period2, period3);

// ---------------------------------------------------------------------------
// STOCHRSI (rsi_period, stoch_period, fastk_period, fastd_period)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct StochRsiBuilder {
    rsi_period: Option<usize>,
    stoch_period: Option<usize>,
    fastk_period: Option<usize>,
    fastd_period: Option<usize>,
}

impl StochRsiBuilder {
    pub fn rsi_period(mut self, v: usize) -> Self { self.rsi_period = Some(v); self }
    pub fn stoch_period(mut self, v: usize) -> Self { self.stoch_period = Some(v); self }
    pub fn fastk_period(mut self, v: usize) -> Self { self.fastk_period = Some(v); self }
    pub fn fastd_period(mut self, v: usize) -> Self { self.fastd_period = Some(v); self }
}

impl Builder for StochRsiBuilder {
    type Output = StreamingStochRsi;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        macro_rules! require {
            ($field:ident) => {
                self.$field.ok_or_else(|| IndicatorError::InvalidParameter {
                    param: stringify!($field).into(),
                    reason: "must be set".into(),
                }).and_then(|v| {
                    if v == 0 {
                        Err(IndicatorError::InvalidParameter {
                            param: stringify!($field).into(),
                            reason: "> 0".into(),
                        })
                    } else { Ok(v) }
                })?
            };
        }
        let rsi_period = require!(rsi_period);
        let stoch_period = require!(stoch_period);
        let fastk_period = require!(fastk_period);
        let fastd_period = require!(fastd_period);
        Ok(StreamingStochRsi::new(rsi_period, stoch_period, fastk_period, fastd_period))
    }
}

impl IndicatorBuilder for StreamingStochRsi {
    type Builder = StochRsiBuilder;
    fn builder() -> Self::Builder { StochRsiBuilder::default() }
}

// ---------------------------------------------------------------------------
// VWAP Bands (period + nb_dev)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct VwapBandsBuilder {
    period: Option<usize>,
    nb_dev: Option<f64>,
}

impl VwapBandsBuilder {
    pub fn period(mut self, v: usize) -> Self { self.period = Some(v); self }
    pub fn nb_dev(mut self, v: f64) -> Self { self.nb_dev = Some(v); self }
}

impl Builder for VwapBandsBuilder {
    type Output = StreamingVwapBands;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        if period < 2 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: ">= 2".into(),
            });
        }
        let nb_dev = self.nb_dev.unwrap_or(2.0);
        Ok(StreamingVwapBands::new(period, nb_dev))
    }
}

impl IndicatorBuilder for StreamingVwapBands {
    type Builder = VwapBandsBuilder;
    fn builder() -> Self::Builder { VwapBandsBuilder::default() }
}

// ---------------------------------------------------------------------------
// SMA — manual builder with price_source support
// ---------------------------------------------------------------------------

use super::price_source::PriceSource;

#[derive(Default)]
pub struct SmaBuilder {
    period: Option<usize>,
    price_source: Option<PriceSource>,
}

impl SmaBuilder {
    pub fn period(mut self, v: usize) -> Self {
        self.period = Some(v);
        self
    }
    pub fn price_source(mut self, v: PriceSource) -> Self {
        self.price_source = Some(v);
        self
    }
}

impl Builder for SmaBuilder {
    type Output = StreamingSma;

    fn build(self) -> Result<Self::Output, IndicatorError> {
        let period = self.period.ok_or_else(|| IndicatorError::InvalidParameter {
            param: "period".into(),
            reason: "must be set".into(),
        })?;
        if period == 0 {
            return Err(IndicatorError::InvalidParameter {
                param: "period".into(),
                reason: "> 0".into(),
            });
        }
        let price_source = self.price_source.unwrap_or_default();
        Ok(StreamingSma::with_price_source(period, price_source))
    }
}

impl IndicatorBuilder for StreamingSma {
    type Builder = SmaBuilder;
    fn builder() -> Self::Builder {
        SmaBuilder::default()
    }
}
