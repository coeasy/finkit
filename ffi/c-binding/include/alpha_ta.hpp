#ifndef ALPHA_TA_HPP
#define ALPHA_TA_HPP

#include "alpha_ta.h"
#include <vector>
#include <stdexcept>
#include <string>
#include <memory>
#include <array>
#include <algorithm>
#include <functional>
#include <chrono>

namespace alphata {

class TaLibException : public std::runtime_error {
public:
    explicit TaLibException(const std::string& msg, int32_t code = 0)
        : std::runtime_error(msg), error_code_(code) {}
    int32_t code() const noexcept { return error_code_; }
private:
    int32_t error_code_;
};

inline std::string version() {
    return std::string(ta_version());
}

namespace detail {

inline void check_result(int32_t result, const std::string& func_name) {
    if (result == 0) return;
    std::string msg = func_name + " failed: ";
    switch (result) {
        case -1: msg += "invalid input"; break;
        case -2: msg += "calculation error"; break;
        default: msg += "unknown error (code: " + std::to_string(result) + ")"; break;
    }
    throw TaLibException(msg, result);
}

inline std::vector<double> allocate_output(int32_t len) {
    return std::vector<double>(static_cast<size_t>(len), 0.0);
}

inline std::vector<int32_t> allocate_int_output(int32_t len) {
    return std::vector<int32_t>(static_cast<size_t>(len), 0);
}

inline std::vector<double> copy_input(const std::vector<double>& input) {
    return input;
}

template<typename Func>
inline std::vector<double> single_output_func(
    const std::vector<double>& input,
    int32_t period,
    Func&& func,
    const std::string& name)
{
    auto input_copy = copy_input(input);
    auto output = allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = func(input_copy.data(), output.data(), static_cast<int32_t>(input.size()), period);
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> single_output_func(
    const std::vector<double>& input,
    double param,
    Func&& func,
    const std::string& name)
{
    auto input_copy = copy_input(input);
    auto output = allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = func(input_copy.data(), output.data(), static_cast<int32_t>(input.size()), param);
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> single_output_func_no_param(
    const std::vector<double>& input,
    Func&& func,
    const std::string& name)
{
    auto input_copy = copy_input(input);
    auto output = allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = func(input_copy.data(), output.data(), static_cast<int32_t>(input.size()));
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> ohl_single_output_func(
    const std::vector<double>& high,
    const std::vector<double>& low,
    int32_t period,
    Func&& func,
    const std::string& name)
{
    auto high_copy = copy_input(high);
    auto low_copy = copy_input(low);
    auto output = allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = func(high_copy.data(), low_copy.data(), output.data(), static_cast<int32_t>(high.size()), period);
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> ohl_single_output_func(
    const std::vector<double>& high,
    const std::vector<double>& low,
    double param1,
    double param2,
    Func&& func,
    const std::string& name)
{
    auto high_copy = copy_input(high);
    auto low_copy = copy_input(low);
    auto output = allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = func(high_copy.data(), low_copy.data(), output.data(), static_cast<int32_t>(high.size()), param1, param2);
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> ohl_single_output_func_no_param(
    const std::vector<double>& high,
    const std::vector<double>& low,
    Func&& func,
    const std::string& name)
{
    auto high_copy = copy_input(high);
    auto low_copy = copy_input(low);
    auto output = allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = func(high_copy.data(), low_copy.data(), output.data(), static_cast<int32_t>(high.size()));
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> ohlc_single_output_func(
    const std::vector<double>& high,
    const std::vector<double>& low,
    const std::vector<double>& close,
    int32_t period,
    Func&& func,
    const std::string& name)
{
    auto high_copy = copy_input(high);
    auto low_copy = copy_input(low);
    auto close_copy = copy_input(close);
    auto output = allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = func(high_copy.data(), low_copy.data(), close_copy.data(), output.data(), static_cast<int32_t>(high.size()), period);
    check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<double> ohlc_single_output_func_no_param(
    const std::vector<double>& high,
    const std::vector<double>& low,
    const std::vector<double>& close,
    Func&& func,
    const std::string& name)
{
    auto high_copy = copy_input(high);
    auto low_copy = copy_input(low);
    auto close_copy = copy_input(close);
    auto output = allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = func(high_copy.data(), low_copy.data(), close_copy.data(), output.data(), static_cast<int32_t>(high.size()));
    check_result(result, name);
    return output;
}

} // namespace detail

// ============================================================
// Overlap Studies
// ============================================================

struct MacdResult {
    std::vector<double> macd;
    std::vector<double> signal;
    std::vector<double> hist;
};

struct BollingerResult {
    std::vector<double> upper;
    std::vector<double> middle;
    std::vector<double> lower;
};

struct StochResult {
    std::vector<double> k;
    std::vector<double> d;
};

struct AroonResult {
    std::vector<double> aroon_up;
    std::vector<double> aroon_down;
};

struct MamaResult {
    std::vector<double> mama;
    std::vector<double> fama;
};

struct SarResult {
    std::vector<double> sar;
};

struct HilbertPhasorResult {
    std::vector<double> in_phase;
    std::vector<double> quadrature;
};

struct HilbertSineResult {
    std::vector<double> sine;
    std::vector<double> lead_sine;
};

// Moving Averages
inline std::vector<double> sma(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_sma, "sma");
}

inline std::vector<double> ema(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_ema, "ema");
}

inline std::vector<double> wma(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_wma, "wma");
}

inline std::vector<double> dema(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_dema, "dema");
}

inline std::vector<double> tema(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_tema, "tema");
}

inline std::vector<double> kama(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_kama, "kama");
}

inline MamaResult mama(const std::vector<double>& input, double fast_limit = 0.5, double slow_limit = 0.05) {
    auto input_copy = detail::copy_input(input);
    auto mama_out = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto fama_out = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_mama(input_copy.data(), mama_out.data(), fama_out.data(),
                             static_cast<int32_t>(input.size()), fast_limit, slow_limit);
    detail::check_result(result, "mama");
    return {std::move(mama_out), std::move(fama_out)};
}

inline std::vector<double> t3(const std::vector<double>& input, int32_t period = 5, double vfactor = 0.7) {
    auto input_copy = detail::copy_input(input);
    auto output = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_t3(input_copy.data(), output.data(), static_cast<int32_t>(input.size()), period, vfactor);
    detail::check_result(result, "t3");
    return output;
}

inline BollingerResult bbands(const std::vector<double>& input, int32_t period = 5,
                               double nbdevup = 2.0, double nbdevdn = 2.0) {
    auto input_copy = detail::copy_input(input);
    auto upper = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto middle = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto lower = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_bbands(input_copy.data(), upper.data(), middle.data(), lower.data(),
                               static_cast<int32_t>(input.size()), period, nbdevup, nbdevdn);
    detail::check_result(result, "bbands");
    return {std::move(upper), std::move(middle), std::move(lower)};
}

inline std::vector<double> midpoint(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_midpoint, "midpoint");
}

inline std::vector<double> midprice(const std::vector<double>& high, const std::vector<double>& low, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, period, ta_midprice, "midprice");
}

