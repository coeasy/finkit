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
extern char* ta_formula_eval_multi(const char *source, const double *open, const double *high, const double *low, const double *close, const double *volume, int length);
extern char* ta_formula_eval_draw(const char *source, const double *open, const double *high, const double *low, const double *close, const double *volume, int length);
extern char* ta_formula_eval_debug(const char *source, const double *open, const double *high, const double *low, const double *close, const double *volume, int length);
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
extern void* ta_streaming_macd_ext_new(int fast, int fast_ma_type, int slow, int slow_ma_type, int signal, int signal_ma_type);
extern int ta_streaming_macd_ext_update(void *handle, double value, double *macd_out, double *signal_out, double *hist_out);
extern void ta_streaming_macd_ext_reset(void *handle);
extern void ta_streaming_macd_ext_free(void *handle);

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
func Sma(input []float64, period int) ([]float64, error) {
	result := C.ta_sma(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Ema calculates the Exponential Moving Average.
func Ema(input []float64, period int) ([]float64, error) {
	result := C.ta_ema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Wma calculates the Weighted Moving Average.
func Wma(input []float64, period int) ([]float64, error) {
	result := C.ta_wma(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Dema calculates the Double Exponential Moving Average.
func Dema(input []float64, period int) ([]float64, error) {
	result := C.ta_dema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Tema calculates the Triple Exponential Moving Average.
func Tema(input []float64, period int) ([]float64, error) {
	result := C.ta_tema(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// Kama calculates the Kaufman Adaptive Moving Average.
func Kama(input []float64, period int) ([]float64, error) {
	result := C.ta_kama(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

// T3 calculates the T3 Moving Average.
func T3(input []float64, period int, vfactor float64) ([]float64, error) {
	result := C.ta_t3(toCSlice(input), cInt(len(input)), cInt(period), cDouble(vfactor))
	return convertResult(result)
}

// ===================== Momentum Indicators =====================

// Rsi calculates the Relative Strength Index.
func Rsi(input []float64, period int) ([]float64, error) {
	result := C.ta_rsi(toCSlice(input), cInt(len(input)), cInt(period))
	return convertResult(result)
}

func Macd(input []float64, fastPeriod, slowPeriod, signalPeriod int) (*MacdResult, error) {
	result := C.ta_macd(toCSlice(input), cInt(len(input)), cInt(fastPeriod), cInt(slowPeriod), cInt(signalPeriod))
	arrays, err := convertMultiResult(result, 3)
	if err != nil { return nil, err }
	if len(arrays) != 3 { return nil, errors.New("unexpected MACD result format") }
	return &MacdResult{Macd: arrays[0], Signal: arrays[1], Hist: arrays[2]}, nil
}

func Stoch(high, low, close []float64, kPeriod, kSlow, dPeriod int) (*StochResult, error) {
	length := len(high)
	if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	result := C.ta_stoch(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(kPeriod), cInt(kSlow), cInt(dPeriod))
	arrays, err := convertMultiResult(result, 2)
	if err != nil { return nil, err }
	if len(arrays) != 2 { return nil, errors.New("unexpected Stochastic result format") }
	return &StochResult{K: arrays[0], D: arrays[1]}, nil
}

func Adx(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_adx(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(period)))
}

func Aroon(high, low []float64, period int) (*AroonResult, error) {
	length := len(high)
	if length != len(low) { return nil, errors.New("high and low must have the same length") }
	arrays, err := convertMultiResult(C.ta_aroon(toCSlice(high), toCSlice(low), cInt(length), cInt(period)), 2)
	if err != nil { return nil, err }
	if len(arrays) != 2 { return nil, errors.New("unexpected Aroon result format") }
	return &AroonResult{AroonUp: arrays[0], AroonDown: arrays[1]}, nil
}

func Cci(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_cci(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(period)))
}
func Mom(input []float64, period int) ([]float64, error) { return convertResult(C.ta_mom(toCSlice(input), cInt(len(input)), cInt(period))) }
func Roc(input []float64, period int) ([]float64, error) { return convertResult(C.ta_roc(toCSlice(input), cInt(len(input)), cInt(period))) }
func Willr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high)
	if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_willr(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(period)))
}

func Obv(close, volume []float64) ([]float64, error) {
	length := len(close); if length != len(volume) { return nil, errors.New("close and volume must have the same length") }
	return convertResult(C.ta_obv(toCSlice(close), toCSlice(volume), cInt(length)))
}
func Ad(high, low, close, volume []float64) ([]float64, error) {
	length := len(high); if length != len(low) || length != len(close) || length != len(volume) { return nil, errors.New("high, low, close, and volume must have the same length") }
	return convertResult(C.ta_ad(toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)))
}
func AdOsc(high, low, close, volume []float64, fastPeriod, slowPeriod int) ([]float64, error) {
	length := len(high); if length != len(low) || length != len(close) || length != len(volume) { return nil, errors.New("high, low, close, and volume must have the same length") }
	return convertResult(C.ta_ad_osc(toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length), cInt(fastPeriod), cInt(slowPeriod)))
}
func Atr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high); if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_atr(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(period)))
}
func Natr(high, low, close []float64, period int) ([]float64, error) {
	length := len(high); if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_natr(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length), cInt(period)))
}
func Trange(high, low, close []float64) ([]float64, error) {
	length := len(high); if length != len(low) || length != len(close) { return nil, errors.New("high, low, and close must have the same length") }
	return convertResult(C.ta_trange(toCSlice(high), toCSlice(low), toCSlice(close), cInt(length)))
}
func Bbands(input []float64, period int, nbDevUp, nbDevDn float64) (*BbandsResult, error) {
	arrays, err := convertMultiResult(C.ta_bbands(toCSlice(input), cInt(len(input)), cInt(period), cDouble(nbDevUp), cDouble(nbDevDn)), 3)
	if err != nil { return nil, err }
	if len(arrays) != 3 { return nil, errors.New("unexpected Bollinger Bands result format") }
	return &BbandsResult{Upper: arrays[0], Middle: arrays[1], Lower: arrays[2]}, nil
}

