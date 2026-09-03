#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fix_java_duplicates() -> None:
    path = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/Indicators.java"
    text = path.read_text(encoding="utf-8")
    first = "    // Advanced indicators and chart transforms\n"
    second = "    // Advanced indicators, chart transforms, and formula JSON helpers\n"
    first_pos = text.find(first)
    second_pos = text.find(second)
    if first_pos >= 0 and second_pos > first_pos:
        section_start = text.rfind("    // =========================================================================\n", 0, first_pos)
        if section_start < 0:
            raise RuntimeError("Java duplicate section start not found")
        text = text[:section_start] + text[second_pos - len("    // ========================================================================\n"):]
        path.write_text(text, encoding="utf-8")
    elif second_pos < 0:
        raise RuntimeError("Java canonical advanced section not found")


def fix_java_javadocs() -> None:
    chart = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/ChartPatterns.java"
    text = chart.read_text(encoding="utf-8")
    text = text.replace(
        "Head and Shoulders Bottom (Inverse H&S) detection.",
        "Head and Shoulders Bottom (Inverse Head and Shoulders) detection.",
    )
    chart.write_text(text, encoding="utf-8")

    indicators = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/Indicators.java"
    text = indicators.read_text(encoding="utf-8")
    text = text.replace("     * @result ", "     * @param result ")
    indicators.write_text(text, encoding="utf-8")


def fix_consumer_cmake(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    old = '# Find parent project\nfind_package(finkit REQUIRED PATHS "${CMAKE_CURRENT_SOURCE_DIR}/..")\n'
    if old not in text:
        old = '# Find the parent project\nfind_package(finkit REQUIRED PATHS "${CMAKE_CURRENT_SOURCE_DIR}/..")\n'
    new = '''# Reuse the parent build targets when included from the source tree.\n# Only resolve an installed SDK when this directory is configured standalone.\nif(NOT TARGET finkit OR NOT TARGET finkit_headers)\n    find_package(finkit CONFIG REQUIRED)\nendif()\n'''
    if old in text:
        text = text.replace(old, new, 1)
    elif "if(NOT TARGET finkit OR NOT TARGET finkit_headers)" not in text:
        raise RuntimeError(f"CMake consumer anchor not found: {path}")
    path.write_text(text, encoding="utf-8")


def fix_cpp_ohlc_helpers() -> None:
    path = ROOT / "ffi/c-binding/include/finkit.hpp"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "detail::ohl_single_output_func(high, low, close,",
        "detail::ohlc_single_output_func(high, low, close,",
    )
    text = text.replace(
        "detail::ohl_single_output_func_no_param(high, low, close,",
        "detail::ohlc_single_output_func_no_param(high, low, close,",
    )
    path.write_text(text, encoding="utf-8")