inline SarResult sar(const std::vector<double>& high, const std::vector<double>& low,
                     double acceleration = 0.02, double maximum = 0.2) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto output = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_sar(high_copy.data(), low_copy.data(), output.data(),
                            static_cast<int32_t>(high.size()), acceleration, maximum);
    detail::check_result(result, "sar");
    return {std::move(output)};
}

// ============================================================
// Momentum Indicators
// ============================================================

inline std::vector<double> rsi(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_rsi, "rsi");
}

inline MacdResult macd(const std::vector<double>& input, int32_t fast_period = 12,
                       int32_t slow_period = 26, int32_t signal_period = 9) {
    auto input_copy = detail::copy_input(input);
    auto macd_out = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto signal_out = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto hist_out = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_macd(input_copy.data(), macd_out.data(), signal_out.data(), hist_out.data(),
                             static_cast<int32_t>(input.size()), fast_period, slow_period, signal_period);
    detail::check_result(result, "macd");
    return {std::move(macd_out), std::move(signal_out), std::move(hist_out)};
}

inline StochResult stoch(const std::vector<double>& high, const std::vector<double>& low,
                         const std::vector<double>& close,
                         int32_t fastk_period = 5, int32_t slowk_period = 3, int32_t slowd_period = 3) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto k_out = detail::allocate_output(static_cast<int32_t>(high.size()));
    auto d_out = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_stoch(high_copy.data(), low_copy.data(), close_copy.data(),
                              k_out.data(), d_out.data(),
                              static_cast<int32_t>(high.size()),
                              fastk_period, slowk_period, slowd_period);
    detail::check_result(result, "stoch");
    return {std::move(k_out), std::move(d_out)};
}

