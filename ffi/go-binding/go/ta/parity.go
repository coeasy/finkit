package ta

// Registry-parity additions for the Go binding.
//
// ta.go declares the indicators the Go binding shipped with. This file adds the
// ones it was missing relative to docs/ffi_registry.json, so the Go binding
// exposes the same 78-indicator surface as the C binding.
//
// Audited by: python3 scripts/audit_binding_parity.py
//
// The C declarations below are repeated (rather than shared with ta.go) because
// cgo gives every file its own translation unit: a struct typedef declared in
// ta.go's preamble is a *different* Go type from the same typedef declared here,
// so the result helpers must also be local.

/*
#cgo windows LDFLAGS: -L../target/release -lfinkit_go -lws2_32 -ladvapi32 -luserenv -lbcrypt -lncrypt -lschannel -luser32
#cgo !windows LDFLAGS: -L../target/release -lfinkit_go -lm -ldl -lpthread

#include <stdlib.h>

typedef struct {
    double *data;
    int length;
    int capacity;
    char *error;
} TaResult;

typedef struct {
    int *data;
    int length;
    int capacity;
    char *error;
} TaIntResult;

extern TaResult* ta_apo(const double *input, int length, int fast_period, int slow_period);
extern TaResult* ta_bop(const double *open, const double *high, const double *low, const double *close, int length);
extern TaResult* ta_cmo(const double *input, int length, int period);
extern TaResult* ta_mfi(const double *high, const double *low, const double *close, const double *volume, int length, int period);
extern TaResult* ta_trix(const double *input, int length, int period);
extern TaResult* ta_vortex(const double *high, const double *low, const double *close, int length, int period);
extern TaResult* ta_vzo(const double *close, const double *volume, int length, int period);
extern TaResult* ta_volume_momentum(const double *volume, int length, int period);
extern TaResult* ta_volume_roc(const double *volume, int length, int period);
extern TaResult* ta_chande_forecast(const double *close, int length, int period);
extern TaResult* ta_twiggs_mf(const double *high, const double *low, const double *close, const double *volume, int length, int period);
extern TaResult* ta_inertia(const double *open, const double *high, const double *low, const double *close, int length, int rvi_period, int linreg_period);
extern TaResult* ta_percent_rank(const double *input, int length, int period);
extern TaResult* ta_avgprice(const double *open, const double *high, const double *low, const double *close, int length);
extern TaResult* ta_medprice(const double *high, const double *low, int length);
extern TaResult* ta_typprice(const double *high, const double *low, const double *close, int length);
extern TaResult* ta_wclprice(const double *high, const double *low, const double *close, int length);
extern TaResult* ta_midpoint(const double *input, int length, int period);
extern TaResult* ta_midprice(const double *high, const double *low, int length, int period);
extern TaResult* ta_sar(const double *high, const double *low, int length, double acceleration, double maximum);
extern TaResult* ta_mama(const double *input, int length, double fast_limit, double slow_limit);

extern TaIntResult* ta_cdl_doji(const double *open, const double *high, const double *low, const double *close, int length, double pct);
extern TaIntResult* ta_cdl_dragonfly_doji(const double *open, const double *high, const double *low, const double *close, int length, double pct);
extern TaIntResult* ta_cdl_gravestone_doji(const double *open, const double *high, const double *low, const double *close, int length, double pct);
extern TaIntResult* ta_cdl_long_legged_doji(const double *open, const double *high, const double *low, const double *close, int length, double pct);
extern TaIntResult* ta_cdl_hammer(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_inverted_hammer(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_hanging_man(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_shooting_star(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_engulfing(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_harami(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_morning_star(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_evening_star(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_three_white_soldiers(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_three_black_crows(const double *open, const double *high, const double *low, const double *close, int length);
extern TaIntResult* ta_cdl_marubozu(const double *open, const double *high, const double *low, const double *close, int length, double pct);

extern void ta_free_result(TaResult *result);
extern void ta_free_int_result(TaIntResult *result);
*/
import "C"

import (
	"errors"
	"unsafe"
)

// VortexResult carries the two Vortex Indicator lines.
type VortexResult struct {
	ViPlus  []float64
	ViMinus []float64
}