func HtDcPeriod(input []float64) ([]float64, error) { return convertResult(C.ta_ht_dcperiod(toCSlice(input), cInt(len(input)))) }
func HtDcPhase(input []float64) ([]float64, error) { return convertResult(C.ta_ht_dcphase(toCSlice(input), cInt(len(input)))) }
func HtPhasor(input []float64) (*HtPhasorResult, error) {
	arrays, err := convertMultiResult(C.ta_ht_phasor(toCSlice(input), cInt(len(input))), 2)
	if err != nil { return nil, err }
	if len(arrays) != 2 { return nil, errors.New("unexpected Hilbert Phasor result format") }
	return &HtPhasorResult{InPhase: arrays[0], Quadrature: arrays[1]}, nil
}
func HtSine(input []float64) (*HtSineResult, error) {
	arrays, err := convertMultiResult(C.ta_ht_sine(toCSlice(input), cInt(len(input))), 2)
	if err != nil { return nil, err }
	if len(arrays) != 2 { return nil, errors.New("unexpected Hilbert Sine result format") }
	return &HtSineResult{Sine: arrays[0], LeadSine: arrays[1]}, nil
}
func HtTrendMode(input []float64) ([]float64, error) { return convertResult(C.ta_ht_trendmode(toCSlice(input), cInt(len(input)))) }
func HtTrendLine(input []float64) ([]float64, error) { return convertResult(C.ta_ht_trendline(toCSlice(input), cInt(len(input)))) }
func ZScore(input []float64, period int) ([]float64, error) { return convertResult(C.ta_zscore(toCSlice(input), cInt(len(input)), cInt(period))) }
func Beta(asset, benchmark []float64, period int) ([]float64, error) {
	length := len(asset); if length != len(benchmark) { return nil, errors.New("asset and benchmark must have the same length") }
	return convertResult(C.ta_beta(toCSlice(asset), toCSlice(benchmark), cInt(length), cInt(period)))
}
func Correlation(inputA, inputB []float64, period int) ([]float64, error) {
	length := len(inputA); if length != len(inputB) { return nil, errors.New("inputA and inputB must have the same length") }
	return convertResult(C.ta_correlation(toCSlice(inputA), toCSlice(inputB), cInt(length), cInt(period)))
}
func StdDev(input []float64, period int, nbDev float64) ([]float64, error) { return convertResult(C.ta_std_dev(toCSlice(input), cInt(len(input)), cInt(period), cDouble(nbDev))) }
func LinearReg(input []float64, period int) ([]float64, error) { return convertResult(C.ta_linear_reg(toCSlice(input), cInt(len(input)), cInt(period))) }
func Tsf(input []float64, period int) ([]float64, error) { return convertResult(C.ta_tsf(toCSlice(input), cInt(len(input)), cInt(period))) }

