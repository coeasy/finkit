package ta

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

extern TaResult* ta_sma(const double *input, int length, int period);
extern TaResult* ta_ema(const double *input, int length, int period);
extern TaResult* ta_wma(const double *input, int length, int period);
extern TaResult* ta_dema(const double *input, int length, int period);
extern TaResult* ta_tema(const double *input, int length, int period);
extern TaResult* ta_kama(const double *input, int length, int period);
extern TaResult* ta_t3(const double *input, int length, int period, double vfactor);

extern TaResult* ta_rsi(const double *input, int length, int period);
extern TaResult* ta_macd(const double *input, int length, int fast_period, int slow_period, int signal_period);
extern TaResult* ta_stoch(const double *high, const double *low, const double *close, int length, int k_period, int k_slow, int d_period);
extern TaResult* ta_adx(const double *high, const double *low, const double *close, int length, int period);
extern TaResult* ta_aroon(const double *high, const double *low, int length, int period);
extern TaResult* ta_cci(const double *high, const double *low, const double *close, int length, int period);
extern TaResult* ta_mom(const double *input, int length, int period);
extern TaResult* ta_roc(const double *input, int length, int period);
extern TaResult* ta_willr(const double *high, const double *low, const double *close, int length, int period);

extern TaResult* ta_obv(const double *close, const double *volume, int length);
extern TaResult* ta_ad(const double *high, const double *low, const double *close, const double *volume, int length);
extern TaResult* ta_ad_osc(const double *high, const double *low, const double *close, const double *volume, int length, int fast_period, int slow_period);

extern TaResult* ta_atr(const double *high, const double *low, const double *close, int length, int period);
extern TaResult* ta_natr(const double *high, const double *low, const double *close, int length, int period);
extern TaResult* ta_trange(const double *high, const double *low, const double *close, int length);
extern TaResult* ta_bbands(const double *input, int length, int period, double nb_dev_up, double nb_dev_dn);

extern TaResult* ta_ht_dcperiod(const double *input, int length);
extern TaResult* ta_ht_dcphase(const double *input, int length);
extern TaResult* ta_ht_phasor(const double *input, int length);
extern TaResult* ta_ht_sine(const double *input, int length);
extern TaResult* ta_ht_trendmode(const double *input, int length);
extern TaResult* ta_ht_trendline(const double *input, int length);

extern TaResult* ta_zscore(const double *input, int length, int period);
extern TaResult* ta_beta(const double *asset, const double *benchmark, int length, int period);
extern TaResult* ta_correlation(const double *input_a, const double *input_b, int length, int period);
extern TaResult* ta_std_dev(const double *input, int length, int period, double nb_dev);
extern TaResult* ta_linear_reg(const double *input, int length, int period);
extern TaResult* ta_tsf(const double *input, int length, int period);

extern const char* ta_version();
extern void ta_free_result(TaResult *result);

extern char* ta_formula_eval(const char *source, const double *open, const double *high, const double *low, const double *close, const double *volume, int length);
extern char* ta_formula_eval_zc_exec(const char *source, const double *open, const double *high, const double *low, const double *close, const double *volume, int length);
extern int ta_formula_validate(const char *source);
extern char* ta_formula_get_template(const char *name);
extern char* ta_formula_search_templates(const char *keyword);
extern char* ta_formula_list_categories();

extern char* ta_darvas_box_json(const double *high, const double *low, const double *close, int length, int lookback, int confirmation);
extern char* ta_renko_json(const double *high, const double *low, int length, double box_size);
extern char* ta_kagi_json(const double *close, int length, double reversal);
extern char* ta_point_and_figure_json(const double *high, const double *low, int length, double box_size, int reversal);
extern char* ta_three_line_break_json(const double *close, int length, int lines);
extern char* ta_williams_alligator_json(const double *close, int length);
extern char* ta_heikin_ashi_json(const double *open, const double *high, const double *low, const double *close, int length);

extern void ta_free_string(char *s);

extern void* ta_streaming_sma_new(int period);
extern double ta_streaming_sma_update(void *handle, double value);
extern void ta_streaming_sma_reset(void *handle);
extern void ta_streaming_sma_free(void *handle);

extern void* ta_streaming_ema_new(int period);
extern double ta_streaming_ema_update(void *handle, double value);
extern void ta_streaming_ema_reset(void *handle);
extern void ta_streaming_ema_free(void *handle);

extern void* ta_streaming_rsi_new(int period);
extern double ta_streaming_rsi_update(void *handle, double value);
extern void ta_streaming_rsi_reset(void *handle);
extern void ta_streaming_rsi_free(void *handle);

extern void* ta_streaming_macd_new(int fast, int slow, int signal);
extern int ta_streaming_macd_update(void *handle, double value, double *macd_out, double *signal_out, double *hist_out);
extern void ta_streaming_macd_reset(void *handle);
extern void ta_streaming_macd_free(void *handle);