inline std::vector<double> adx(const std::vector<double>& high, const std::vector<double>& low,
                               const std::vector<double>& close, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, close, period, ta_adx, "adx");
}

inline AroonResult aroon(const std::vector<double>& high, const std::vector<double>& low, int32_t period = 14) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto up_out = detail::allocate_output(static_cast<int32_t>(high.size()));
    auto down_out = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_aroon(high_copy.data(), low_copy.data(), up_out.data(), down_out.data(),
                              static_cast<int32_t>(high.size()), period);
    detail::check_result(result, "aroon");
    return {std::move(up_out), std::move(down_out)};
}

inline std::vector<double> cci(const std::vector<double>& high, const std::vector<double>& low,
                               const std::vector<double>& close, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, close, period, ta_cci, "cci");
}

inline std::vector<double> mom(const std::vector<double>& input, int32_t period = 10) {
    return detail::single_output_func(input, period, ta_mom, "mom");
}

inline std::vector<double> roc(const std::vector<double>& input, int32_t period = 10) {
    return detail::single_output_func(input, period, ta_roc, "roc");
}

inline std::vector<double> willr(const std::vector<double>& high, const std::vector<double>& low,
                                 const std::vector<double>& close, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, close, period, ta_willr, "willr");
}

inline std::vector<double> apo(const std::vector<double>& input, int32_t fast_period = 12, int32_t slow_period = 26) {
    auto input_copy = detail::copy_input(input);
    auto output = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_apo(input_copy.data(), output.data(), static_cast<int32_t>(input.size()), fast_period, slow_period);
    detail::check_result(result, "apo");
    return output;
}

inline std::vector<double> bop(const std::vector<double>& open, const std::vector<double>& high,
                               const std::vector<double>& low, const std::vector<double>& close) {
    auto open_copy = detail::copy_input(open);
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto output = detail::allocate_output(static_cast<int32_t>(open.size()));
    int32_t result = ta_bop(open_copy.data(), high_copy.data(), low_copy.data(), close_copy.data(),
                            output.data(), static_cast<int32_t>(open.size()));
    detail::check_result(result, "bop");
    return output;
}

inline std::vector<double> cmo(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_cmo, "cmo");
}

inline std::vector<double> mfi(const std::vector<double>& high, const std::vector<double>& low,
                               const std::vector<double>& close, const std::vector<double>& volume,
                               int32_t period = 14) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto volume_copy = detail::copy_input(volume);
    auto output = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_mfi(high_copy.data(), low_copy.data(), close_copy.data(), volume_copy.data(),
                            output.data(), static_cast<int32_t>(high.size()), period);
    detail::check_result(result, "mfi");
    return output;
}

inline std::vector<double> trix(const std::vector<double>& input, int32_t period = 30) {
    return detail::single_output_func(input, period, ta_trix, "trix");
}

// ============================================================
// Volatility Indicators
// ============================================================

inline std::vector<double> atr(const std::vector<double>& high, const std::vector<double>& low,
                               const std::vector<double>& close, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, close, period, ta_atr, "atr");
}

inline std::vector<double> natr(const std::vector<double>& high, const std::vector<double>& low,
                                const std::vector<double>& close, int32_t period = 14) {
    return detail::ohl_single_output_func(high, low, close, period, ta_natr, "natr");
}

inline std::vector<double> trange(const std::vector<double>& high, const std::vector<double>& low,
                                  const std::vector<double>& close) {
    return detail::ohl_single_output_func_no_param(high, low, close, ta_trange, "trange");
}

// ============================================================
// Volume Indicators
// ============================================================

