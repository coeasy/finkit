// test_indicators.cpp - Comprehensive tests for finkit C++ bindings
#include <iostream>
#include <vector>
#include <cmath>
#include <cassert>
#include <string>
#include <algorithm>
#include <numeric>
#include "finkit.hpp"

using namespace finkit;

// Test helpers
static int tests_passed = 0;
static int tests_failed = 0;

#define TEST(name) std::cout << "[TEST] " << name << "... ";
#define PASS() do { tests_passed++; std::cout << "PASSED" << std::endl; } while(0)
#define FAIL(msg) do { tests_failed++; std::cout << "FAILED: " << msg << std::endl; } while(0)
#define ASSERT(cond, msg) do { if (!(cond)) { FAIL(msg); return; } } while(0)
#define ASSERT_EQ(a, b, msg) ASSERT((a) == (b), msg)
#define ASSERT_NEAR(a, b, eps, msg) ASSERT(std::abs((a) - (b)) < (eps), msg)
#define ASSERT_TRUE(cond, msg) ASSERT((cond), msg)
#define ASSERT_FALSE(cond, msg) ASSERT(!(cond), msg)
#define ASSERT_NOT_NULL(ptr, msg) ASSERT((ptr) != nullptr, msg)
#define ASSERT_NOT_EMPTY(v, msg) ASSERT(!(v).empty(), msg)
#define ASSERT_SIZE(v, n, msg) ASSERT((v).size() == (n), msg)
#define ASSERT_RANGE(v, low, high, msg) do { \
    bool in_range = std::all_of((v).begin(), (v).end(), [&](double x) { return x >= (low) && x <= (high); }); \
    ASSERT(in_range, msg); \
} while(0)

// Generate sample data
std::vector<double> generate_trend_data(size_t n, double start = 1.0, double step = 0.01) {
    std::vector<double> data(n);
    for (size_t i = 0; i < n; ++i) {
        data[i] = start + i * step;
    }
    return data;
}

std::vector<double> generate_oscillating_data(size_t n, double amplitude = 1.0, double period = 20.0) {
    std::vector<double> data(n);
    for (size_t i = 0; i < n; ++i) {
        data[i] = amplitude * std::sin(2.0 * 3.14159265359 * i / period);
    }
    return data;
}

// Test 1: Version and initialization
void test_version() {
    TEST("Version and initialization")
    auto ver = version();
    ASSERT_NOT_EMPTY(ver, "Version string is empty");
    PASS();
}

// Test 2: Simple Moving Average (SMA)
void test_sma() {
    TEST("Simple Moving Average (SMA)")
    auto data = generate_trend_data(100);
    auto result = sma(data, 14);
    ASSERT_SIZE(result, 100, "SMA output size mismatch");
    ASSERT_RANGE(result, 0.0, 2.0, "SMA output out of expected range");
    // For trend data, SMA should lag behind
    ASSERT(result.back() < data.back(), "SMA should lag behind uptrend data");
    PASS();
}

// Test 3: Exponential Moving Average (EMA)
void test_ema() {
    TEST("Exponential Moving Average (EMA)")
    auto data = generate_trend_data(100);
    auto result = ema(data, 14);
    ASSERT_SIZE(result, 100, "EMA output size mismatch");
    // EMA should be closer to current price than SMA
    auto sma_result = sma(data, 14);
    ASSERT(result.back() > sma_result.back(), "EMA should react faster than SMA");
    PASS();
}

// Test 4: MACD
void test_macd() {
    TEST("MACD")
    auto data = generate_trend_data(100);
    auto result = macd(data, 12, 26, 9);
    ASSERT_SIZE(result.macd, 100, "MACD output size mismatch");
    ASSERT_SIZE(result.signal, 100, "MACD signal size mismatch");
    ASSERT_SIZE(result.hist, 100, "MACD histogram size mismatch");
    // Histogram should be MACD - Signal
    for (size_t i = 0; i < 100; ++i) {
        double expected = result.macd[i] - result.signal[i];
        ASSERT_NEAR(result.hist[i], expected, 1e-10, "MACD histogram calculation error");
    }
    PASS();
}

// Test 5: RSI
void test_rsi() {
    TEST("Relative Strength Index (RSI)")
    auto data = generate_trend_data(100);
    auto result = rsi(data, 14);
    ASSERT_SIZE(result, 100, "RSI output size mismatch");
    // RSI should be between 0 and 100
    ASSERT_RANGE(result, 0.0, 100.0, "RSI output out of bounds");
    // For uptrend, RSI should be above 50
    ASSERT(result.back() > 50.0, "RSI should be above 50 for uptrend");
    PASS();
}