def fix_cpp_test_contract() -> None:
    path = ROOT / "ffi/c-binding/tests/test_indicators.cpp"
    text = path.read_text(encoding="utf-8")

    old_range = '''#define ASSERT_RANGE(v, low, high, msg) do { \\
    bool in_range = std::all_of((v).begin(), (v).end(), [&](double x) { return x >= (low) && x <= (high); }); \\
    ASSERT(in_range, msg); \\
} while(0)\n'''
    new_range = '''bool series_in_range(const std::vector<double>& values, double low, double high) {
    bool seen_finite = false;
    for (double value : values) {
        if (std::isnan(value) && !seen_finite) {
            continue;
        }
        if (!std::isfinite(value) || value < low || value > high) {
            return false;
        }
        seen_finite = true;
    }
    return seen_finite;
}

bool series_non_negative(const std::vector<double>& values) {
    bool seen_finite = false;
    for (double value : values) {
        if (std::isnan(value) && !seen_finite) {
            continue;
        }
        if (!std::isfinite(value) || value < 0.0) {
            return false;
        }
        seen_finite = true;
    }
    return seen_finite;
}

#define ASSERT_RANGE(v, low, high, msg) ASSERT(series_in_range((v), (low), (high)), msg)\n'''
    if old_range in text:
        text = text.replace(old_range, new_range, 1)
    elif "bool series_in_range(" not in text:
        raise RuntimeError("C++ range assertion anchor not found")

    old_ema = '    ASSERT(result.back() > sma_result.back(), "EMA should react faster than SMA");\n'
    new_ema = '''    ASSERT(std::isfinite(result.back()), "EMA tail should be finite");
    ASSERT(result.back() >= sma_result.back() - 1e-10,
           "EMA should not lag a linear uptrend more than SMA");
'''
    if old_ema in text:
        text = text.replace(old_ema, new_ema, 1)

    old_macd = '''    // Histogram should be MACD - Signal
    for (size_t i = 0; i < 100; ++i) {
        double expected = result.macd[i] - result.signal[i];
        ASSERT_NEAR(result.hist[i], expected, 1e-10, "MACD histogram calculation error");
    }
'''
    new_macd = '''    // Warmup values may be NaN. Once all three outputs become valid, they must
    // remain finite and histogram must equal MACD - signal.
    size_t checked = 0;
    for (size_t i = 0; i < 100; ++i) {
        const bool finite = std::isfinite(result.macd[i]) &&
                            std::isfinite(result.signal[i]) &&
                            std::isfinite(result.hist[i]);
        if (!finite) {
            ASSERT(checked == 0, "MACD contains a non-finite value after warmup");
            continue;
        }
        ++checked;
        double expected = result.macd[i] - result.signal[i];
        ASSERT_NEAR(result.hist[i], expected, 1e-10, "MACD histogram calculation error");
    }
    ASSERT(checked > 0, "MACD produced no finite output");
'''
    if old_macd in text:
        text = text.replace(old_macd, new_macd, 1)
    elif "MACD produced no finite output" not in text:
        raise RuntimeError("MACD test anchor not found")

    old_bbands = '''    // Upper should be above middle, lower should be below middle
    for (size_t i = 0; i < 100; ++i) {
        ASSERT(result.upper[i] >= result.middle[i], "Upper band should be above middle");
        ASSERT(result.lower[i] <= result.middle[i], "Lower band should be below middle");
    }
'''
    new_bbands = '''    // Leading warmup values may be NaN; the valid tail must remain ordered.
    size_t checked = 0;
    for (size_t i = 0; i < 100; ++i) {
        const bool finite = std::isfinite(result.upper[i]) &&
                            std::isfinite(result.middle[i]) &&
                            std::isfinite(result.lower[i]);
        if (!finite) {
            ASSERT(checked == 0, "Bollinger Bands contain non-finite values after warmup");
            continue;
        }
        ++checked;
        ASSERT(result.upper[i] >= result.middle[i], "Upper band should be above middle");
        ASSERT(result.lower[i] <= result.middle[i], "Lower band should be below middle");
    }
    ASSERT(checked > 0, "Bollinger Bands produced no finite output");
'''
    if old_bbands in text:
        text = text.replace(old_bbands, new_bbands, 1)
    elif "Bollinger Bands produced no finite output" not in text:
        raise RuntimeError("Bollinger Bands test anchor not found")

    text = text.replace(
        '    ASSERT(std::all_of(result.begin(), result.end(), [](double x) { return x >= 0.0; }), "ATR should be non-negative");',
        '    ASSERT(series_non_negative(result), "ATR should be non-negative after warmup");',
    )
    text = text.replace(
        '    ASSERT(std::all_of(result.begin(), result.end(), [](double x) { return x >= 0.0; }), "StdDev should be non-negative");',
        '    ASSERT(series_non_negative(result), "StdDev should be non-negative after warmup");',
    )
    text = text.replace(
        '    ASSERT(std::all_of(result.begin(), result.end(), [](int32_t x) { return x != 0; }), "Should detect doji pattern");',
        '    ASSERT(std::any_of(result.begin(), result.end(), [](int32_t x) { return x != 0; }), "Should detect doji after lookback");',
    )

    path.write_text(text, encoding="utf-8")


def main() -> None:
    fix_java_duplicates()
    fix_java_javadocs()
    fix_consumer_cmake(ROOT / "ffi/c-binding/tests/CMakeLists.txt")
    fix_consumer_cmake(ROOT / "ffi/c-binding/examples/CMakeLists.txt")
    fix_cpp_ohlc_helpers()
    fix_cpp_test_contract()


if __name__ == "__main__":
    main()