// MamaResult carries the MESA Adaptive Moving Average and its following line.
type MamaResult struct {
	Mama []float64
	Fama []float64
}

func convertResultP(result *C.TaResult) ([]float64, error) {
	defer C.ta_free_result(result)

	if result.error != nil {
		return nil, errors.New(C.GoString(result.error))
	}
	if result.data == nil || result.length == 0 {
		return nil, nil
	}

	length := int(result.length)
	goSlice := unsafe.Slice((*float64)(result.data), length)

	resultCopy := make([]float64, length)
	copy(resultCopy, goSlice)
	return resultCopy, nil
}

func convertIntResult(result *C.TaIntResult) ([]int32, error) {
	defer C.ta_free_int_result(result)

	if result.error != nil {
		return nil, errors.New(C.GoString(result.error))
	}
	if result.data == nil || result.length == 0 {
		return nil, nil
	}

	length := int(result.length)
	goSlice := unsafe.Slice((*int32)(result.data), length)

	resultCopy := make([]int32, length)
	copy(resultCopy, goSlice)
	return resultCopy, nil
}

// splitPair splits one concatenated two-line result into its halves.
func splitPair(values []float64) ([]float64, []float64) {
	if len(values)%2 != 0 {
		return values, nil
	}
	half := len(values) / 2
	return values[:half], values[half:]
}

// ============ Momentum / trend ============

// Apo returns the Absolute Price Oscillator.
func Apo(input []float64, fastPeriod, slowPeriod int) ([]float64, error) {
	return convertResultP(C.ta_apo(toCSlice(input), cInt(len(input)), cInt(fastPeriod), cInt(slowPeriod)))
}

// Cmo returns the Chande Momentum Oscillator.
func Cmo(input []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_cmo(toCSlice(input), cInt(len(input)), cInt(period)))
}

// Trix returns the 1-day rate of change of a triple-smoothed EMA.
func Trix(input []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_trix(toCSlice(input), cInt(len(input)), cInt(period)))
}

// PercentRank returns the percentile rank of the current value in the window.
func PercentRank(input []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_percent_rank(toCSlice(input), cInt(len(input)), cInt(period)))
}

// ChandeForecast returns the Chande Forecast Oscillator.
func ChandeForecast(close []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_chande_forecast(toCSlice(close), cInt(len(close)), cInt(period)))
}

// Inertia returns the RVI-smoothed linear-regression trend measure.
func Inertia(open, high, low, close []float64, rviPeriod, linregPeriod int) ([]float64, error) {
	return convertResultP(C.ta_inertia(
		toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close),
		cInt(len(close)), cInt(rviPeriod), cInt(linregPeriod),
	))
}

// Mama returns the MESA Adaptive Moving Average and its following line.
func Mama(input []float64, fastLimit, slowLimit float64) (MamaResult, error) {
	values, err := convertResultP(C.ta_mama(toCSlice(input), cInt(len(input)), C.double(fastLimit), C.double(slowLimit)))
	if err != nil {
		return MamaResult{}, err
	}
	mama, fama := splitPair(values)
	return MamaResult{Mama: mama, Fama: fama}, nil
}

// Sar returns the Parabolic SAR.
func Sar(high, low []float64, acceleration, maximum float64) ([]float64, error) {
	return convertResultP(C.ta_sar(toCSlice(high), toCSlice(low), cInt(len(high)), C.double(acceleration), C.double(maximum)))
}

// ============ Price transforms ============