// Test 6: Bollinger Bands
void test_bbands() {
    TEST("Bollinger Bands")
    auto data = generate_oscillating_data(100);
    auto result = bbands(data, 20, 2.0, 2.0);
    ASSERT_SIZE(result.upper, 100, "BBands upper size mismatch");
    ASSERT_SIZE(result.middle, 100, "BBands middle size mismatch");
    ASSERT_SIZE(result.lower, 100, "BBands lower size mismatch");
    // Upper should be above middle, lower should be below middle
    for (size_t i = 0; i < 100; ++i) {
        ASSERT(result.upper[i] >= result.middle[i], "Upper band should be above middle");
        ASSERT(result.lower[i] <= result.middle[i], "Lower band should be below middle");
    }
    PASS();
}

// Test 7: Stochastic Oscillator
void test_stoch() {
    TEST("Stochastic Oscillator")
    auto high = generate_trend_data(100, 1.1, 0.01);
    auto low = generate_trend_data(100, 0.9, 0.01);
    auto close = generate_trend_data(100, 1.0, 0.01);
    auto result = stoch(high, low, close, 5, 3, 3);
    ASSERT_SIZE(result.k, 100, "Stochastic K size mismatch");
    ASSERT_SIZE(result.d, 100, "Stochastic D size mismatch");
    ASSERT_RANGE(result.k, 0.0, 100.0, "Stochastic K out of bounds");
    ASSERT_RANGE(result.d, 0.0, 100.0, "Stochastic D out of bounds");
    PASS();
}

// Test 8: ATR
void test_atr() {
    TEST("Average True Range (ATR)")
    auto high = generate_trend_data(100, 1.1, 0.01);
    auto low = generate_trend_data(100, 0.9, 0.01);
    auto close = generate_trend_data(100, 1.0, 0.01);
    auto result = atr(high, low, close, 14);
    ASSERT_SIZE(result, 100, "ATR output size mismatch");
    // ATR should be positive
    ASSERT(std::all_of(result.begin(), result.end(), [](double x) { return x >= 0.0; }), "ATR should be non-negative");
    PASS();
}

// Test 9: ADX
void test_adx() {
    TEST("Average Directional Index (ADX)")
    auto high = generate_trend_data(100, 1.1, 0.01);
    auto low = generate_trend_data(100, 0.9, 0.01);
    auto close = generate_trend_data(100, 1.0, 0.01);
    auto result = adx(high, low, close, 14);
    ASSERT_SIZE(result, 100, "ADX output size mismatch");
    ASSERT_RANGE(result, 0.0, 100.0, "ADX out of bounds");
    PASS();
}

// Test 10: OBV
void test_obv() {
    TEST("On Balance Volume (OBV)")
    auto close = generate_trend_data(100);
    auto volume = std::vector<double>(100, 1000.0);
    auto result = obv(close, volume);
    ASSERT_SIZE(result, 100, "OBV output size mismatch");
    // For uptrend, OBV should be increasing
    ASSERT(result.back() > result.front(), "OBV should increase for uptrend");
    PASS();
}

// Test 11: CCI
void test_cci() {
    TEST("Commodity Channel Index (CCI)")
    auto high = generate_trend_data(100, 1.1, 0.01);
    auto low = generate_trend_data(100, 0.9, 0.01);
    auto close = generate_trend_data(100, 1.0, 0.01);
    auto result = cci(high, low, close, 14);
    ASSERT_SIZE(result, 100, "CCI output size mismatch");
    PASS();
}

// Test 12: Williams %R
void test_willr() {
    TEST("Williams %R")
    auto high = generate_trend_data(100, 1.1, 0.01);
    auto low = generate_trend_data(100, 0.9, 0.01);
    auto close = generate_trend_data(100, 1.0, 0.01);
    auto result = willr(high, low, close, 14);
    ASSERT_SIZE(result, 100, "Williams %R output size mismatch");
    ASSERT_RANGE(result, -100.0, 0.0, "Williams %R out of bounds");
    PASS();
}

// Test 13: MAMA
void test_mama() {
    TEST("MESA Adaptive Moving Average (MAMA)")
    auto data = generate_oscillating_data(100);
    auto result = mama(data, 0.5, 0.05);
    ASSERT_SIZE(result.mama, 100, "MAMA output size mismatch");
    ASSERT_SIZE(result.fama, 100, "FAMA output size mismatch");
    PASS();
}

