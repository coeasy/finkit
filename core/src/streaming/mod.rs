//! Streaming (incremental) indicator framework.
//!
//! This module provides O(1) per-bar indicator updates via the
//! [`StreamingIndicator`] trait, an [`Ohlcv`] abstraction for bar data,
//! and [`IndicatorMeta`] for machine-readable indicator metadata.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────────────┐     ┌───────────────┐
//! │  Data Feed   │────▶│  StreamingIndicator   │────▶│    Output     │
//! │  (Ohlcv bar) │     │  .next(bar) -> O(1)   │     │   (f64/struct)│
//! └─────────────┘     └──────────────────────┘     └───────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```
//! use finkit::streaming::{StreamingIndicator, OhlcvBar, Ohlcv};
//! use finkit::streaming::overlap::sma::StreamingSma;
//!
//! let mut sma = StreamingSma::new(3);
//! assert_eq!(sma.next(11.0), None);  // warming up
//! assert_eq!(sma.next(12.0), None);
//! let val = sma.next(13.0);          // ready
//! assert_eq!(val, Some(12.0));
//! assert_eq!(sma.value(), Some(12.0)); // cached last value
//! ```

// ---------------------------------------------------------------------------
// Sub-category modules
// ---------------------------------------------------------------------------
pub mod breadth;
pub mod cycle;
pub mod math;
pub mod momentum;
pub mod overlap;
pub mod pattern;
pub mod price_transform;
pub mod statistics;
pub mod trend;
pub mod volatility;
pub mod volume;

// ---------------------------------------------------------------------------
// Infrastructure modules
// ---------------------------------------------------------------------------
pub mod builder;
pub mod checkpoint;
pub mod float_trait;
pub mod macros;
pub mod output_types;
pub mod price_source;
pub mod registry;
pub mod repaint;
pub mod ring_buffer;
pub mod rolling_minmax;
pub mod traits;
mod types;

// ---------------------------------------------------------------------------
// Convenience re-export hub: streaming::indicators::StreamingSma, etc.
// ---------------------------------------------------------------------------
pub mod indicators {
    pub use super::overlap::alma::StreamingAlma;
    pub use super::overlap::anchored_vwap::StreamingAnchoredVwap;
    pub use super::overlap::dema::StreamingDema;
    pub use super::overlap::dma::{DmaOutput, StreamingDma};
    pub use super::overlap::efficiency_ratio::StreamingEfficiencyRatio;
    pub use super::overlap::ema::StreamingEma;
    pub use super::overlap::expma::{ExpmaOutput, StreamingExpma};
    pub use super::overlap::hma::StreamingHma;
    pub use super::overlap::ichimoku::{IchimokuOutput, StreamingIchimoku};
    pub use super::overlap::jma::StreamingJma;
    pub use super::overlap::kama::StreamingKama;
    pub use super::overlap::mama::StreamingMama;
    pub use super::overlap::mcginley::StreamingMcGinley;
    pub use super::overlap::midpoint::StreamingMidpoint;
    pub use super::overlap::sma::StreamingSma;
    pub use super::overlap::t3::StreamingT3;
    pub use super::overlap::tema::StreamingTema;
    pub use super::overlap::trima::StreamingTrima;
    pub use super::overlap::vidya::StreamingVidya;
    pub use super::overlap::vwap::StreamingVwap;
    pub use super::overlap::vwap_bands::{StreamingVwapBands, VwapBandsOutput};
    pub use super::overlap::vwap_mtf::{StreamingVwapMtf, VwapMtfInput};
    pub use super::overlap::vwma::StreamingVwma;
    pub use super::overlap::wma::StreamingWma;
    pub use super::overlap::zlema::StreamingZlema;

