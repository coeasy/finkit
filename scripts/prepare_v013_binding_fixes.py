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


def main() -> None:
    fix_java_duplicates()
    fix_consumer_cmake(ROOT / "ffi/c-binding/tests/CMakeLists.txt")
    fix_consumer_cmake(ROOT / "ffi/c-binding/examples/CMakeLists.txt")
    fix_cpp_ohlc_helpers()


if __name__ == "__main__":
    main()
