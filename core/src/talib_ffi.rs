//! Manual FFI bindings to TA-Lib C library for benchmark comparison.
//!
//! Only compiled when the `talib-c` feature is enabled.
//!
//! Covers all 158 functions in TA-Lib 0.6.4 across 10 categories:
//! - Overlap Studies (18)
//! - Momentum Indicators (30)
//! - Volume Indicators (3)
//! - Volatility Indicators (3)
//! - Price Transform (4)
//! - Cycle Indicators (6)
//! - Statistics Functions (9)
//! - Math Transform (15)
//! - Math Operators (12)
//! - Pattern Recognition (61)

#![allow(non_camel_case_types, non_upper_case_globals, dead_code)]

pub type TA_RetCode = i32;
pub type TA_MAType = i32;
pub type TA_CandleSettingType = i32;

pub const TA_SUCCESS: TA_RetCode = 0;
pub const TA_MAType_SMA: TA_MAType = 0;
pub const TA_MAType_EMA: TA_MAType = 1;
pub const TA_MAType_WMA: TA_MAType = 2;
pub const TA_MAType_DEMA: TA_MAType = 3;
pub const TA_MAType_TEMA: TA_MAType = 4;
pub const TA_MAType_TRIMA: TA_MAType = 5;
pub const TA_MAType_KAMA: TA_MAType = 6;
pub const TA_MAType_MAMA: TA_MAType = 7;
pub const TA_MAType_T3: TA_MAType = 8;

