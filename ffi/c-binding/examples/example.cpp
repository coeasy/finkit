// Example: Using finkit in C++
// This example demonstrates all indicators with practical usage patterns

#include <iostream>
#include <vector>
#include <iomanip>
#include <string>
#include <algorithm>
#include <numeric>
#include <chrono>
#include <cmath>
#include <sstream>
#include "finkit.hpp"

using namespace finkit;

// Helper to print a few values from a vector
template<typename T>
void print_sample(const std::vector<T>& data, const std::string& name, size_t start = 0, size_t count = 5) {
    std::cout << std::left << std::setw(20) << name << ": ";
    for (size_t i = start; i < std::min(start + count, data.size()); ++i) {
        std::cout << std::fixed << std::setprecision(4) << data[i] << " ";
    }
    std::cout << std::endl;
}

// Generate sample OHLCV data
struct MarketData {
    std::vector<double> open;
    std::vector<double> high;
    std::vector<double> low;
    std::vector<double> close;
    std::vector<double> volume;
    
    MarketData(size_t n = 200) {
        open.resize(n);
        high.resize(n);
        low.resize(n);
        close.resize(n);
        volume.resize(n);
        
        double price = 100.0;
        for (size_t i = 0; i < n; ++i) {
            double change = (i % 20 < 10) ? 0.5 : -0.3;
            price += change;
            open[i] = price;
            high[i] = price + 1.5 + (i % 3) * 0.5;
            low[i] = price - 1.5 - (i % 2) * 0.5;
            close[i] = price + (i % 5 - 2) * 0.3;
            volume[i] = 1000000.0 + (i % 10) * 100000.0;
        }
    }
};

// Print separator
void print_separator(const std::string& title) {
    std::cout << std::endl;
    std::cout << "============================================================" << std::endl;
    std::cout << "  " << title << std::endl;
    std::cout << "============================================================" << std::endl;
    std::cout << std::endl;
}

// Example 1: Basic moving averages
void example_moving_averages(const MarketData& data) {
    print_separator("Moving Averages");
    
    auto sma_result = sma(data.close, 20);
    auto ema_result = ema(data.close, 20);
    auto wma_result = wma(data.close, 20);
    auto dema_result = dema(data.close, 20);
    auto tema_result = tema(data.close, 20);
    auto kama_result = kama(data.close, 20);
    auto t3_result = t3(data.close, 5, 0.7);
    auto mama_result = mama(data.close, 0.5, 0.05);
    
    std::cout << "Sample values (indices 180-184):" << std::endl;
    print_sample(sma_result, "SMA(20)", 180);
    print_sample(ema_result, "EMA(20)", 180);
    print_sample(wma_result, "WMA(20)", 180);
    print_sample(dema_result, "DEMA(20)", 180);
    print_sample(tema_result, "TEMA(20)", 180);
    print_sample(kama_result, "KAMA(20)", 180);
    print_sample(t3_result, "T3(5, 0.7)", 180);
    print_sample(mama_result.mama, "MAMA", 180);
    print_sample(mama_result.fama, "FAMA", 180);
}

// Example 2: Bollinger Bands
void example_bollinger_bands(const MarketData& data) {
    print_separator("Bollinger Bands");
    
    auto bb_result = bbands(data.close, 20, 2.0, 2.0);
    auto mid_result = midpoint(data.close, 20);
    auto midprice_result = midprice(data.high, data.low, 20);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(bb_result.upper, "Upper Band", 195);
    print_sample(bb_result.middle, "Middle Band", 195);
    print_sample(bb_result.lower, "Lower Band", 195);
    print_sample(mid_result, "Midpoint", 195);
    print_sample(midprice_result, "Midprice", 195);
}

// Example 3: MACD
void example_macd(const MarketData& data) {
    print_separator("MACD");
    
    auto macd_result = macd(data.close, 12, 26, 9);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(macd_result.macd, "MACD Line", 195);
    print_sample(macd_result.signal, "Signal Line", 195);
    print_sample(macd_result.hist, "Histogram", 195);
    
    // Trading signal: MACD crosses above signal = buy
    for (size_t i = 50; i < data.close.size() - 1; ++i) {
        if (macd_result.macd[i-1] < macd_result.signal[i-1] && macd_result.macd[i] > macd_result.signal[i]) {
            std::cout << "  [BUY Signal at index " << i << "]" << std::endl;
            break;
        }
        if (macd_result.macd[i-1] > macd_result.signal[i-1] && macd_result.macd[i] < macd_result.signal[i]) {
            std::cout << "  [SELL Signal at index " << i << "]" << std::endl;
            break;
        }
    }
}