func FormulaEval(source string, open, high, low, close, volume []float64) (map[string][]float64, error) {
	length := len(open)
	if len(high) != length || len(low) != length || len(close) != length || len(volume) != length { return nil, errors.New("all input arrays must have the same length") }
	cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource))
	cResult := C.ta_formula_eval(cSource, toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)); defer C.ta_free_string(cResult)
	resultStr := C.GoString(cResult)
	if len(resultStr) > 6 && resultStr[:6] == "error:" { return nil, errors.New(resultStr[7:]) }
	var result map[string][]*float64
	if err := json.Unmarshal([]byte(resultStr), &result); err != nil { return nil, err }
	out := make(map[string][]float64)
	for name, values := range result {
		arr := make([]float64, len(values))
		for i, v := range values { if v != nil { arr[i] = *v } }
		out[name] = arr
	}
	return out, nil
}
func FormulaValidate(source string) bool { cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource)); return C.ta_formula_validate(cSource) == 1 }
func formulaJSONResult(result *C.char) (string, error) {
	if result == nil { return "", errors.New("native formula call returned a null result") }
	defer C.ta_free_string(result)
	value := C.GoString(result); if !json.Valid([]byte(value)) { return "", errors.New(value) }
	return value, nil
}
func FormulaEvalMultiJSON(source string, open, high, low, close, volume []float64) (string, error) {
	length := len(open); if len(high) != length || len(low) != length || len(close) != length || len(volume) != length { return "", errors.New("all input arrays must have the same length") }
	cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource)); return formulaJSONResult(C.ta_formula_eval_multi(cSource, toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)))
}
func FormulaEvalDrawJSON(source string, open, high, low, close, volume []float64) (string, error) {
	length := len(open); if len(high) != length || len(low) != length || len(close) != length || len(volume) != length { return "", errors.New("all input arrays must have the same length") }
	cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource)); return formulaJSONResult(C.ta_formula_eval_draw(cSource, toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)))
}
func FormulaEvalDebugJSON(source string, open, high, low, close, volume []float64) (string, error) {
	length := len(open); if len(high) != length || len(low) != length || len(close) != length || len(volume) != length { return "", errors.New("all input arrays must have the same length") }
	cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource)); return formulaJSONResult(C.ta_formula_eval_debug(cSource, toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)))
}
func FormulaGetTemplate(name string) (string, error) { cName := C.CString(name); defer C.free(unsafe.Pointer(cName)); return formulaJSONResult(C.ta_formula_get_template(cName)) }
func FormulaSearchTemplates(keyword string) (string, error) { cKeyword := C.CString(keyword); defer C.free(unsafe.Pointer(cKeyword)); return formulaJSONResult(C.ta_formula_search_templates(cKeyword)) }
func FormulaListCategories() (string, error) { return formulaJSONResult(C.ta_formula_list_categories()) }
func DarvasBoxJSON(high, low, close []float64, lookback, confirmation int) (string, error) {
	if len(high) != len(low) || len(high) != len(close) { return "", errors.New("all input arrays must have the same length") }
	return formulaJSONResult(C.ta_darvas_box_json(toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(high)), cInt(lookback), cInt(confirmation)))
}
func RenkoJSON(high, low []float64, boxSize float64) (string, error) {
	if len(high) != len(low) { return "", errors.New("all input arrays must have the same length") }
	return formulaJSONResult(C.ta_renko_json(toCSlice(high), toCSlice(low), cInt(len(high)), C.double(boxSize)))
}
func KagiJSON(close []float64, reversal float64) (string, error) { return formulaJSONResult(C.ta_kagi_json(toCSlice(close), cInt(len(close)), C.double(reversal))) }
func PointAndFigureJSON(high, low []float64, boxSize float64, reversal int) (string, error) {
	if len(high) != len(low) { return "", errors.New("all input arrays must have the same length") }
	return formulaJSONResult(C.ta_point_and_figure_json(toCSlice(high), toCSlice(low), cInt(len(high)), C.double(boxSize), cInt(reversal)))
}
func ThreeLineBreakJSON(close []float64, lines int) (string, error) { return formulaJSONResult(C.ta_three_line_break_json(toCSlice(close), cInt(len(close)), cInt(lines))) }
func WilliamsAlligatorJSON(close []float64) (string, error) { return formulaJSONResult(C.ta_williams_alligator_json(toCSlice(close), cInt(len(close)))) }
func HeikinAshiJSON(open, high, low, close []float64) (string, error) {
	if len(open) != len(high) || len(open) != len(low) || len(open) != len(close) { return "", errors.New("all input arrays must have the same length") }
	return formulaJSONResult(C.ta_heikin_ashi_json(toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), cInt(len(open))))
}
func FormulaEvalZeroCopy(source string, open, high, low, close, volume []float64) (map[string][]float64, error) {
	length := len(open); if len(high) != length || len(low) != length || len(close) != length || len(volume) != length { return nil, errors.New("all input arrays must have the same length") }
	cSource := C.CString(source); defer C.free(unsafe.Pointer(cSource))
	cResult := C.ta_formula_eval_zc_exec(cSource, toCSlice(open), toCSlice(high), toCSlice(low), toCSlice(close), toCSlice(volume), cInt(length)); defer C.ta_free_string(cResult)
	resultStr := C.GoString(cResult); if len(resultStr) > 6 && resultStr[:6] == "error:" { return nil, errors.New(resultStr[7:]) }
	var result map[string][]*float64; if err := json.Unmarshal([]byte(resultStr), &result); err != nil { return nil, err }
	out := make(map[string][]float64); for name, values := range result { arr := make([]float64, len(values)); for i, v := range values { if v != nil { arr[i] = *v } }; out[name] = arr }; return out, nil
}

