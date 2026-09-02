#ifndef ALPHA_TA_IOS_H
#define ALPHA_TA_IOS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// ABI version of the bundled static lib. Bumped whenever the symbol set
/// changes in a backwards-incompatible way.
int32_t alpha_ta_ios_abi_version(void);

// ---- moving averages -------------------------------------------------------
int32_t alpha_ta_sma(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_ema(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_wma(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_dema(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_tema(const double *input, int32_t len, int32_t period, double *out);

// ---- momentum --------------------------------------------------------------
int32_t alpha_ta_rsi(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_roc(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_mom(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_cmo(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_trix(const double *input, int32_t len, int32_t period, double *out);

// ---- statistics ------------------------------------------------------------
int32_t alpha_ta_midpoint(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_zscore(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_tsf(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_linear_reg(const double *input, int32_t len, int32_t period, double *out);
int32_t alpha_ta_percent_rank(const double *input, int32_t len, int32_t period, double *out);

// ---- candlestick patterns --------------------------------------------------
/// Returns the number of candlestick patterns detected in the supplied OHLC
/// series. The returned value is the total number of non-zero detections
/// across the built-in Doji, Hammer, and Engulfing detectors.
int32_t alpha_ta_detect_candlestick(const double *open, const double *high,
                                  const double *low,  const double *close,
                                  int32_t len);

#ifdef __cplusplus
}
#endif

#endif