// Example 4: RSI
void example_rsi(const MarketData& data) {
    print_separator("RSI (Relative Strength Index)");
    
    auto rsi_result = rsi(data.close, 14);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(rsi_result, "RSI(14)", 195);
    
    double last_rsi = rsi_result.back();
    if (last_rsi > 70.0) {
        std::cout << "  -> OVERBOUGHT (RSI > 70)" << std::endl;
    } else if (last_rsi < 30.0) {
        std::cout << "  -> OVERSOLD (RSI < 30)" << std::endl;
    } else {
        std::cout << "  -> NORMAL" << std::endl;
    }
}

// Example 5: Stochastic Oscillator
void example_stochastic(const MarketData& data) {
    print_separator("Stochastic Oscillator");
    
    auto stoch_result = stoch(data.high, data.low, data.close, 5, 3, 3);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(stoch_result.k, "%K", 195);
    print_sample(stoch_result.d, "%D", 195);
}

// Example 6: ADX and Aroon
void example_trend_indicators(const MarketData& data) {
    print_separator("Trend Indicators (ADX, Aroon)");
    
    auto adx_result = adx(data.high, data.low, data.close, 14);
    auto aroon_result = aroon(data.high, data.low, 14);
    auto cci_result = cci(data.high, data.low, data.close, 14);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(adx_result, "ADX(14)", 195);
    print_sample(aroon_result.aroon_up, "Aroon Up", 195);
    print_sample(aroon_result.aroon_down, "Aroon Down", 195);
    print_sample(cci_result, "CCI(14)", 195);
    
    double last_adx = adx_result.back();
    if (last_adx > 25.0) {
        std::cout << "  -> STRONG TREND (ADX > 25)" << std::endl;
    } else {
        std::cout << "  -> WEAK TREND (ADX < 25)" << std::endl;
    }
}

// Example 7: Momentum Indicators
void example_momentum(const MarketData& data) {
    print_separator("Momentum Indicators");
    
    auto mom_result = mom(data.close, 10);
    auto roc_result = roc(data.close, 10);
    auto willr_result = willr(data.high, data.low, data.close, 14);
    auto apo_result = apo(data.close, 12, 26);
    auto cmo_result = cmo(data.close, 14);
    auto mfi_result = mfi(data.high, data.low, data.close, data.volume, 14);
    auto trix_result = trix(data.close, 30);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(mom_result, "Momentum(10)", 195);
    print_sample(roc_result, "ROC(10)", 195);
    print_sample(willr_result, "Williams %R", 195);
    print_sample(apo_result, "APO(12,26)", 195);
    print_sample(cmo_result, "CMO(14)", 195);
    print_sample(mfi_result, "MFI(14)", 195);
    print_sample(trix_result, "TRIX(30)", 195);
}

// Example 8: Volatility Indicators
void example_volatility(const MarketData& data) {
    print_separator("Volatility Indicators");
    
    auto atr_result = atr(data.high, data.low, data.close, 14);
    auto natr_result = natr(data.high, data.low, data.close, 14);
    auto trange_result = trange(data.high, data.low, data.close);
    auto sar_result = sar(data.high, data.low, 0.02, 0.2);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(atr_result, "ATR(14)", 195);
    print_sample(natr_result, "NATR(14)", 195);
    print_sample(trange_result, "True Range", 195);
    print_sample(sar_result.sar, "Parabolic SAR", 195);
}

// Example 9: Volume Indicators
void example_volume(const MarketData& data) {
    print_separator("Volume Indicators");
    
    auto obv_result = obv(data.close, data.volume);
    auto ad_result = ad(data.high, data.low, data.close, data.volume);
    auto adosc_result = adosc(data.high, data.low, data.close, data.volume, 3, 10);
    auto bop_result = bop(data.open, data.high, data.low, data.close);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(obv_result, "OBV", 195);
    print_sample(ad_result, "A/D Line", 195);
    print_sample(adosc_result, "A/D Osc(3,10)", 195);
    print_sample(bop_result, "BOP", 195);
}

// Example 10: Hilbert Transform Indicators
void example_hilbert(const MarketData& data) {
    print_separator("Hilbert Transform Indicators");
    
    auto ht_dcperiod_result = ht_dcperiod(data.close);
    auto ht_dcphase_result = ht_dcphase(data.close);
    auto ht_phasor_result = ht_phasor(data.close);
    auto ht_sine_result = ht_sine(data.close);
    auto ht_trendmode_result = ht_trendmode(data.close);
    auto ht_trendline_result = ht_trendline(data.close);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(ht_dcperiod_result, "HT DCPeriod", 195);
    print_sample(ht_dcphase_result, "HT DCPhase", 195);
    print_sample(ht_phasor_result.in_phase, "HT In-Phase", 195);
    print_sample(ht_phasor_result.quadrature, "HT Quadrature", 195);
    print_sample(ht_sine_result.sine, "HT Sine", 195);
    print_sample(ht_sine_result.lead_sine, "HT Lead Sine", 195);
    print_sample(ht_trendmode_result, "HT TrendMode", 195);
    print_sample(ht_trendline_result, "HT Trendline", 195);
}