    pub use super::momentum::ao::StreamingAo;
    pub use super::momentum::apo::StreamingApo;
    pub use super::momentum::aroon::{AroonOutput, StreamingAroon};
    pub use super::momentum::aroon_osc::StreamingAroonOsc;
    pub use super::momentum::bias::StreamingBias;
    pub use super::momentum::cci::StreamingCci;
    pub use super::momentum::cfo::StreamingCfo;
    pub use super::momentum::cmo::StreamingCmo;
    pub use super::momentum::coppock::StreamingCoppock;
    pub use super::momentum::dpo::StreamingDpo;
    pub use super::momentum::elder_ray::{ElderRayOutput, StreamingElderRay};
    pub use super::momentum::fisher_streaming::{FisherOutput, StreamingFisher};
    pub use super::momentum::kdj::{KdjOutput, StreamingKdj};
    pub use super::momentum::kst::{KstOutput, StreamingKst};
    pub use super::momentum::macd::{MacdOutput, StreamingMacd};
    pub use super::momentum::macd_ext::StreamingMacdExt;
    pub use super::momentum::macd_fix::StreamingMacdFix;
    pub use super::momentum::mom::StreamingMom;
    pub use super::momentum::ppo::StreamingPpo;
    pub use super::momentum::psy::StreamingPsy;
    pub use super::momentum::roc::{StreamingRoc, StreamingRocp, StreamingRocr, StreamingRocr100};
    pub use super::momentum::rsi::StreamingRsi;
    pub use super::momentum::rvi_streaming::{RviOutput, StreamingRvi};
    pub use super::momentum::stc::StreamingStc;
    pub use super::momentum::stoch::{StochOutput, StreamingStoch};
    pub use super::momentum::stoch_f::StreamingStochF;
    pub use super::momentum::stoch_rsi::StreamingStochRsi;
    pub use super::momentum::trix::StreamingTrix;
    pub use super::momentum::tsf::StreamingTsf;
    pub use super::momentum::tsi_streaming::StreamingTsi;
    pub use super::momentum::ult_osc::StreamingUltOsc;
    pub use super::momentum::willr::StreamingWillR;

    pub use super::trend::adx::StreamingAdx;
    pub use super::trend::adxr::StreamingAdxr;
    pub use super::trend::dx::StreamingDx;
    pub use super::trend::ht_measurement::StreamingHtMeasurement;
    pub use super::trend::ht_trendline::StreamingHtTrendline;
    pub use super::trend::ht_trendmode::StreamingHtTrendMode;
    pub use super::trend::inertia::StreamingInertia;
    pub use super::trend::minus_di::StreamingMinusDi;
    pub use super::trend::minus_dm::StreamingMinusDm;
    pub use super::trend::plus_di::StreamingPlusDi;
    pub use super::trend::plus_dm::StreamingPlusDm;
    pub use super::trend::sar::{SarOutput, StreamingSar};
    pub use super::trend::supertrend::{StreamingSuperTrend, SuperTrendOutput};
    pub use super::trend::vortex::{StreamingVortex, VortexOutput};

    pub use super::volatility::adr::StreamingAdr;
    pub use super::volatility::atr::StreamingAtr;
    pub use super::volatility::boll::{BollOutput, StreamingBoll};
    pub use super::volatility::chaikin_vol::StreamingChaikinVol;
    pub use super::volatility::chop_streaming::StreamingChop;
    pub use super::volatility::donchian::{DonchianOutput, StreamingDonchian};
    pub use super::volatility::ene::{EneOutput, StreamingEne};
    pub use super::volatility::hv::StreamingHv;
    pub use super::volatility::keltner::{KeltnerOutput, StreamingKeltner};
    pub use super::volatility::natr::StreamingNatr;
    pub use super::volatility::stddev::StreamingStdDev;
    pub use super::volatility::trange::StreamingTrange;
    pub use super::volatility::ulcer_index::StreamingUlcerIndex;
    pub use super::volatility::var::StreamingVar;

    pub use super::volume::ad::StreamingAd;
    pub use super::volume::adosc::StreamingAdosc;
    pub use super::volume::cmf::StreamingCmf;
    pub use super::volume::eom::StreamingEom;
    pub use super::volume::force_index::StreamingForceIndex;
    pub use super::volume::kvo::{KvoOutput, StreamingKvo};
    pub use super::volume::mfi::StreamingMfi;
    pub use super::volume::nvi::StreamingNvi;
    pub use super::volume::obv::StreamingObv;
    pub use super::volume::pvi::StreamingPvi;
    pub use super::volume::pvt::StreamingPvt;
    pub use super::volume::twiggs_mf::StreamingTwiggsMf;
    pub use super::volume::volume_momentum::{StreamingVolumeMomentum, StreamingVolumeRoc};
    pub use super::volume::volume_oscillator::StreamingVolumeOscillator;
    pub use super::volume::vr::StreamingVr;
    pub use super::volume::vzo::StreamingVzo;