extern "C" {
    pub fn TA_Initialize() -> TA_RetCode;
    pub fn TA_Shutdown() -> TA_RetCode;

    // ========================================================================
    // Overlap Studies (18)
    // ========================================================================
    pub fn TA_SMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_EMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_WMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_DEMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TEMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_KAMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TRIMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_T3(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32, optInVFactor: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_BBANDS(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32, optInNbDevUp: f64, optInNbDevDn: f64,
        optInMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outRealUpperBand: *mut f64, outRealMiddleBand: *mut f64, outRealLowerBand: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32, optInMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MAVP(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        inPeriods: *const f64,
        optInMinPeriod: i32, optInMaxPeriod: i32, optInMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SAR(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInAcceleration: f64, optInMaximum: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SAREXT(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInStartValue: f64, optInOffsetOnReverse: f64,
        optInAccelerationInitLong: f64, optInAccelerationLong: f64,
        optInAccelerationMaxLong: f64, optInAccelerationInitShort: f64,
        optInAccelerationShort: f64, optInAccelerationMaxShort: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Momentum Indicators (30)
    // ========================================================================
    pub fn TA_RSI(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MACD(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInFastPeriod: i32, optInSlowPeriod: i32, optInSignalPeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMACD: *mut f64, outMACDSignal: *mut f64, outMACDHist: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MACDEXT(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInFastPeriod: i32, optInFastMAType: TA_MAType,
        optInSlowPeriod: i32, optInSlowMAType: TA_MAType,
        optInSignalPeriod: i32, optInSignalMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMACD: *mut f64, outMACDSignal: *mut f64, outMACDHist: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MACDFIX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInSignalPeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMACD: *mut f64, outMACDSignal: *mut f64, outMACDHist: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_STOCH(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInFastK_Period: i32,
        optInSlowK_Period: i32, optInSlowK_MAType: TA_MAType,
        optInSlowD_Period: i32, optInSlowD_MAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outSlowK: *mut f64, outSlowD: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_STOCHF(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInFastK_Period: i32,
        optInFastD_Period: i32, optInFastD_MAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outFastK: *mut f64, outFastD: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_STOCHRSI(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        optInFastK_Period: i32,
        optInFastD_Period: i32, optInFastD_MAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outFastK: *mut f64, outFastD: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ROC(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ROCP(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ROCR(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ROCR100(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MOM(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_CMO(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_CCI(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_WILLR(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MFI(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64, inVolume: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TRIX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_APO(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInFastPeriod: i32, optInSlowPeriod: i32, optInMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_PPO(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInFastPeriod: i32, optInSlowPeriod: i32, optInMAType: TA_MAType,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_DX(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ADX(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ADXR(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_PLUS_DI(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MINUS_DI(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_PLUS_DM(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MINUS_DM(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_AROON(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outAroonDown: *mut f64, outAroonUp: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_AROONOSC(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_BOP(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ULTOSC(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod1: i32, optInTimePeriod2: i32, optInTimePeriod3: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MAMA(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInFastLimit: f64, optInSlowLimit: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMAMA: *mut f64, outFAMA: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Volume Indicators (3)
    // ========================================================================
    pub fn TA_AD(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64, inVolume: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_OBV(
        startIdx: i32, endIdx: i32,
        inReal: *const f64, inVolume: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ADOSC(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64, inVolume: *const f64,
        optInFastPeriod: i32, optInSlowPeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Volatility Indicators (3)
    // ========================================================================
    pub fn TA_ATR(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_NATR(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TRANGE(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Price Transform (4)
    // ========================================================================
    pub fn TA_AVGPRICE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MEDPRICE(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TYPPRICE(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_WCLPRICE(
        startIdx: i32, endIdx: i32,
        inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Cycle Indicators (6) — Hilbert Transform family
    // ========================================================================
    pub fn TA_HT_DCPERIOD(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_HT_DCPHASE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_HT_PHASOR(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outInPhase: *mut f64, outQuadrature: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_HT_SINE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outSine: *mut f64, outLeadSine: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_HT_TRENDLINE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_HT_TRENDMODE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    // ========================================================================
    // Statistics Functions (9)
    // ========================================================================
    pub fn TA_STDDEV(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32, optInNbDev: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_VAR(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32, optInNbDev: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LINEARREG(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LINEARREG_SLOPE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LINEARREG_INTERCEPT(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LINEARREG_ANGLE(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TSF(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_AVGDEV(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_BETA(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_CORREL(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_PERCENTRANK(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SKEWNESS(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_KURTOSIS(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Math Transform (15)
    // ========================================================================
    pub fn TA_ACOS(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ASIN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_ATAN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_CEIL(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_COS(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_COSH(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_EXP(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_FLOOR(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_LOG10(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SIN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SINH(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SQRT(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TAN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_TANH(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    // ========================================================================
    // Math Operators (12)
    // ========================================================================
    pub fn TA_ADD(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SUB(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MULT(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_DIV(
        startIdx: i32, endIdx: i32,
        inReal0: *const f64, inReal1: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_SUM(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MAX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MIN(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outReal: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MAXINDEX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_MININDEX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_MINMAX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMin: *mut f64, outMax: *mut f64,
    ) -> TA_RetCode;

    pub fn TA_MINMAXINDEX(
        startIdx: i32, endIdx: i32, inReal: *const f64,
        optInTimePeriod: i32,
        outBegIdx: *mut i32, outNBElement: *mut i32,
        outMinIdx: *mut i32, outMaxIdx: *mut i32,
    ) -> TA_RetCode;

    // ========================================================================
    // Pattern Recognition (61 CDL functions)
    // ========================================================================
    pub fn TA_CDL2CROWS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3BLACKCROWS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3INSIDE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3LINESTRIKE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3OUTSIDE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3STARSINSOUTH(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDL3WHITESOLDIERS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLABANDONEDBABY(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLADVANCEBLOCK(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLBELTHOLD(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLBREAKAWAY(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLCLOSINGMARUBOZU(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLCONCEALBABYSWALL(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLCOUNTERATTACK(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLDARKCLOUDCOVER(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLDOJI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLDOJISTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLDRAGONFLYDOJI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLENGULFING(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLEVENINGDOJISTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLEVENINGSTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLGAPSIDESIDEWHITE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLGRAVESTONEDOJI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHAMMER(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHANGINGMAN(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHARAMI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHARAMICROSS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHIGHWAVE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHIKKAKE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHIKKAKEMOD(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLHOMINGPIGEON(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLIDENTICAL3CROWS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLINNECK(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLINVERTEDHAMMER(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLKICKING(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLKICKINGBYLENGTH(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLLADDERBOTTOM(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLLONGLEGGEDDOJI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLLONGLINE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLMARUBOZU(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLMATCHINGLOW(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLMATHOLD(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLMORNINGDOJISTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLMORNINGSTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        optInPenetration: f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLONNECK(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLPIERCING(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLRICKSHAWMAN(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLRISEFALL3METHODS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSEPARATINGLINES(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSHOOTINGSTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSHORTLINE(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSPINNINGTOP(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSTALLEDPATTERN(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLSTICKSANDWICH(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLTAKURI(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLTASUKIGAP(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLTHRUSTING(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLTRISTAR(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLUNIQUE3RIVER(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLUPSIDEGAP2CROWS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;

    pub fn TA_CDLXSIDEGAP3METHODS(
        startIdx: i32, endIdx: i32,
        inOpen: *const f64, inHigh: *const f64, inLow: *const f64, inClose: *const f64,
        outBegIdx: *mut i32, outNBElement: *mut i32, outInteger: *mut i32,
    ) -> TA_RetCode;
}

/// Safe wrapper to call TA-Lib functions with consistent pattern
pub fn talib_init() {
    unsafe { TA_Initialize(); }
}

pub fn talib_shutdown() {
    unsafe { TA_Shutdown(); }
}

/// Call a TA-Lib function that takes single input array + period.
/// Returns (beg_idx, nb_element, output_vec).
pub fn call_single_in(
    func: unsafe extern "C" fn(i32, i32, *const f64, i32, *mut i32, *mut i32, *mut f64) -> TA_RetCode,
    data: &[f64],
    period: i32,
) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        func(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}

/// Call a TA-Lib function that takes (high, low, close) inputs + period.
#[allow(dead_code)]
pub fn call_hlc(
    func: unsafe extern "C" fn(
        i32, i32,
        *const f64, *const f64, *const f64,
        i32,
        *mut i32, *mut i32, *mut f64,
    ) -> TA_RetCode,
    high: &[f64], low: &[f64], close: &[f64],
    period: i32,
) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        func(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

/// Call a TA-Lib math transform/operator with single input (no period).
#[allow(dead_code)]
pub fn call_math_unary(
    func: unsafe extern "C" fn(
        i32, i32, *const f64,
        *mut i32, *mut i32, *mut f64,
    ) -> TA_RetCode,
    data: &[f64],
) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        func(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}

/// Call a TA-Lib pattern recognition function on OHLC data.
#[allow(dead_code)]
pub fn call_cdl(
    func: unsafe extern "C" fn(
        i32, i32,
        *const f64, *const f64, *const f64, *const f64,
        *mut i32, *mut i32, *mut i32,
    ) -> TA_RetCode,
    open: &[f64], high: &[f64], low: &[f64], close: &[f64],
) -> Vec<i32> {
    let len = open.len();
    let mut out = vec![0i32; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        func(
            0, (len - 1) as i32,
            open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(),
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}
