#ifndef FINKIT_IOS_H
#define FINKIT_IOS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Finkit iOS C ABI.
 *
 * The alpha_ta_* symbol prefix is retained for binary/source compatibility
 * with the first iOS binding implementation. New Swift-facing APIs use the
 * Finkit name; changing these C symbols is a separate ABI-breaking decision.
 */

/// ABI version of the bundled static library. Bumped whenever the symbol set
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
/// Returns the total number of non-zero detections across the built-in
/// Doji, Hammer, and Engulfing detectors. Returns a negative value on error.
int32_t alpha_ta_detect_candlestick(const double *open, const double *high,
                                    const double *low, const double *close,
                                    int32_t len);

// ---- research / quant evaluation (JSON contracts) --------------------------
//
// Both functions return a newly allocated NUL-terminated JSON document, or
// `NULL` only when allocation fails; an invalid request is reported as an
// error document, never as `NULL`. Release the result with
// `finkit_ios_factor_study_free_string` — one free function serves both
// entry points because both allocate the same way. Passing `NULL` to the free
// function is a no-op.
//
// `alpha_ta_ffi_panic_test` is deliberately absent: it is compiled only under
// `cfg(test)` and is not part of the shipped symbol set.

/// Runs the factor-research JSON contract.
char *finkit_ios_factor_study_json(const char *request_json);

/// Runs the quantitative-evaluation JSON contract.
char *finkit_ios_quant_evaluation_json(const char *request_json);

/// Frees a string returned by either of the two functions above.
void finkit_ios_factor_study_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif
