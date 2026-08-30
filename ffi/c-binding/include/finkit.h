#ifndef FINKIT_H
#define FINKIT_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

#ifdef _WIN32
  #ifdef FINKIT_EXPORTS
    #define TA_API __declspec(dllexport)
  #else
    #define TA_API __declspec(dllimport)
  #endif
#else
  #define TA_API __attribute__((visibility("default")))
#endif

typedef int32_t ta_result_t;

/**
 * Stable ABI error classification (`#[repr(i32)]` in lib.rs).
 * Detailed codes are available via ta_last_error_code().
 */
typedef enum FfiStatus {
    FfiStatus_Ok = 0,
    FfiStatus_NullPointer = -1,
    FfiStatus_InvalidParameter = -2,
    FfiStatus_InsufficientData = -3,
    FfiStatus_InternalError = -4,
    FfiStatus_InvalidUtf8 = -5,
    FfiStatus_Unknown = -99
} FfiStatus;

/* ── Version & error reporting ─────────────────────────────────────────── */

TA_API char *ta_version(void);
TA_API char *ta_last_error(void);
TA_API int32_t ta_last_error_code(void);
TA_API void finkit_free_string(char *s);


/* ── Moving averages & overlays ─────────────────────────────────────────── */
TA_API ta_result_t ta_sma(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_ema(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_wma(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_dema(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_tema(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_kama(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_mama(const double *input, double *mama_out, double *fama_out, int32_t len, double fast_limit, double slow_limit);
TA_API ta_result_t ta_t3(const double *input, double *output, int32_t len, int32_t period, double vfactor);
TA_API ta_result_t ta_bbands(const double *input, double *upper, double *middle, double *lower, int32_t len, int32_t period, double nbdevup, double nbdevdn);
TA_API ta_result_t ta_midpoint(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_midprice(const double *high, const double *low, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_sar(const double *high, const double *low, double *output, int32_t len, double acceleration, double maximum);

/* ── Momentum & oscillators ─────────────────────────────────────────────── */
TA_API ta_result_t ta_rsi(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_macd(const double *input, double *macd_out, double *signal_out, double *hist_out, int32_t len, int32_t fast_period, int32_t slow_period, int32_t signal_period);
TA_API ta_result_t ta_stoch(const double *high, const double *low, const double *close, double *slowk, double *slowd, int32_t len, int32_t fastk_period, int32_t slowk_period, int32_t slowd_period);
TA_API ta_result_t ta_adx(const double *high, const double *low, const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_aroon(const double *high, const double *low, double *aroon_up, double *aroon_down, int32_t len, int32_t period);
TA_API ta_result_t ta_cci(const double *high, const double *low, const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_mom(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_roc(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_willr(const double *high, const double *low, const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_apo(const double *input, double *output, int32_t len, int32_t fast_period, int32_t slow_period);
TA_API ta_result_t ta_bop(const double *open, const double *high, const double *low, const double *close, double *output, int32_t len);
TA_API ta_result_t ta_cmo(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_mfi(const double *high, const double *low, const double *close, const double *volume, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_trix(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_vortex(const double *high, const double *low, const double *close, double *vi_plus, double *vi_minus, int32_t len, int32_t period);
TA_API ta_result_t ta_vzo(const double *close, const double *volume, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_volume_momentum(const double *volume, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_volume_roc(const double *volume, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_chande_forecast(const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_twiggs_mf(const double *high, const double *low, const double *close, const double *volume, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_inertia(const double *open, const double *high, const double *low, const double *close, double *output, int32_t len, int32_t rvi_period, int32_t linreg_period);

/* ── Volatility & volume ────────────────────────────────────────────────── */
TA_API ta_result_t ta_atr(const double *high, const double *low, const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_natr(const double *high, const double *low, const double *close, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_trange(const double *high, const double *low, const double *close, double *output, int32_t len);
TA_API ta_result_t ta_obv(const double *close, const double *volume, double *output, int32_t len);
TA_API ta_result_t ta_ad(const double *high, const double *low, const double *close, const double *volume, double *output, int32_t len);
TA_API ta_result_t ta_adosc(const double *high, const double *low, const double *close, const double *volume, double *output, int32_t len, int32_t fast_period, int32_t slow_period);

/* ── Hilbert transform ──────────────────────────────────────────────────── */
TA_API ta_result_t ta_ht_dcperiod(const double *input, double *output, int32_t len);
TA_API ta_result_t ta_ht_dcphase(const double *input, double *output, int32_t len);
TA_API ta_result_t ta_ht_phasor(const double *input, double *in_phase, double *quadrature, int32_t len);
TA_API ta_result_t ta_ht_sine(const double *input, double *sine, double *lead_sine, int32_t len);
TA_API ta_result_t ta_ht_trendmode(const double *input, double *output, int32_t len);
TA_API ta_result_t ta_ht_trendline(const double *input, double *output, int32_t len);

/* ── Statistics & price transforms ──────────────────────────────────────── */
TA_API ta_result_t ta_zscore(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_beta(const double *asset, const double *benchmark, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_correlation(const double *input_a, const double *input_b, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_stddev(const double *input, double *output, int32_t len, int32_t period, double nb_dev);
TA_API ta_result_t ta_tsf(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_linear_reg(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_percent_rank(const double *input, double *output, int32_t len, int32_t period);
TA_API ta_result_t ta_avgprice(const double *open, const double *high, const double *low, const double *close, double *output, int32_t len);
TA_API ta_result_t ta_medprice(const double *high, const double *low, double *output, int32_t len);
TA_API ta_result_t ta_typprice(const double *high, const double *low, const double *close, double *output, int32_t len);
TA_API ta_result_t ta_wclprice(const double *high, const double *low, const double *close, double *output, int32_t len);

/* ── Candlestick patterns ───────────────────────────────────────────────── */
TA_API ta_result_t ta_cdl_doji(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len, double doji_pct);
TA_API ta_result_t ta_cdl_dragonfly_doji(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len, double doji_pct);
TA_API ta_result_t ta_cdl_gravestone_doji(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len, double doji_pct);
TA_API ta_result_t ta_cdl_long_legged_doji(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len, double doji_pct);
TA_API ta_result_t ta_cdl_hammer(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_inverted_hammer(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_hanging_man(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_shooting_star(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_engulfing(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_harami(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_morning_star(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_evening_star(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_three_white_soldiers(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_three_black_crows(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len);
TA_API ta_result_t ta_cdl_marubozu(const double *open, const double *high, const double *low, const double *close, int32_t *output, int32_t len, double shadow_pct);

/* ── Chart patterns (FTA-native) ────────────────────────────────────────── */
TA_API ta_result_t ta_darvas_box(const double *high, const double *low, const double *close, double *out_top, double *out_bottom, int32_t *out_signal, int32_t len, int32_t lookback, int32_t confirmation);
TA_API ta_result_t ta_renko(const double *high, const double *low, double *out_bricks, int32_t *out_dir, int32_t len, double box_size);
TA_API ta_result_t ta_kagi(const double *close, double *out_kagi, int32_t *out_dir, int32_t len, double reversal);
TA_API ta_result_t ta_point_and_figure(const double *high, const double *low, double *out_pnf, int32_t *out_col, int32_t *out_new, int32_t len, double box_size, int32_t reversal);
TA_API ta_result_t ta_three_line_break(const double *close, double *out_line, int32_t *out_dir, int32_t len, int32_t lines);
TA_API ta_result_t ta_williams_alligator(const double *close, double *out_jaw, double *out_teeth, double *out_lips, int32_t len);
TA_API ta_result_t ta_heikin_ashi(const double *open, const double *high, const double *low, const double *close, double *out_o, double *out_h, double *out_l, double *out_c, int32_t len);

/* ── K-line visualization ────────────────────────────────────────────────── */

typedef int64_t finkit_kline_data_t;
typedef int64_t finkit_kline_chart_t;

TA_API finkit_kline_data_t finkit_kline_data_new(
    const char * const *dates,
    const double *opens,
    const double *highs,
    const double *lows,
    const double *closes,
    const double *volumes,
    int32_t len);
TA_API void finkit_kline_data_free(finkit_kline_data_t handle);
TA_API int32_t finkit_kline_data_validate(finkit_kline_data_t handle);

TA_API finkit_kline_chart_t finkit_kline_chart_new(
    finkit_kline_data_t data_handle,
    const char *language,
    const char *title,
    uint32_t width,
    uint32_t height);
TA_API void finkit_kline_chart_free(finkit_kline_chart_t handle);

TA_API int32_t finkit_kline_chart_add_ma(
    finkit_kline_chart_t handle,
    const int32_t *periods,
    int32_t periods_len);
TA_API int32_t finkit_kline_chart_add_macd(
    finkit_kline_chart_t handle,
    int32_t fast,
    int32_t slow,
    int32_t signal);
TA_API int32_t finkit_kline_chart_add_rsi(
    finkit_kline_chart_t handle,
    int32_t period);
TA_API int32_t finkit_kline_chart_add_boll(
    finkit_kline_chart_t handle,
    int32_t period,
    double nb_dev);

TA_API int32_t finkit_kline_chart_save_as_svg(
    finkit_kline_chart_t handle,
    const char *path);
TA_API char *finkit_kline_chart_to_svg(finkit_kline_chart_t handle);

#ifdef __cplusplus
}
#endif

#endif /* FINKIT_H */