extern void* ta_streaming_bbands_new(int period, double nb_dev_up, double nb_dev_dn);
extern int ta_streaming_bbands_update(void *handle, double value, double *upper_out, double *middle_out, double *lower_out);
extern void ta_streaming_bbands_reset(void *handle);
extern void ta_streaming_bbands_free(void *handle);

extern void* ta_streaming_atr_new(int period);
extern double ta_streaming_atr_update_hlc(void *handle, double high, double low, double close);
extern void ta_streaming_atr_reset(void *handle);
extern void ta_streaming_atr_free(void *handle);
*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// Version returns the library version string.
func Version() string {
	return C.GoString(C.ta_version())
}

// convertResult converts a C TaResult to a Go slice and frees the C memory.
func convertResult(result *C.TaResult) ([]float64, error) {
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

// convertMultiResult converts a C TaResult containing multiple concatenated arrays.
func convertMultiResult(result *C.TaResult, numArrays int) ([][]float64, error) {
	defer C.ta_free_result(result)

	if result.error != nil {
		return nil, errors.New(C.GoString(result.error))
	}

	if result.data == nil || result.length == 0 {
		return nil, nil
	}

	totalLength := int(result.length)
	arrayLength := totalLength / numArrays

	goSlice := unsafe.Slice((*float64)(result.data), totalLength)

	arrays := make([][]float64, numArrays)
	for i := 0; i < numArrays; i++ {
		arrays[i] = make([]float64, arrayLength)
		copy(arrays[i], goSlice[i*arrayLength:(i+1)*arrayLength])
	}

	return arrays, nil
}

// toCSlice converts a Go float64 slice to a C array pointer.
func toCSlice(input []float64) *C.double {
	if len(input) == 0 {
		return nil
	}
	return (*C.double)(unsafe.Pointer(&input[0]))
}

// cInt returns the C int representation of an int.
func cInt(n int) C.int {
	return C.int(n)
}

// cDouble returns the C double representation of a float64.
func cDouble(f float64) C.double {
	return C.double(f)
}

// ===================== Moving Averages =====================

// Sma calculates the Simple Moving Average.
//
// The Simple Moving Average (SMA) is calculated by taking the arithmetic mean
// of a given set of values over a specified number of periods.
//
// Parameters:
//   - input: Input data series (e.g., closing prices)
//   - period: Number of periods to calculate the average
//
// Returns:
//   - []float64: SMA values (initial values will be NaN until enough data is available)
//   - error: Error if calculation fails
func Sma(input []float64, period int) ([]float64, error) {
	result := C.ta_sma(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Ema calculates the Exponential Moving Average.
//
// The Exponential Moving Average (EMA) gives more weight to recent prices,
// making it more responsive to new information than the SMA.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the EMA calculation
//
// Returns:
//   - []float64: EMA values
//   - error: Error if calculation fails
func Ema(input []float64, period int) ([]float64, error) {
	result := C.ta_ema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Wma calculates the Weighted Moving Average.
//
// The Weighted Moving Average (WMA) assigns a linearly decreasing weight
// to each data point, giving more importance to recent data.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the WMA calculation
//
// Returns:
//   - []float64: WMA values
//   - error: Error if calculation fails
func Wma(input []float64, period int) ([]float64, error) {
	result := C.ta_wma(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Dema calculates the Double Exponential Moving Average.
//
// DEMA reduces the lag of traditional EMAs by using a combination of
// single and double-smoothed EMAs.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the DEMA calculation
//
// Returns:
//   - []float64: DEMA values
//   - error: Error if calculation fails
func Dema(input []float64, period int) ([]float64, error) {
	result := C.ta_dema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Tema calculates the Triple Exponential Moving Average.
//
// TEMA further reduces lag by using a combination of single, double,
// and triple-smoothed EMAs.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the TEMA calculation
//
// Returns:
//   - []float64: TEMA values
//   - error: Error if calculation fails
func Tema(input []float64, period int) ([]float64, error) {
	result := C.ta_tema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Kama calculates the Kaufman Adaptive Moving Average.
//
// KAMA adapts to market volatility by adjusting the smoothing constant
// based on the Efficiency Ratio of price movements.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the KAMA calculation
//
// Returns:
//   - []float64: KAMA values
//   - error: Error if calculation fails
func Kama(input []float64, period int) ([]float64, error) {
	result := C.ta_kama(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// T3 calculates the T3 Moving Average.
//
// T3 is a smooth moving average that uses exponential smoothing with a
// volume factor to reduce lag and improve signal quality.
//
// Parameters:
//   - input: Input data series
//   - period: Number of periods for the T3 calculation
//   - vfactor: Volume factor (0 to 1, typically 0.7)
//
// Returns:
//   - []float64: T3 values
//   - error: Error if calculation fails
func T3(input []float64, period int, vfactor float64) ([]float64, error) {
	result := C.ta_t3(toCSlice(input), cInt(len(input)), cInt(period), cDouble(vfactor))
	return convertResult(result)
}

// ===================== Momentum Indicators =====================

// Rsi calculates the Relative Strength Index.
//
// RSI measures the magnitude of recent price changes to evaluate
// overbought or oversold conditions in the price of an asset.
//
// Parameters:
//   - input: Input data series (typically close prices)
//   - period: Lookback period (commonly 14)
//
// Returns:
//   - []float64: RSI values (0-100 range)
//   - error: Error if calculation fails
func Rsi(input []float64, period int) ([]float64, error) {
	result := C.ta_rsi(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Macd calculates the Moving Average Convergence Divergence.
//
// MACD shows the relationship between two EMAs of a security's price.
// It consists of three components: the MACD line, signal line, and histogram.
//
// Parameters:
//   - input: Input data series
//   - fastPeriod: Fast EMA period (commonly 12)
//   - slowPeriod: Slow EMA period (commonly 26)
//   - signalPeriod: Signal line EMA period (commonly 9)
//
// Returns:
//   - *MacdResult: Contains MACD, Signal, and Hist arrays
//   - error: Error if calculation fails
func Macd(input []float64, fastPeriod, slowPeriod, signalPeriod int) (*MacdResult, error) {
	result := C.ta_macd(
		toCSlice(input),
		cInt(len(input)),
		cInt(fastPeriod),
		cInt(slowPeriod),
		cInt(signalPeriod),
	)

	arrays, err := convertMultiResult(result, 3)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 3 {
		return nil, errors.New("unexpected MACD result format")
	}

	return &MacdResult{
		Macd:   arrays[0],
		Signal: arrays[1],
		Hist:   arrays[2],
	}, nil
}

// Stoch calculates the Stochastic Oscillator.
//
// The Stochastic Oscillator compares a security's closing price to its
// price range over a given period. It consists of %K and %D lines.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - kPeriod: %K lookback period (commonly 14)
//   - kSlow: %K slowing period (commonly 3)
//   - dPeriod: %D period (commonly 3)
//
// Returns:
//   - *StochResult: Contains K and D arrays
//   - error: Error if calculation fails
func Stoch(high, low, close []float64, kPeriod, kSlow, dPeriod int) (*StochResult, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_stoch(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(kPeriod),
		cInt(kSlow),
		cInt(dPeriod),
	)

	arrays, err := convertMultiResult(result, 2)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 2 {
		return nil, errors.New("unexpected Stochastic result format")
	}

	return &StochResult{
		K: arrays[0],
		D: arrays[1],
	}, nil
}

// Adx calculates the Average Directional Index.
//
// ADX measures trend strength regardless of trend direction.
// Higher values indicate a stronger trend.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - period: Lookback period (commonly 14)
//
// Returns:
//   - []float64: ADX values
//   - error: Error if calculation fails
func Adx(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_adx(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// Aroon calculates the Aroon Indicator.
//
// Aroon identifies trend changes and measures the strength of the trend.
// It consists of Aroon Up and Aroon Down lines.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - period: Lookback period (commonly 14 or 25)
//
// Returns:
//   - *AroonResult: Contains AroonUp and AroonDown arrays
//   - error: Error if calculation fails
func Aroon(high, low []float64, period int) (*AroonResult, error) {
	length := len(high)
	if length != len(low) {
		return nil, errors.New("high and low must have the same length")
	}

	result := C.ta_aroon(
		toCSlice(high),
		toCSlice(low),
		cInt(length),
		cInt(period),
	)

	arrays, err := convertMultiResult(result, 2)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 2 {
		return nil, errors.New("unexpected Aroon result format")
	}

	return &AroonResult{
		AroonUp:   arrays[0],
		AroonDown: arrays[1],
	}, nil
}

// Cci calculates the Commodity Channel Index.
//
// CCI measures the current price level relative to an average price level
// over a given period. It helps identify overbought/oversold conditions.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - period: Lookback period (commonly 14 or 20)
//
// Returns:
//   - []float64: CCI values
//   - error: Error if calculation fails
func Cci(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_cci(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// Mom calculates the Momentum indicator.
//
// Momentum measures the change in price over a given period.
// It is the difference between the current price and the price n periods ago.
//
// Parameters:
//   - input: Input data series
//   - period: Lookback period
//
// Returns:
//   - []float64: Momentum values
//   - error: Error if calculation fails
func Mom(input []float64, period int) ([]float64, error) {
	result := C.ta_mom(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Roc calculates the Rate of Change.
//
// ROC measures the percentage change in price over a given period.
//
// Parameters:
//   - input: Input data series
//   - period: Lookback period
//
// Returns:
//   - []float64: ROC values (in percentage)
//   - error: Error if calculation fails
func Roc(input []float64, period int) ([]float64, error) {
	result := C.ta_roc(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Willr calculates the Williams %R.
//
// Williams %R is a momentum indicator that measures overbought/oversold levels.
// Values range from -100 to 0.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - period: Lookback period (commonly 14)
//
// Returns:
//   - []float64: Williams %R values
//   - error: Error if calculation fails
func Willr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_willr(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// ===================== Volume Indicators =====================

// Obv calculates the On Balance Volume.
//
// OBV is a cumulative indicator that uses volume flow to predict price changes.
// It adds volume on up days and subtracts volume on down days.
//
// Parameters:
//   - close: Close prices
//   - volume: Volume data
//
// Returns:
//   - []float64: OBV values
//   - error: Error if calculation fails
func Obv(close, volume []float64) ([]float64, error) {
	length := len(close)
	if length != len(volume) {
		return nil, errors.New("close and volume must have the same length")
	}

	result := C.ta_obv(
		toCSlice(close),
		toCSlice(volume),
		cInt(length),
	)
	return convertResult(result)
}

// Ad calculates the Accumulation/Distribution Line.
//
// The A/D Line uses volume and price to assess whether an asset
// is being accumulated or distributed.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - volume: Volume data
//
// Returns:
//   - []float64: A/D Line values
//   - error: Error if calculation fails
func Ad(high, low, close, volume []float64) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) || length != len(volume) {
		return nil, errors.New("high, low, close, and volume must have the same length")
	}

	result := C.ta_ad(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		toCSlice(volume),
		cInt(length),
	)
	return convertResult(result)
}

// AdOsc calculates the Chaikin A/D Oscillator.
//
// The A/D Oscillator measures the momentum of the Accumulation/Distribution Line
// using two EMAs (fast and slow).
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - volume: Volume data
//   - fastPeriod: Fast EMA period (commonly 3)
//   - slowPeriod: Slow EMA period (commonly 10)
//
// Returns:
//   - []float64: A/D Oscillator values
//   - error: Error if calculation fails
func AdOsc(high, low, close, volume []float64, fastPeriod, slowPeriod int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) || length != len(volume) {
		return nil, errors.New("high, low, close, and volume must have the same length")
	}

	result := C.ta_ad_osc(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		toCSlice(volume),
		cInt(length),
		cInt(fastPeriod),
		cInt(slowPeriod),
	)
	return convertResult(result)
}

// ===================== Volatility Indicators =====================

// Atr calculates the Average True Range.
//
// ATR measures market volatility by decomposing the entire range of
// an asset price for that period.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - period: Lookback period (commonly 14)
//
// Returns:
//   - []float64: ATR values
//   - error: Error if calculation fails
func Atr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_atr(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// Natr calculates the Normalized Average True Range.
//
// NATR is the ATR expressed as a percentage of the close price,
// making it easier to compare across different securities.
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - period: Lookback period (commonly 14)
//
// Returns:
//   - []float64: NATR values (in percentage)
//   - error: Error if calculation fails
func Natr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_natr(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// Trange calculates the True Range.
//
// True Range is the greatest of:
//   - High - Low
//   - |High - Previous Close|
//   - |Low - Previous Close|
//
// Parameters:
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//
// Returns:
//   - []float64: True Range values
//   - error: Error if calculation fails
func Trange(high, low, close []float64) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) {
		return nil, errors.New("high, low, and close must have the same length")
	}

	result := C.ta_trange(
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		cInt(length),
	)
	return convertResult(result)
}

// Bbands calculates the Bollinger Bands.
//
// Bollinger Bands consist of a middle band (SMA) and two outer bands
// that are standard deviations away from the middle band.
//
// Parameters:
//   - input: Input data series
//   - period: Lookback period (commonly 20)
//   - nbDevUp: Number of standard deviations for upper band (commonly 2.0)
//   - nbDevDn: Number of standard deviations for lower band (commonly 2.0)
//
// Returns:
//   - *BbandsResult: Contains Upper, Middle, and Lower arrays
//   - error: Error if calculation fails
func Bbands(input []float64, period int, nbDevUp, nbDevDn float64) (*BbandsResult, error) {
	result := C.ta_bbands(
		toCSlice(input),
		cInt(len(input)),
		cInt(period),
		cDouble(nbDevUp),
		cDouble(nbDevDn),
	)

	arrays, err := convertMultiResult(result, 3)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 3 {
		return nil, errors.New("unexpected Bollinger Bands result format")
	}

	return &BbandsResult{
		Upper:  arrays[0],
		Middle: arrays[1],
		Lower:  arrays[2],
	}, nil
}

// ===================== Hilbert Transform Indicators =====================

// HtDcPeriod calculates the Hilbert Transform - Dominant Cycle Period.
//
// Measures the dominant cycle period of the price series using
// the Hilbert Transform.
//
// Parameters:
//   - input: Input data series (typically typical price)
//
// Returns:
//   - []float64: Dominant cycle period values
//   - error: Error if calculation fails
func HtDcPeriod(input []float64) ([]float64, error) {
	result := C.ta_ht_dcperiod(toCSlice(input), cInt(len(input)))
	return convertResult(result)
}

// HtDcPhase calculates the Hilbert Transform - Dominant Cycle Phase.
//
// Measures the dominant cycle phase of the price series, indicating
// where the current price is within the dominant cycle (0-360 degrees).
//
// Parameters:
//   - input: Input data series
//
// Returns:
//   - []float64: Dominant cycle phase values (in degrees)
//   - error: Error if calculation fails
func HtDcPhase(input []float64) ([]float64, error) {
	result := C.ta_ht_dcphase(toCSlice(input), cInt(len(input)))
	return convertResult(result)
}

// HtPhasor calculates the Hilbert Transform - Phasor Components.
//
// Returns the in-phase and quadrature components of the Hilbert Transform.
// These components represent the signal decomposed into two orthogonal parts.
//
// Parameters:
//   - input: Input data series
//
// Returns:
//   - *HtPhasorResult: Contains InPhase and Quadrature arrays
//   - error: Error if calculation fails
func HtPhasor(input []float64) (*HtPhasorResult, error) {
	result := C.ta_ht_phasor(toCSlice(input), cInt(len(input)))

	arrays, err := convertMultiResult(result, 2)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 2 {
		return nil, errors.New("unexpected Hilbert Phasor result format")
	}

	return &HtPhasorResult{
		InPhase:    arrays[0],
		Quadrature: arrays[1],
	}, nil
}

// HtSine calculates the Hilbert Transform - Sine Wave.
//
// Returns the sine and lead sine wave components derived from
// the Hilbert Transform.
//
// Parameters:
//   - input: Input data series
//
// Returns:
//   - *HtSineResult: Contains Sine and LeadSine arrays
//   - error: Error if calculation fails
func HtSine(input []float64) (*HtSineResult, error) {
	result := C.ta_ht_sine(toCSlice(input), cInt(len(input)))

	arrays, err := convertMultiResult(result, 2)
	if err != nil {
		return nil, err
	}

	if len(arrays) != 2 {
		return nil, errors.New("unexpected Hilbert Sine result format")
	}

	return &HtSineResult{
		Sine:     arrays[0],
		LeadSine: arrays[1],
	}, nil
}

// HtTrendMode calculates the Hilbert Transform - Trend vs Cycle Mode.
//
// Indicates whether the market is in trend mode (1.0) or cycle mode (0.0).
//
// Parameters:
//   - input: Input data series
//
// Returns:
//   - []float64: Mode values (1.0 for trend, 0.0 for cycle)
//   - error: Error if calculation fails
func HtTrendMode(input []float64) ([]float64, error) {
	result := C.ta_ht_trendmode(toCSlice(input), cInt(len(input)))
	return convertResult(result)
}

// HtTrendLine calculates the Hilbert Transform - Instantaneous Trendline.
//
// Computes the instantaneous trendline of the price series using
// the Hilbert Transform.
//
// Parameters:
//   - input: Input data series (typically typical price)
//
// Returns:
//   - []float64: Trendline values
//   - error: Error if calculation fails
func HtTrendLine(input []float64) ([]float64, error) {
	result := C.ta_ht_trendline(toCSlice(input), cInt(len(input)))
	return convertResult(result)
}

// ===================== Statistical Functions =====================

// ZScore calculates the Z-Score (standard score).
//
// Z-Score indicates how many standard deviations a data point is
// from the mean of a rolling window.
//
// Parameters:
//   - input: Input data series
//   - period: Rolling window size
//
// Returns:
//   - []float64: Z-Score values
//   - error: Error if calculation fails
func ZScore(input []float64, period int) ([]float64, error) {
	result := C.ta_zscore(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Beta calculates the Beta coefficient between two assets.
//
// Beta measures the volatility of an asset relative to a benchmark.
// Beta > 1 indicates higher volatility than the benchmark.
//
// Parameters:
//   - asset: Asset price series (e.g., stock)
//   - benchmark: Benchmark price series (e.g., market index)
//   - period: Rolling window size
//
// Returns:
//   - []float64: Beta values
//   - error: Error if calculation fails
func Beta(asset, benchmark []float64, period int) ([]float64, error) {
	length := len(asset)
	if length != len(benchmark) {
		return nil, errors.New("asset and benchmark must have the same length")
	}

	result := C.ta_beta(
		toCSlice(asset),
		toCSlice(benchmark),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// Correlation calculates the Pearson correlation coefficient.
//
// Measures the linear correlation between two data series over
// a rolling window. Values range from -1 to 1.
//
// Parameters:
//   - inputA: First data series
//   - inputB: Second data series
//   - period: Rolling window size
//
// Returns:
//   - []float64: Correlation values
//   - error: Error if calculation fails
func Correlation(inputA, inputB []float64, period int) ([]float64, error) {
	length := len(inputA)
	if length != len(inputB) {
		return nil, errors.New("inputA and inputB must have the same length")
	}

	result := C.ta_correlation(
		toCSlice(inputA),
		toCSlice(inputB),
		cInt(length),
		cInt(period),
	)
	return convertResult(result)
}

// StdDev calculates the rolling standard deviation.
//
// Parameters:
//   - input: Input data series
//   - period: Rolling window size
//   - nbDev: Number of standard deviations (for API compatibility)
//
// Returns:
//   - []float64: Standard deviation values
//   - error: Error if calculation fails
func StdDev(input []float64, period int, nbDev float64) ([]float64, error) {
	result := C.ta_std_dev(toCSlice(input), cInt(len(input)), cInt(period), cDouble(nbDev))
	return convertResult(result)
}

// LinearReg calculates the rolling linear regression.
//
// Uses least squares method to calculate the predicted values
// of rolling linear regression.
//
// Parameters:
//   - input: Input data series
//   - period: Rolling window size
//
// Returns:
//   - []float64: Linear regression predicted values
//   - error: Error if calculation fails
func LinearReg(input []float64, period int) ([]float64, error) {
	result := C.ta_linear_reg(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Tsf calculates the Time Series Forecast.
//
// TSF is the predicted value of the linear regression curve
// extrapolated one time unit into the future.
//
// Parameters:
//   - input: Input data series
//   - period: Rolling window size
//
// Returns:
//   - []float64: Time series forecast values
//   - error: Error if calculation fails
func Tsf(input []float64, period int) ([]float64, error) {
	result := C.ta_tsf(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// ===================== Formula Engine =====================

// FormulaEval evaluates a formula string against OHLCV data.
//
// It returns a map of variable names to their computed arrays,
// plus a special "__final__" key containing the last expression result.
//
// Parameters:
//   - source: Formula source code string
//   - open: Open prices
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - volume: Volume data
//
// Returns:
//   - map[string][]float64: Computed variables including "__final__"
//   - error: Error if evaluation fails
func FormulaEval(source string, open, high, low, close, volume []float64) (map[string][]float64, error) {
	length := len(open)
	if len(high) != length || len(low) != length || len(close) != length || len(volume) != length {
		return nil, errors.New("all input arrays must have the same length")
	}

	cSource := C.CString(source)
	defer C.free(unsafe.Pointer(cSource))

	cResult := C.ta_formula_eval(
		cSource,
		toCSlice(open),
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		toCSlice(volume),
		cInt(length),
	)
	defer C.ta_free_string(cResult)

	resultStr := C.GoString(cResult)

	if len(resultStr) > 6 && resultStr[:6] == "error:" {
		return nil, errors.New(resultStr[7:])
	}

	var result map[string][]*float64
	if err := json.Unmarshal([]byte(resultStr), &result); err != nil {
		return nil, err
	}

	out := make(map[string][]float64)
	for name, values := range result {
		arr := make([]float64, len(values))
		for i, v := range values {
			if v == nil {
				arr[i] = 0 // NaN represented as 0 in JSON
			} else {
				arr[i] = *v
			}
		}
		out[name] = arr
	}

	return out, nil
}

// FormulaValidate checks if a formula source string is syntactically valid.
//
// Parameters:
//   - source: Formula source code string
//
// Returns:
//   - bool: true if the formula is valid, false otherwise
func FormulaValidate(source string) bool {
	cSource := C.CString(source)
	defer C.free(unsafe.Pointer(cSource))

	result := C.ta_formula_validate(cSource)
	return result == 1
}

// formulaJSONResult validates and owns a JSON string returned by the native layer.
func formulaJSONResult(result *C.char) (string, error) {
	if result == nil {
		return "", errors.New("native formula call returned a null result")
	}
	defer C.ta_free_string(result)

	value := C.GoString(result)
	if !json.Valid([]byte(value)) {
		return "", errors.New(value)
	}
	return value, nil
}

// FormulaGetTemplate returns one named formula template as JSON.
func FormulaGetTemplate(name string) (string, error) {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	return formulaJSONResult(C.ta_formula_get_template(cName))
}

// FormulaSearchTemplates searches formula templates and returns the JSON array.
func FormulaSearchTemplates(keyword string) (string, error) {
	cKeyword := C.CString(keyword)
	defer C.free(unsafe.Pointer(cKeyword))
	return formulaJSONResult(C.ta_formula_search_templates(cKeyword))
}

// FormulaListCategories returns all formula template categories as JSON.
func FormulaListCategories() (string, error) {
	return formulaJSONResult(C.ta_formula_list_categories())
}

// DarvasBoxJSON returns the Darvas box result as JSON.
func DarvasBoxJSON(high, low, close []float64, lookback, confirmation int) (string, error) {
	if len(high) != len(low) || len(high) != len(close) {
		return "", errors.New("all input arrays must have the same length")
	}
	return formulaJSONResult(C.ta_darvas_box_json(
		toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(high)),
		cInt(lookback), cInt(confirmation),
	))
}

// RenkoJSON returns the Renko result as JSON.
func RenkoJSON(high, low []float64, boxSize float64) (string, error) {
	if len(high) != len(low) {
		return "", errors.New("all input arrays must have the same length")
	}
	return formulaJSONResult(C.ta_renko_json(toCSlice(high), toCSlice(low), cInt(len(high)), C.double(boxSize)))
}

// KagiJSON returns the Kagi result as JSON.
func KagiJSON(close []float64, reversal float64) (string, error) {
	return formulaJSONResult(C.ta_kagi_json(toCSlice(close), cInt(len(close)), C.double(reversal)))
}

// PointAndFigureJSON returns the point-and-figure result as JSON.
func PointAndFigureJSON(high, low []float64, boxSize float64, reversal int) (string, error) {
	if len(high) != len(low) {
		return "", errors.New("all input arrays must have the same length")
	}
	return formulaJSONResult(C.ta_point_and_figure_json(
		toCSlice(high), toCSlice(low), cInt(len(high)),
		C.double(boxSize), cInt(reversal),
	))
}

// ThreeLineBreakJSON returns the three-line-break result as JSON.
func ThreeLineBreakJSON(close []float64, lines int) (string, error) {
	return formulaJSONResult(C.ta_three_line_break_json(toCSlice(close), cInt(len(close)), cInt(lines)))
}

// WilliamsAlligatorJSON returns the Williams Alligator result as JSON.
func WilliamsAlligatorJSON(close []float64) (string, error) {
	return formulaJSONResult(C.ta_williams_alligator_json(toCSlice(close), cInt(len(close))))
}

// HeikinAshiJSON returns the Heikin-Ashi result as JSON.
func HeikinAshiJSON(open, high, low, close []float64) (string, error) {
	if len(open) != len(high) || len(open) != len(low) || len(open) != len(close) {
		return "", errors.New("all input arrays must have the same length")
	}
	return formulaJSONResult(C.ta_heikin_ashi_json(
		toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(open)),
	))
}

// FormulaEvalZeroCopy evaluates a formula string with zero-copy optimization.
//
// Minimizes memory allocations by operating directly on input buffers
// without copying data. This provides the lowest latency execution path
// for latency-sensitive applications.
//
// Parameters:
//   - source: Formula source code string
//   - open: Open prices
//   - high: High prices
//   - low: Low prices
//   - close: Close prices
//   - volume: Volume data
//
// Returns:
//   - map[string][]float64: Computed variables including "__final__"
//   - error: Error if evaluation fails
func FormulaEvalZeroCopy(source string, open, high, low, close, volume []float64) (map[string][]float64, error) {
	length := len(open)
	if len(high) != length || len(low) != length || len(close) != length || len(volume) != length {
		return nil, errors.New("all input arrays must have the same length")
	}

	cSource := C.CString(source)
	defer C.free(unsafe.Pointer(cSource))

	cResult := C.ta_formula_eval_zc_exec(
		cSource,
		toCSlice(open),
		toCSlice(high),
		toCSlice(low),
		toCSlice(close),
		toCSlice(volume),
		cInt(length),
	)
	defer C.ta_free_string(cResult)

	resultStr := C.GoString(cResult)

	if len(resultStr) > 6 && resultStr[:6] == "error:" {
		return nil, errors.New(resultStr[7:])
	}

	var result map[string][]*float64
	if err := json.Unmarshal([]byte(resultStr), &result); err != nil {
		return nil, err
	}

	out := make(map[string][]float64)
	for name, values := range result {
		arr := make([]float64, len(values))
		for i, v := range values {
			if v == nil {
				arr[i] = 0
			} else {
				arr[i] = *v
			}
		}
		out[name] = arr
	}

	return out, nil
}

// ===================== Streaming Indicators =====================

// StreamingSma is a stateful Simple Moving Average indicator.
type StreamingSma struct {
	handle unsafe.Pointer
}

func NewStreamingSma(period int) *StreamingSma {
	h := C.ta_streaming_sma_new(cInt(period))
	if h == nil {
		return nil
	}
	return &StreamingSma{handle: unsafe.Pointer(h)}
}

func (s *StreamingSma) Update(value float64) float64 {
	return float64(C.ta_streaming_sma_update(s.handle, cDouble(value)))
}

func (s *StreamingSma) Reset() {
	C.ta_streaming_sma_reset(s.handle)
}

func (s *StreamingSma) Free() {
	if s.handle != nil {
		C.ta_streaming_sma_free(s.handle)
		s.handle = nil
	}
}

// StreamingEma is a stateful Exponential Moving Average indicator.
type StreamingEma struct {
	handle unsafe.Pointer
}

func NewStreamingEma(period int) *StreamingEma {
	h := C.ta_streaming_ema_new(cInt(period))
	if h == nil {
		return nil
	}
	return &StreamingEma{handle: unsafe.Pointer(h)}
}

func (s *StreamingEma) Update(value float64) float64 {
	return float64(C.ta_streaming_ema_update(s.handle, cDouble(value)))
}

func (s *StreamingEma) Reset() {
	C.ta_streaming_ema_reset(s.handle)
}

func (s *StreamingEma) Free() {
	if s.handle != nil {
		C.ta_streaming_ema_free(s.handle)
		s.handle = nil
	}
}

// StreamingRsi is a stateful Relative Strength Index indicator.
type StreamingRsi struct {
	handle unsafe.Pointer
}

func NewStreamingRsi(period int) *StreamingRsi {
	h := C.ta_streaming_rsi_new(cInt(period))
	if h == nil {
		return nil
	}
	return &StreamingRsi{handle: unsafe.Pointer(h)}
}

func (s *StreamingRsi) Update(value float64) float64 {
	return float64(C.ta_streaming_rsi_update(s.handle, cDouble(value)))
}

func (s *StreamingRsi) Reset() {
	C.ta_streaming_rsi_reset(s.handle)
}

func (s *StreamingRsi) Free() {
	if s.handle != nil {
		C.ta_streaming_rsi_free(s.handle)
		s.handle = nil
	}
}

type MacdOutput struct {
	Macd   float64
	Signal float64
	Hist   float64
}

type StreamingMacd struct {
	handle unsafe.Pointer
}

func NewStreamingMacd(fastPeriod, slowPeriod, signalPeriod int) *StreamingMacd {
	h := C.ta_streaming_macd_new(cInt(fastPeriod), cInt(slowPeriod), cInt(signalPeriod))
	if h == nil {
		return nil
	}
	return &StreamingMacd{handle: unsafe.Pointer(h)}
}

func (s *StreamingMacd) Update(value float64) (MacdOutput, bool) {
	var macd, sig, hist C.double
	ready := C.ta_streaming_macd_update(s.handle, cDouble(value), &macd, &sig, &hist)
	if ready == 0 {
		return MacdOutput{}, false
	}
	return MacdOutput{Macd: float64(macd), Signal: float64(sig), Hist: float64(hist)}, true
}

func (s *StreamingMacd) Reset() {
	C.ta_streaming_macd_reset(s.handle)
}

func (s *StreamingMacd) Free() {
	if s.handle != nil {
		C.ta_streaming_macd_free(s.handle)
		s.handle = nil
	}
}

type BbandsOutput struct {
	Upper  float64
	Middle float64
	Lower  float64
}

type StreamingBbands struct {
	handle unsafe.Pointer
}

func NewStreamingBbands(period int, nbDevUp, nbDevDn float64) *StreamingBbands {
	h := C.ta_streaming_bbands_new(cInt(period), cDouble(nbDevUp), cDouble(nbDevDn))
	if h == nil {
		return nil
	}
	return &StreamingBbands{handle: unsafe.Pointer(h)}
}

func (s *StreamingBbands) Update(value float64) (BbandsOutput, bool) {
	var upper, middle, lower C.double
	ready := C.ta_streaming_bbands_update(s.handle, cDouble(value), &upper, &middle, &lower)
	if ready == 0 {
		return BbandsOutput{}, false
	}
	return BbandsOutput{Upper: float64(upper), Middle: float64(middle), Lower: float64(lower)}, true
}

func (s *StreamingBbands) Reset() {
	C.ta_streaming_bbands_reset(s.handle)
}

func (s *StreamingBbands) Free() {
	if s.handle != nil {
		C.ta_streaming_bbands_free(s.handle)
		s.handle = nil
	}
}

type StreamingAtr struct {
	handle unsafe.Pointer
}

func NewStreamingAtr(period int) *StreamingAtr {
	h := C.ta_streaming_atr_new(cInt(period))
	if h == nil {
		return nil
	}
	return &StreamingAtr{handle: unsafe.Pointer(h)}
}

func (s *StreamingAtr) Update(high, low, close float64) float64 {
	return float64(C.ta_streaming_atr_update_hlc(s.handle, cDouble(high), cDouble(low), cDouble(close)))
}

func (s *StreamingAtr) Reset() {
	C.ta_streaming_atr_reset(s.handle)
}

func (s *StreamingAtr) Free() {
	if s.handle != nil {
		C.ta_streaming_atr_free(s.handle)
		s.handle = nil
	}
}