inline std::vector<double> obv(const std::vector<double>& close, const std::vector<double>& volume) {
    auto close_copy = detail::copy_input(close);
    auto volume_copy = detail::copy_input(volume);
    auto output = detail::allocate_output(static_cast<int32_t>(close.size()));
    int32_t result = ta_obv(close_copy.data(), volume_copy.data(), output.data(),
                            static_cast<int32_t>(close.size()));
    detail::check_result(result, "obv");
    return output;
}

inline std::vector<double> ad(const std::vector<double>& high, const std::vector<double>& low,
                              const std::vector<double>& close, const std::vector<double>& volume) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto volume_copy = detail::copy_input(volume);
    auto output = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_ad(high_copy.data(), low_copy.data(), close_copy.data(), volume_copy.data(),
                           output.data(), static_cast<int32_t>(high.size()));
    detail::check_result(result, "ad");
    return output;
}

inline std::vector<double> adosc(const std::vector<double>& high, const std::vector<double>& low,
                                 const std::vector<double>& close, const std::vector<double>& volume,
                                 int32_t fast_period = 3, int32_t slow_period = 10) {
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto volume_copy = detail::copy_input(volume);
    auto output = detail::allocate_output(static_cast<int32_t>(high.size()));
    int32_t result = ta_adosc(high_copy.data(), low_copy.data(), close_copy.data(), volume_copy.data(),
                              output.data(), static_cast<int32_t>(high.size()),
                              fast_period, slow_period);
    detail::check_result(result, "adosc");
    return output;
}

// ============================================================
// Hilbert Transform Indicators
// ============================================================

inline std::vector<double> ht_dcperiod(const std::vector<double>& input) {
    return detail::single_output_func_no_param(input, ta_ht_dcperiod, "ht_dcperiod");
}

inline std::vector<double> ht_dcphase(const std::vector<double>& input) {
    return detail::single_output_func_no_param(input, ta_ht_dcphase, "ht_dcphase");
}

inline HilbertPhasorResult ht_phasor(const std::vector<double>& input) {
    auto input_copy = detail::copy_input(input);
    auto in_phase = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto quadrature = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_ht_phasor(input_copy.data(), in_phase.data(), quadrature.data(),
                                  static_cast<int32_t>(input.size()));
    detail::check_result(result, "ht_phasor");
    return {std::move(in_phase), std::move(quadrature)};
}

inline HilbertSineResult ht_sine(const std::vector<double>& input) {
    auto input_copy = detail::copy_input(input);
    auto sine = detail::allocate_output(static_cast<int32_t>(input.size()));
    auto lead_sine = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_ht_sine(input_copy.data(), sine.data(), lead_sine.data(),
                                static_cast<int32_t>(input.size()));
    detail::check_result(result, "ht_sine");
    return {std::move(sine), std::move(lead_sine)};
}

inline std::vector<double> ht_trendmode(const std::vector<double>& input) {
    return detail::single_output_func_no_param(input, ta_ht_trendmode, "ht_trendmode");
}

inline std::vector<double> ht_trendline(const std::vector<double>& input) {
    return detail::single_output_func_no_param(input, ta_ht_trendline, "ht_trendline");
}

// ============================================================
// Statistics Functions
// ============================================================

inline std::vector<double> zscore(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_zscore, "zscore");
}

inline std::vector<double> beta(const std::vector<double>& asset, const std::vector<double>& benchmark, int32_t period = 5) {
    auto asset_copy = detail::copy_input(asset);
    auto benchmark_copy = detail::copy_input(benchmark);
    auto output = detail::allocate_output(static_cast<int32_t>(asset.size()));
    int32_t result = ta_beta(asset_copy.data(), benchmark_copy.data(), output.data(),
                             static_cast<int32_t>(asset.size()), period);
    detail::check_result(result, "beta");
    return output;
}

inline std::vector<double> correlation(const std::vector<double>& input_a, const std::vector<double>& input_b, int32_t period = 30) {
    auto a_copy = detail::copy_input(input_a);
    auto b_copy = detail::copy_input(input_b);
    auto output = detail::allocate_output(static_cast<int32_t>(input_a.size()));
    int32_t result = ta_correlation(a_copy.data(), b_copy.data(), output.data(),
                                    static_cast<int32_t>(input_a.size()), period);
    detail::check_result(result, "correlation");
    return output;
}