// Test 14: T3
void test_t3() {
    TEST("T3 Moving Average")
    auto data = generate_trend_data(100);
    auto result = t3(data, 5, 0.7);
    ASSERT_SIZE(result, 100, "T3 output size mismatch");
    PASS();
}

// Test 15: KAMA
void test_kama() {
    TEST("Kaufman Adaptive Moving Average (KAMA)")
    auto data = generate_trend_data(100);
    auto result = kama(data, 14);
    ASSERT_SIZE(result, 100, "KAMA output size mismatch");
    PASS();
}

// Test 16: Hilbert Transform - Dominant Cycle Period
void test_ht_dcperiod() {
    TEST("Hilbert Transform - Dominant Cycle Period")
    auto data = generate_oscillating_data(100);
    auto result = ht_dcperiod(data);
    ASSERT_SIZE(result, 100, "HT DCPERIOD output size mismatch");
    PASS();
}

// Test 17: Hilbert Transform - Sine Wave
void test_ht_sine() {
    TEST("Hilbert Transform - Sine Wave")
    auto data = generate_oscillating_data(100);
    auto result = ht_sine(data);
    ASSERT_SIZE(result.sine, 100, "HT SINE output size mismatch");
    ASSERT_SIZE(result.lead_sine, 100, "HT LEAD SINE output size mismatch");
    PASS();
}

// Test 18: Z-Score
void test_zscore() {
    TEST("Z-Score")
    auto data = generate_oscillating_data(100);
    auto result = zscore(data, 20);
    ASSERT_SIZE(result, 100, "Z-Score output size mismatch");
    PASS();
}

// Test 19: Correlation
void test_correlation() {
    TEST("Correlation")
    auto a = generate_trend_data(100, 1.0, 0.01);
    auto b = generate_trend_data(100, 2.0, 0.02);
    auto result = correlation(a, b, 30);
    ASSERT_SIZE(result, 100, "Correlation output size mismatch");
    ASSERT_RANGE(result, -1.0, 1.0, "Correlation out of bounds");
    PASS();
}

// Test 20: Standard Deviation
void test_stddev() {
    TEST("Standard Deviation")
    auto data = generate_oscillating_data(100);
    auto result = stddev(data, 20, 1.0);
    ASSERT_SIZE(result, 100, "StdDev output size mismatch");
    ASSERT(std::all_of(result.begin(), result.end(), [](double x) { return x >= 0.0; }), "StdDev should be non-negative");
    PASS();
}

// Test 21: Linear Regression
void test_linear_reg() {
    TEST("Linear Regression")
    auto data = generate_trend_data(100);
    auto result = linear_reg(data, 14);
    ASSERT_SIZE(result, 100, "Linear Regression output size mismatch");
    PASS();
}

// Test 22: Time Series Forecast
void test_tsf() {
    TEST("Time Series Forecast (TSF)")
    auto data = generate_trend_data(100);
    auto result = tsf(data, 14);
    ASSERT_SIZE(result, 100, "TSF output size mismatch");
    PASS();
}

// Test 23: Candlestick - Doji
void test_cdl_doji() {
    TEST("Candlestick - Doji")
    auto open = std::vector<double>(100, 1.0);
    auto high = std::vector<double>(100, 1.05);
    auto low = std::vector<double>(100, 0.95);
    auto close = std::vector<double>(100, 1.0);
    auto result = cdl_doji(open, high, low, close, 0.1);
    ASSERT_SIZE(result, 100, "Doji output size mismatch");
    // All should be doji since open == close
    ASSERT(std::all_of(result.begin(), result.end(), [](int32_t x) { return x != 0; }), "Should detect doji pattern");
    PASS();
}

// Test 24: Candlestick - Engulfing
void test_cdl_engulfing() {
    TEST("Candlestick - Engulfing")
    // Create a bearish then bullish pattern
    std::vector<double> open = {100, 98, 95, 93, 96};
    std::vector<double> high = {102, 99, 96, 94, 100};
    std::vector<double> low = {98, 96, 93, 91, 94};
    std::vector<double> close = {98, 96, 93, 92, 99};
    auto result = cdl_engulfing(open, high, low, close);
    ASSERT_SIZE(result, 5, "Engulfing output size mismatch");
    PASS();
}