// ===================== Streaming Indicators =====================

type StreamingSma struct { handle unsafe.Pointer }
func NewStreamingSma(period int) *StreamingSma { h := C.ta_streaming_sma_new(cInt(period)); if h == nil { return nil }; return &StreamingSma{handle: unsafe.Pointer(h)} }
func (s *StreamingSma) Update(value float64) float64 { return float64(C.ta_streaming_sma_update(s.handle, cDouble(value))) }
func (s *StreamingSma) Reset() { C.ta_streaming_sma_reset(s.handle) }
func (s *StreamingSma) Free() { if s.handle != nil { C.ta_streaming_sma_free(s.handle); s.handle = nil } }
type StreamingEma struct { handle unsafe.Pointer }
func NewStreamingEma(period int) *StreamingEma { h := C.ta_streaming_ema_new(cInt(period)); if h == nil { return nil }; return &StreamingEma{handle: unsafe.Pointer(h)} }
func (s *StreamingEma) Update(value float64) float64 { return float64(C.ta_streaming_ema_update(s.handle, cDouble(value))) }
func (s *StreamingEma) Reset() { C.ta_streaming_ema_reset(s.handle) }
func (s *StreamingEma) Free() { if s.handle != nil { C.ta_streaming_ema_free(s.handle); s.handle = nil } }
type StreamingRsi struct { handle unsafe.Pointer }
func NewStreamingRsi(period int) *StreamingRsi { h := C.ta_streaming_rsi_new(cInt(period)); if h == nil { return nil }; return &StreamingRsi{handle: unsafe.Pointer(h)} }
func (s *StreamingRsi) Update(value float64) float64 { return float64(C.ta_streaming_rsi_update(s.handle, cDouble(value))) }
func (s *StreamingRsi) Reset() { C.ta_streaming_rsi_reset(s.handle) }
func (s *StreamingRsi) Free() { if s.handle != nil { C.ta_streaming_rsi_free(s.handle); s.handle = nil } }