inline std::vector<double> stddev(const std::vector<double>& input, int32_t period = 5, double nb_dev = 1.0) {
    auto input_copy = detail::copy_input(input);
    auto output = detail::allocate_output(static_cast<int32_t>(input.size()));
    int32_t result = ta_stddev(input_copy.data(), output.data(), static_cast<int32_t>(input.size()), period, nb_dev);
    detail::check_result(result, "stddev");
    return output;
}

inline std::vector<double> tsf(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_tsf, "tsf");
}

inline std::vector<double> linear_reg(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_linear_reg, "linear_reg");
}

inline std::vector<double> percent_rank(const std::vector<double>& input, int32_t period = 14) {
    return detail::single_output_func(input, period, ta_percent_rank, "percent_rank");
}

// ============================================================
// Price Transform Functions
// ============================================================

inline std::vector<double> avgprice(const std::vector<double>& open, const std::vector<double>& high,
                                    const std::vector<double>& low, const std::vector<double>& close) {
    auto open_copy = detail::copy_input(open);
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto output = detail::allocate_output(static_cast<int32_t>(open.size()));
    int32_t result = ta_avgprice(open_copy.data(), high_copy.data(), low_copy.data(), close_copy.data(),
                                 output.data(), static_cast<int32_t>(open.size()));
    detail::check_result(result, "avgprice");
    return output;
}

inline std::vector<double> medprice(const std::vector<double>& high, const std::vector<double>& low) {
    return detail::ohl_single_output_func_no_param(high, low, ta_medprice, "medprice");
}

inline std::vector<double> typprice(const std::vector<double>& high, const std::vector<double>& low,
                                    const std::vector<double>& close) {
    return detail::ohl_single_output_func_no_param(high, low, close, ta_typprice, "typprice");
}

inline std::vector<double> wclprice(const std::vector<double>& high, const std::vector<double>& low,
                                    const std::vector<double>& close) {
    return detail::ohl_single_output_func_no_param(high, low, close, ta_wclprice, "wclprice");
}

// ============================================================
// Candlestick Pattern Functions
// ============================================================

namespace detail {

template<typename Func>
inline std::vector<int32_t> candle_pattern_4arg(
    const std::vector<double>& open,
    const std::vector<double>& high,
    const std::vector<double>& low,
    const std::vector<double>& close,
    double param,
    Func&& func,
    const std::string& name)
{
    auto open_copy = detail::copy_input(open);
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto output = detail::allocate_int_output(static_cast<int32_t>(open.size()));
    int32_t result = func(open_copy.data(), high_copy.data(), low_copy.data(), close_copy.data(),
                          output.data(), static_cast<int32_t>(open.size()), param);
    detail::check_result(result, name);
    return output;
}

template<typename Func>
inline std::vector<int32_t> candle_pattern_4arg_no_param(
    const std::vector<double>& open,
    const std::vector<double>& high,
    const std::vector<double>& low,
    const std::vector<double>& close,
    Func&& func,
    const std::string& name)
{
    auto open_copy = detail::copy_input(open);
    auto high_copy = detail::copy_input(high);
    auto low_copy = detail::copy_input(low);
    auto close_copy = detail::copy_input(close);
    auto output = detail::allocate_int_output(static_cast<int32_t>(open.size()));
    int32_t result = func(open_copy.data(), high_copy.data(), low_copy.data(), close_copy.data(),
                          output.data(), static_cast<int32_t>(open.size()));
    detail::check_result(result, name);
    return output;
}

} // namespace detail

inline std::vector<int32_t> cdl_doji(const std::vector<double>& open, const std::vector<double>& high,
                                     const std::vector<double>& low, const std::vector<double>& close,
                                     double doji_pct = 0.1) {
    return detail::candle_pattern_4arg(open, high, low, close, doji_pct, ta_cdl_doji, "cdl_doji");
}

inline std::vector<int32_t> cdl_dragonfly_doji(const std::vector<double>& open, const std::vector<double>& high,
                                                const std::vector<double>& low, const std::vector<double>& close,
                                                double doji_pct = 0.1) {
    return detail::candle_pattern_4arg(open, high, low, close, doji_pct, ta_cdl_dragonfly_doji, "cdl_dragonfly_doji");
}