// Example 11: Statistics Functions
void example_statistics(const MarketData& data) {
    print_separator("Statistics Functions");
    
    auto zscore_result = zscore(data.close, 20);
    auto stddev_result = stddev(data.close, 20, 1.0);
    auto tsf_result = tsf(data.close, 14);
    auto linear_reg_result = linear_reg(data.close, 14);
    auto percent_rank_result = percent_rank(data.close, 14);
    
    // Generate benchmark data for correlation and beta
    std::vector<double> benchmark(data.close.size());
    for (size_t i = 0; i < data.close.size(); ++i) {
        benchmark[i] = data.close[i] * 1.1 + 5.0;
    }
    auto correlation_result = correlation(data.close, benchmark, 30);
    auto beta_result = beta(data.close, benchmark, 30);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(zscore_result, "Z-Score(20)", 195);
    print_sample(stddev_result, "StdDev(20)", 195);
    print_sample(tsf_result, "TSF(14)", 195);
    print_sample(linear_reg_result, "Linear Reg", 195);
    print_sample(percent_rank_result, "Percent Rank", 195);
    print_sample(correlation_result, "Correlation(30)", 195);
    print_sample(beta_result, "Beta(30)", 195);
}

// Example 12: Price Transform Functions
void example_price_transforms(const MarketData& data) {
    print_separator("Price Transform Functions");
    
    auto avgprice_result = avgprice(data.open, data.high, data.low, data.close);
    auto medprice_result = medprice(data.high, data.low);
    auto typprice_result = typprice(data.high, data.low, data.close);
    auto wclprice_result = wclprice(data.high, data.low, data.close);
    
    std::cout << "Last 5 values:" << std::endl;
    print_sample(avgprice_result, "AvgPrice", 195);
    print_sample(medprice_result, "MedPrice", 195);
    print_sample(typprice_result, "TypPrice", 195);
    print_sample(wclprice_result, "WCLPrice", 195);
}

// Example 13: Candlestick Patterns
void example_candlestick_patterns(const MarketData& data) {
    print_separator("Candlestick Pattern Detection");
    
    auto doji = cdl_doji(data.open, data.high, data.low, data.close, 0.1);
    auto hammer = cdl_hammer(data.open, data.high, data.low, data.close);
    auto engulfing = cdl_engulfing(data.open, data.high, data.low, data.close);
    auto morning_star = cdl_morning_star(data.open, data.high, data.low, data.close);
    auto evening_star = cdl_evening_star(data.open, data.high, data.low, data.close);
    auto dragonfly = cdl_dragonfly_doji(data.open, data.high, data.low, data.close, 0.1);
    auto gravestone = cdl_gravestone_doji(data.open, data.high, data.low, data.close, 0.1);
    auto long_legged = cdl_long_legged_doji(data.open, data.high, data.low, data.close, 0.1);
    auto inverted_hammer = cdl_inverted_hammer(data.open, data.high, data.low, data.close);
    auto hanging_man = cdl_hanging_man(data.open, data.high, data.low, data.close);
    auto shooting_star = cdl_shooting_star(data.open, data.high, data.low, data.close);
    auto harami = cdl_harami(data.open, data.high, data.low, data.close);
    auto three_white = cdl_three_white_soldiers(data.open, data.high, data.low, data.close);
    auto three_black = cdl_three_black_crows(data.open, data.high, data.low, data.close);
    auto marubozu = cdl_marubozu(data.open, data.high, data.low, data.close, 0.05);
    
    // Count detected patterns
    auto count_pattern = [](const std::vector<int32_t>& result) {
        return std::count_if(result.begin(), result.end(), [](int32_t x) { return x != 0; });
    };
    
    std::cout << "Pattern Detection Summary:" << std::endl;
    std::cout << std::left << std::setw(25) << "  Pattern" << std::setw(10) << "Count" << std::endl;
    std::cout << "  " << std::string(35, '-') << std::endl;
    std::cout << std::left << std::setw(25) << "  Doji" << count_pattern(doji) << std::endl;
    std::cout << std::left << std::setw(25) << "  Dragonfly Doji" << count_pattern(dragonfly) << std::endl;
    std::cout << std::left << std::setw(25) << "  Gravestone Doji" << count_pattern(gravestone) << std::endl;
    std::cout << std::left << std::setw(25) << "  Long-Legged Doji" << count_pattern(long_legged) << std::endl;
    std::cout << std::left << std::setw(25) << "  Hammer" << count_pattern(hammer) << std::endl;
    std::cout << std::left << std::setw(25) << "  Inverted Hammer" << count_pattern(inverted_hammer) << std::endl;
    std::cout << std::left << std::setw(25) << "  Hanging Man" << count_pattern(hanging_man) << std::endl;
    std::cout << std::left << std::setw(25) << "  Shooting Star" << count_pattern(shooting_star) << std::endl;
    std::cout << std::left << std::setw(25) << "  Engulfing" << count_pattern(engulfing) << std::endl;
    std::cout << std::left << std::setw(25) << "  Harami" << count_pattern(harami) << std::endl;
    std::cout << std::left << std::setw(25) << "  Morning Star" << count_pattern(morning_star) << std::endl;
    std::cout << std::left << std::setw(25) << "  Evening Star" << count_pattern(evening_star) << std::endl;
    std::cout << std::left << std::setw(25) << "  Three White Soldiers" << count_pattern(three_white) << std::endl;
    std::cout << std::left << std::setw(25) << "  Three Black Crows" << count_pattern(three_black) << std::endl;
    std::cout << std::left << std::setw(25) << "  Marubozu" << count_pattern(marubozu) << std::endl;
    std::cout << std::endl;
}