type MacdOutput struct { Macd float64; Signal float64; Hist float64 }
type StreamingMacd struct { handle unsafe.Pointer }
type MaType int
const (
	MaSMA MaType = iota
	MaEMA
	MaWMA
	MaDEMA
	MaTEMA
	MaKAMA
	MaT3
	MaTRIMA
	MaHMA
	MaALMA
	MaVIDYA
)
type StreamingMacdExt struct { handle unsafe.Pointer }
func NewStreamingMacdExt(fastPeriod int, fastMa MaType, slowPeriod int, slowMa MaType, signalPeriod int, signalMa MaType) *StreamingMacdExt {
	h := C.ta_streaming_macd_ext_new(
		cInt(fastPeriod), cInt(int(fastMa)), cInt(slowPeriod), cInt(int(slowMa)), cInt(signalPeriod), cInt(int(signalMa)),
	)
	if h == nil { return nil }
	return &StreamingMacdExt{handle: unsafe.Pointer(h)}
}
func (s *StreamingMacdExt) Update(value float64) (MacdOutput, bool) {
	if s == nil || s.handle == nil { return MacdOutput{}, false }
	var macd, sig, hist C.double
	ready := C.ta_streaming_macd_ext_update(s.handle, cDouble(value), &macd, &sig, &hist)
	if ready == 0 { return MacdOutput{}, false }
	return MacdOutput{Macd: float64(macd), Signal: float64(sig), Hist: float64(hist)}, true
}
func (s *StreamingMacdExt) Reset() { if s != nil && s.handle != nil { C.ta_streaming_macd_ext_reset(s.handle) } }
func (s *StreamingMacdExt) Free() { if s != nil && s.handle != nil { C.ta_streaming_macd_ext_free(s.handle); s.handle = nil } }
func NewStreamingMacd(fastPeriod, slowPeriod, signalPeriod int) *StreamingMacd { h := C.ta_streaming_macd_new(cInt(fastPeriod), cInt(slowPeriod), cInt(signalPeriod)); if h == nil { return nil }; return &StreamingMacd{handle: unsafe.Pointer(h)} }
func (s *StreamingMacd) Update(value float64) (MacdOutput, bool) { var macd, sig, hist C.double; ready := C.ta_streaming_macd_update(s.handle, cDouble(value), &macd, &sig, &hist); if ready == 0 { return MacdOutput{}, false }; return MacdOutput{Macd: float64(macd), Signal: float64(sig), Hist: float64(hist)}, true }
func (s *StreamingMacd) Reset() { C.ta_streaming_macd_reset(s.handle) }
func (s *StreamingMacd) Free() { if s.handle != nil { C.ta_streaming_macd_free(s.handle); s.handle = nil } }
type BbandsOutput struct { Upper float64; Middle float64; Lower float64 }
type StreamingBbands struct { handle unsafe.Pointer }
func NewStreamingBbands(period int, nbDevUp, nbDevDn float64) *StreamingBbands { h := C.ta_streaming_bbands_new(cInt(period), cDouble(nbDevUp), cDouble(nbDevDn)); if h == nil { return nil }; return &StreamingBbands{handle: unsafe.Pointer(h)} }
func (s *StreamingBbands) Update(value float64) (BbandsOutput, bool) { var upper, middle, lower C.double; ready := C.ta_streaming_bbands_update(s.handle, cDouble(value), &upper, &middle, &lower); if ready == 0 { return BbandsOutput{}, false }; return BbandsOutput{Upper: float64(upper), Middle: float64(middle), Lower: float64(lower)}, true }
func (s *StreamingBbands) Reset() { C.ta_streaming_bbands_reset(s.handle) }
func (s *StreamingBbands) Free() { if s.handle != nil { C.ta_streaming_bbands_free(s.handle); s.handle = nil } }
type StreamingAtr struct { handle unsafe.Pointer }
func NewStreamingAtr(period int) *StreamingAtr { h := C.ta_streaming_atr_new(cInt(period)); if h == nil { return nil }; return &StreamingAtr{handle: unsafe.Pointer(h)} }
func (s *StreamingAtr) Update(high, low, close float64) float64 { return float64(C.ta_streaming_atr_update_hlc(s.handle, cDouble(high), cDouble(low), cDouble(close))) }
func (s *StreamingAtr) Reset() { C.ta_streaming_atr_reset(s.handle) }
func (s *StreamingAtr) Free() { if s.handle != nil { C.ta_streaming_atr_free(s.handle); s.handle = nil } }