inline std::vector<int32_t> cdl_gravestone_doji(const std::vector<double>& open, const std::vector<double>& high,
                                                 const std::vector<double>& low, const std::vector<double>& close,
                                                 double doji_pct = 0.1) {
    return detail::candle_pattern_4arg(open, high, low, close, doji_pct, ta_cdl_gravestone_doji, "cdl_gravestone_doji");
}

inline std::vector<int32_t> cdl_long_legged_doji(const std::vector<double>& open, const std::vector<double>& high,
                                                  const std::vector<double>& low, const std::vector<double>& close,
                                                  double doji_pct = 0.1) {
    return detail::candle_pattern_4arg(open, high, low, close, doji_pct, ta_cdl_long_legged_doji, "cdl_long_legged_doji");
}

inline std::vector<int32_t> cdl_hammer(const std::vector<double>& open, const std::vector<double>& high,
                                       const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_hammer, "cdl_hammer");
}

inline std::vector<int32_t> cdl_inverted_hammer(const std::vector<double>& open, const std::vector<double>& high,
                                                 const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_inverted_hammer, "cdl_inverted_hammer");
}

inline std::vector<int32_t> cdl_hanging_man(const std::vector<double>& open, const std::vector<double>& high,
                                            const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_hanging_man, "cdl_hanging_man");
}

inline std::vector<int32_t> cdl_shooting_star(const std::vector<double>& open, const std::vector<double>& high,
                                              const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_shooting_star, "cdl_shooting_star");
}

inline std::vector<int32_t> cdl_engulfing(const std::vector<double>& open, const std::vector<double>& high,
                                          const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_engulfing, "cdl_engulfing");
}

inline std::vector<int32_t> cdl_harami(const std::vector<double>& open, const std::vector<double>& high,
                                       const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_harami, "cdl_harami");
}

inline std::vector<int32_t> cdl_morning_star(const std::vector<double>& open, const std::vector<double>& high,
                                             const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_morning_star, "cdl_morning_star");
}

inline std::vector<int32_t> cdl_evening_star(const std::vector<double>& open, const std::vector<double>& high,
                                             const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_evening_star, "cdl_evening_star");
}

inline std::vector<int32_t> cdl_three_white_soldiers(const std::vector<double>& open, const std::vector<double>& high,
                                                     const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_three_white_soldiers, "cdl_three_white_soldiers");
}

inline std::vector<int32_t> cdl_three_black_crows(const std::vector<double>& open, const std::vector<double>& high,
                                                  const std::vector<double>& low, const std::vector<double>& close) {
    return detail::candle_pattern_4arg_no_param(open, high, low, close, ta_cdl_three_black_crows, "cdl_three_black_crows");
}

inline std::vector<int32_t> cdl_marubozu(const std::vector<double>& open, const std::vector<double>& high,
                                         const std::vector<double>& low, const std::vector<double>& close,
                                         double shadow_pct = 0.05) {
    return detail::candle_pattern_4arg(open, high, low, close, shadow_pct, ta_cdl_marubozu, "cdl_marubozu");
}

// ============================================================
// Benchmark Utilities
// ============================================================

struct BenchmarkResult {
    std::string name;
    size_t iterations;
    double total_ms;
    double avg_ms;
    double ops_per_sec;
};

template<typename Func, typename... Args>
inline BenchmarkResult benchmark(const std::string& name, size_t iterations, Func&& func, Args&&... args) {
    auto start = std::chrono::high_resolution_clock::now();
    for (size_t i = 0; i < iterations; ++i) {
        func(std::forward<Args>(args)...);
    }
    auto end = std::chrono::high_resolution_clock::now();
    double total_ms = std::chrono::duration<double, std::milli>(end - start).count();
    double avg_ms = total_ms / iterations;
    double ops_per_sec = (iterations * 1000.0) / total_ms;
    return {name, iterations, total_ms, avg_ms, ops_per_sec};
}

} // namespace alphata

#endif /* ALPHA_TA_HPP */