// Example 14: Error handling
void example_error_handling() {
    print_separator("Error Handling Examples");
    
    // Empty input
    try {
        std::vector<double> empty;
        sma(empty, 14);
    } catch (const TaLibException& e) {
        std::cout << "  Caught expected exception for empty input: " << e.what() << std::endl;
        std::cout << "  Error code: " << e.code() << std::endl;
    }
    
    // Invalid period
    try {
        std::vector<double> data = {1.0, 2.0, 3.0};
        sma(data, 0);
    } catch (const TaLibException& e) {
        std::cout << "  Caught expected exception for invalid period: " << e.what() << std::endl;
    }
    
    std::cout << std::endl;
}

// Example 15: Performance benchmark
void example_benchmark() {
    print_separator("Performance Benchmark");
    
    size_t data_size = 10000;
    size_t iterations = 100;
    
    MarketData large_data(data_size);
    
    std::cout << "Data size: " << data_size << " points, Iterations: " << iterations << std::endl;
    std::cout << std::endl;
    
    auto benchmarks = {
        benchmark("SMA(20)", iterations, sma, large_data.close, 20),
        benchmark("EMA(20)", iterations, ema, large_data.close, 20),
        benchmark("RSI(14)", iterations, rsi, large_data.close, 14),
        benchmark("MACD(12,26,9)", iterations, macd, large_data.close, 12, 26, 9),
        benchmark("BBands(20)", iterations, bbands, large_data.close, 20, 2.0, 2.0),
        benchmark("ATR(14)", iterations, atr, large_data.high, large_data.low, large_data.close, 14),
        benchmark("ADX(14)", iterations, adx, large_data.high, large_data.low, large_data.close, 14),
        benchmark("OBV", iterations, obv, large_data.close, large_data.volume),
        benchmark("Stoch(5,3,3)", iterations, stoch, large_data.high, large_data.low, large_data.close, 5, 3, 3),
    };
    
    std::cout << std::left << std::setw(20) << "Indicator" 
              << std::setw(15) << "Total (ms)" 
              << std::setw(15) << "Avg (ms)" 
              << std::setw(15) << "Ops/sec" << std::endl;
    std::cout << std::string(65, '-') << std::endl;
    
    for (const auto& b : benchmarks) {
        std::cout << std::left << std::setw(20) << b.name
                  << std::fixed << std::setprecision(2) << std::setw(15) << b.total_ms
                  << std::setw(15) << b.avg_ms
                  << std::setw(15) << std::setprecision(0) << b.ops_per_sec
                  << std::endl;
    }
    
    std::cout << std::endl;
}

int main() {
    std::cout << "============================================================" << std::endl;
    std::cout << "  finkit C++ Binding Examples" << std::endl;
    std::cout << "  Library Version: " << version() << std::endl;
    std::cout << "============================================================" << std::endl;
    
    // Generate sample market data
    MarketData data(200);
    
    // Run all examples
    example_moving_averages(data);
    example_bollinger_bands(data);
    example_macd(data);
    example_rsi(data);
    example_stochastic(data);
    example_trend_indicators(data);
    example_momentum(data);
    example_volatility(data);
    example_volume(data);
    example_hilbert(data);
    example_statistics(data);
    example_price_transforms(data);
    example_candlestick_patterns(data);
    example_error_handling();
    example_benchmark();
    
    std::cout << "All examples completed successfully!" << std::endl;
    
    return 0;
}