    pub use super::cycle::ehlers::{
        StreamingBandpass, StreamingDecycler, StreamingInstantaneousTrendline,
        StreamingRoofingFilter, StreamingSuperSmoother, StreamingSuperSmoother3Pole,
    };
    pub use super::cycle::ht_dcperiod::StreamingHtDcPeriod;
    pub use super::cycle::ht_dcphase::StreamingHtDcPhase;
    pub use super::cycle::ht_phasor::StreamingHtPhasor;
    pub use super::cycle::ht_sine::{HtSineOutput, StreamingHtSine};
    pub use super::cycle::mass_index::StreamingMassIndex;
    pub use super::cycle::mcclellan::StreamingMcClellanOscillator;

    pub use super::price_transform::avgprice::StreamingAvgPrice;
    pub use super::price_transform::bop::StreamingBop;
    pub use super::price_transform::medprice::StreamingMedPrice;
    pub use super::price_transform::midprice::StreamingMidprice;
    pub use super::price_transform::qstick::StreamingQStick;
    pub use super::price_transform::typprice::StreamingTypPrice;
    pub use super::price_transform::wclprice::StreamingWclPrice;

    pub use super::statistics::avgdev::StreamingAvgdev;
    pub use super::statistics::beta::StreamingBeta;
    pub use super::statistics::correl::StreamingCorrel;
    pub use super::statistics::linreg::StreamingLinReg;
    pub use super::statistics::linreg_angle::StreamingLinRegAngle;
    pub use super::statistics::linreg_intercept::StreamingLinRegIntercept;
    pub use super::statistics::linreg_slope::StreamingLinRegSlope;
    pub use super::statistics::max_min_index::{StreamingMaxIndex, StreamingMinIndex};
    pub use super::statistics::percent_rank::StreamingPercentRank;
    pub use super::statistics::zscore::StreamingZscore;

    pub use super::pattern::patterns::{
        StreamingCdl3BlackCrows, StreamingCdl3WhiteSoldiers, StreamingCdlAbandonedBaby,
        StreamingCdlDarkCloudCover, StreamingCdlDoji, StreamingCdlDojiStar, StreamingCdlEngulfing,
        StreamingCdlEveningStar, StreamingCdlHammer, StreamingCdlHangingMan, StreamingCdlHarami,
        StreamingCdlInvertedHammer, StreamingCdlKicking, StreamingCdlMarubozu,
        StreamingCdlMorningStar, StreamingCdlPiercing, StreamingCdlShootingStar,
        StreamingCdlSpinningTop, StreamingCdlTasukiGap, StreamingCdlTristar,
    };
    pub use super::pattern::smc::{StreamingFairValueGap, StreamingOrderBlock};
    pub use super::pattern::squeeze_momentum::{SqueezeMomentumOutput, StreamingSqueezeMomentum};

    pub use super::breadth::advance_decline::StreamingAdvanceDeclineLine;
    pub use super::breadth::ar::StreamingAr;
    pub use super::breadth::br::StreamingBr;
    pub use super::breadth::cr::StreamingCr;
    pub use super::breadth::fear_greed::StreamingFearGreedIndex;
    pub use super::breadth::put_call_ratio::StreamingPutCallRatio;
    pub use super::breadth::trin::StreamingTrin;

    pub use super::math::math_operators::{
        StreamingAdd, StreamingDiv, StreamingMax, StreamingMin, StreamingMinus, StreamingMult,
        StreamingSub, StreamingSum,
    };
    pub use super::math::math_transform::{
        StreamingAcos, StreamingAsin, StreamingAtan, StreamingCeil, StreamingCos, StreamingCosh,
        StreamingExp, StreamingFloor, StreamingLn, StreamingLog10, StreamingSin, StreamingSinh,
        StreamingSqrt, StreamingTan, StreamingTanh,
    };
}

// ---------------------------------------------------------------------------
// Top-level re-exports
// ---------------------------------------------------------------------------
pub use builder::{Builder, IndicatorBuilder};
#[cfg(feature = "serde")]
pub use checkpoint::{CheckpointError, CheckpointState};
pub use price_source::PriceSource;
pub use registry::{all_indicators, IndicatorInfo, ParamInfo, RegistryDocument};
pub use traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
pub use types::OhlcvBar;