// Test 25: Error handling with empty input
void test_error_handling() {
    TEST("Error handling with empty input")
    bool caught = false;
    try {
        std::vector<double> empty;
        sma(empty, 14);
    } catch (const TaLibException& e) {
        caught = true;
        ASSERT(e.code() != 0, "Exception should have error code");
    }
    ASSERT(caught, "Should throw exception for empty input");
    PASS();
}

// Test 26: Multiple moving averages comparison
void test_moving_averages_comparison() {
    TEST("Multiple Moving Averages Comparison")
    auto data = generate_trend_data(200);
    auto sma_result = sma(data, 20);
    auto ema_result = ema(data, 20);
    auto wma_result = wma(data, 20);
    auto dema_result = dema(data, 20);
    auto tema_result = tema(data, 20);
    auto kama_result = kama(data, 20);
    auto t3_result = t3(data, 5, 0.7);
    
    ASSERT_SIZE(sma_result, 200, "SMA size mismatch");
    ASSERT_SIZE(ema_result, 200, "EMA size mismatch");
    ASSERT_SIZE(wma_result, 200, "WMA size mismatch");
    ASSERT_SIZE(dema_result, 200, "DEMA size mismatch");
    ASSERT_SIZE(tema_result, 200, "TEMA size mismatch");
    ASSERT_SIZE(kama_result, 200, "KAMA size mismatch");
    ASSERT_SIZE(t3_result, 200, "T3 size mismatch");
    
    PASS();
}

// Test 27: Price transforms
void test_price_transforms() {
    TEST("Price Transforms")
    auto open = generate_trend_data(100, 100.0, 0.1);
    auto high = generate_trend_data(100, 101.0, 0.1);
    auto low = generate_trend_data(100, 99.0, 0.1);
    auto close = generate_trend_data(100, 100.5, 0.1);
    
    auto avg = avgprice(open, high, low, close);
    auto med = medprice(high, low);
    auto typ = typprice(high, low, close);
    auto wcl = wclprice(high, low, close);
    
    ASSERT_SIZE(avg, 100, "AvgPrice size mismatch");
    ASSERT_SIZE(med, 100, "MedPrice size mismatch");
    ASSERT_SIZE(typ, 100, "TypPrice size mismatch");
    ASSERT_SIZE(wcl, 100, "WCLPrice size mismatch");
    
    PASS();
}

// Test 28: Benchmark
void test_benchmark() {
    TEST("Benchmark Utility")
    auto data = generate_trend_data(1000);
    auto result = benchmark("SMA(20) 100 iterations", 100, sma, data, 20);
    
    ASSERT(result.iterations == 100, "Benchmark iterations mismatch");
    ASSERT(result.total_ms > 0, "Benchmark total time should be positive");
    ASSERT(result.avg_ms > 0, "Benchmark avg time should be positive");
    ASSERT(result.ops_per_sec > 0, "Benchmark ops/sec should be positive");
    
    PASS();
}

int main() {
    std::cout << "========================================" << std::endl;
    std::cout << "  finkit C++ Binding Tests" << std::endl;
    std::cout << "  Version: " << version() << std::endl;
    std::cout << "========================================" << std::endl;
    std::cout << std::endl;
    
    // Core indicators
    test_version();
    test_sma();
    test_ema();
    test_macd();
    test_rsi();
    test_bbands();
    test_stoch();
    test_atr();
    test_adx();
    test_obv();
    test_cci();
    test_willr();
    
    // Advanced indicators
    test_mama();
    test_t3();
    test_kama();
    
    // Hilbert transforms
    test_ht_dcperiod();
    test_ht_sine();
    
    // Statistics
    test_zscore();
    test_correlation();
    test_stddev();
    test_linear_reg();
    test_tsf();
    
    // Price transforms
    test_price_transforms();
    
    // Candlestick patterns
    test_cdl_doji();
    test_cdl_engulfing();
    
    // Error handling and utilities
    test_error_handling();
    test_moving_averages_comparison();
    test_benchmark();
    
    std::cout << std::endl;
    std::cout << "========================================" << std::endl;
    std::cout << "  Test Results:" << std::endl;
    std::cout << "  Passed: " << tests_passed << std::endl;
    std::cout << "  Failed: " << tests_failed << std::endl;
    std::cout << "  Total:  " << (tests_passed + tests_failed) << std::endl;
    std::cout << "========================================" << std::endl;
    
    return tests_failed == 0 ? 0 : 1;
}