// AvgPrice returns (open + high + low + close) / 4.
func AvgPrice(open, high, low, close []float64) ([]float64, error) {
	return convertResultP(C.ta_avgprice(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// MedPrice returns (high + low) / 2.
func MedPrice(high, low []float64) ([]float64, error) {
	return convertResultP(C.ta_medprice(toCSlice(high), toCSlice(low), cInt(len(high))))
}

// TypPrice returns (high + low + close) / 3.
func TypPrice(high, low, close []float64) ([]float64, error) {
	return convertResultP(C.ta_typprice(toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// WclPrice returns the weighted close (high + low + 2*close) / 4.
func WclPrice(high, low, close []float64) ([]float64, error) {
	return convertResultP(C.ta_wclprice(toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// MidPoint returns the rolling midpoint of the input.
func MidPoint(input []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_midpoint(toCSlice(input), cInt(len(input)), cInt(period)))
}

// MidPrice returns the rolling midpoint of (high, low).
func MidPrice(high, low []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_midprice(toCSlice(high), toCSlice(low), cInt(len(high)), cInt(period)))
}

// ============ Volume ============

// Bop returns the Balance of Power.
func Bop(open, high, low, close []float64) ([]float64, error) {
	return convertResultP(C.ta_bop(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// Mfi returns the Money Flow Index.
func Mfi(high, low, close, volume []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_mfi(
		toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume),
		cInt(len(close)), cInt(period),
	))
}

// Vzo returns the Volume Zone Oscillator.
func Vzo(close, volume []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_vzo(toCSlice(close), toCSlice(volume), cInt(len(close)), cInt(period)))
}

// VolumeMomentum returns the volume momentum oscillator.
func VolumeMomentum(volume []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_volume_momentum(toCSlice(volume), cInt(len(volume)), cInt(period)))
}

// VolumeRoc returns the volume rate of change.
func VolumeRoc(volume []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_volume_roc(toCSlice(volume), cInt(len(volume)), cInt(period)))
}

// TwiggsMoneyFlow returns the Twiggs Money Flow.
func TwiggsMoneyFlow(high, low, close, volume []float64, period int) ([]float64, error) {
	return convertResultP(C.ta_twiggs_mf(
		toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume),
		cInt(len(close)), cInt(period),
	))
}

// ============ Volatility ============

// Vortex returns the two Vortex Indicator lines.
func Vortex(high, low, close []float64, period int) (VortexResult, error) {
	values, err := convertResultP(C.ta_vortex(toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), cInt(period)))
	if err != nil {
		return VortexResult{}, err
	}
	viPlus, viMinus := splitPair(values)
	return VortexResult{ViPlus: viPlus, ViMinus: viMinus}, nil
}

// ============ Candlestick patterns ============

// CdlDoji detects Doji candlestick patterns. Values are +100, 0 or -100.
func CdlDoji(open, high, low, close []float64, dojiPct float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_doji(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), C.double(dojiPct)))
}

// CdlDragonflyDoji detects Dragonfly Doji patterns.
func CdlDragonflyDoji(open, high, low, close []float64, dojiPct float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_dragonfly_doji(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), C.double(dojiPct)))
}

// CdlGravestoneDoji detects Gravestone Doji patterns.
func CdlGravestoneDoji(open, high, low, close []float64, dojiPct float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_gravestone_doji(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), C.double(dojiPct)))
}

// CdlLongLeggedDoji detects Long-Legged Doji patterns.
func CdlLongLeggedDoji(open, high, low, close []float64, dojiPct float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_long_legged_doji(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), C.double(dojiPct)))
}

// CdlHammer detects Hammer patterns.
func CdlHammer(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_hammer(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlInvertedHammer detects Inverted Hammer patterns.
func CdlInvertedHammer(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_inverted_hammer(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlHangingMan detects Hanging Man patterns.
func CdlHangingMan(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_hanging_man(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlShootingStar detects Shooting Star patterns.
func CdlShootingStar(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_shooting_star(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlEngulfing detects Engulfing patterns.
func CdlEngulfing(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_engulfing(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlHarami detects Harami patterns.
func CdlHarami(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_harami(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlMorningStar detects Morning Star patterns.
func CdlMorningStar(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_morning_star(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlEveningStar detects Evening Star patterns.
func CdlEveningStar(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_evening_star(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlThreeWhiteSoldiers detects Three White Soldiers patterns.
func CdlThreeWhiteSoldiers(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_three_white_soldiers(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlThreeBlackCrows detects Three Black Crows patterns.
func CdlThreeBlackCrows(open, high, low, close []float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_three_black_crows(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close))))
}

// CdlMarubozu detects Marubozu patterns.
func CdlMarubozu(open, high, low, close []float64, shadowPct float64) ([]int32, error) {
	return convertIntResult(C.ta_cdl_marubozu(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(close)), C.double(shadowPct)))
}
