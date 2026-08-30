cmake_minimum_required(VERSION 3.16)

project(alpha_ta
    VERSION 1.0.0
    DESCRIPTION "AlphaTA - Financial Technical Analysis Library C/C++ Bindings"
    LANGUAGES CXX C
)

# ----------------------------------------------------------------------------
# Options
# ----------------------------------------------------------------------------
option(ALPHA_TA_BUILD_TESTS    "Build alpha_ta C/C++ binding tests"    ON)
option(ALPHA_TA_BUILD_EXAMPLES "Build alpha_ta C/C++ binding examples" ON)
option(ALPHA_TA_AUTO_BUILD_RS  "Auto-invoke `cargo build` when the prebuilt FFI library is missing" ON)
option(ALPHA_TA_INSTALL_CMAKECONFIG "Generate and install alpha_ta-config.cmake" ON)

find_package(PkgConfig REQUIRED)
include(GNUInstallDirs)

# ----------------------------------------------------------------------------
# Cross-platform shared library naming
#
# Rust produces a `cdylib` whose filename follows the platform's convention:
#   * Windows : alpha_ta_ffi.dll (+ .dll.lib import lib)
#   * Linux   : libalpha_ta_ffi.so
#   * macOS   : libalpha_ta_ffi.dylib
# ----------------------------------------------------------------------------
set(ALPHA_TA_RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(ALPHA_TA_TARGET_DIR "${ALPHA_TA_RUST_DIR}/target/release")

if(WIN32)
    set(ALPHA_TA_SHARED_NAME  "alpha_ta_ffi.dll")
    set(ALPHA_TA_IMPLIB_NAME  "alpha_ta_ffi.dll.lib")
    set(ALPHA_TA_CARGO_TARGET_TRIPLE "")
elseif(APPLE)
    set(ALPHA_TA_SHARED_NAME  "libalpha_ta_ffi.dylib")
    set(ALPHA_TA_IMPLIB_NAME  "")
    # Apple Silicon vs Intel — pick the same triple Rust would produce by default
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
        set(ALPHA_TA_CARGO_TARGET_TRIPLE "aarch64-apple-darwin")
    else()
        set(ALPHA_TA_CARGO_TARGET_TRIPLE "x86_64-apple-darwin")
    endif()
elseif(UNIX)
    set(ALPHA_TA_SHARED_NAME  "libalpha_ta_ffi.so")
    set(ALPHA_TA_IMPLIB_NAME  "")
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64|arm64")
        set(ALPHA_TA_CARGO_TARGET_TRIPLE "aarch64-unknown-linux-gnu")
    else()
        set(ALPHA_TA_CARGO_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    endif()
else()
    message(FATAL_ERROR "alpha_ta: unsupported platform (WIN32/APPLE/UNIX only).")
endif()

set(ALPHA_TA_SHARED_PATH "${ALPHA_TA_TARGET_DIR}/${ALPHA_TA_SHARED_NAME}")
if(ALPHA_TA_IMPLIB_NAME)
    set(ALPHA_TA_IMPLIB_PATH "${ALPHA_TA_TARGET_DIR}/${ALPHA_TA_IMPLIB_NAME}")
else()
    set(ALPHA_TA_IMPLIB_PATH "")
endif()

# ----------------------------------------------------------------------------
# Auto-build the Rust library when the prebuilt artifact is missing.
#
# This is intentionally a *best effort* convenience for downstream consumers:
# when they clone the repo and configure with cmake, the FFI dylib is produced
# for them in the workspace's `target/release` directory.
# ----------------------------------------------------------------------------
if(ALPHA_TA_AUTO_BUILD_RS AND NOT EXISTS "${ALPHA_TA_SHARED_PATH}")
    message(STATUS "alpha_ta: prebuilt '${ALPHA_TA_SHARED_NAME}' not found, invoking `cargo build --release`")
    set(_cargo_args "build" "--release")
    if(ALPHA_TA_CARGO_TARGET_TRIPLE)
        list(APPEND _cargo_args "--target" "${ALPHA_TA_CARGO_TARGET_TRIPLE}")
    endif()
    execute_process(
        COMMAND cargo ${_cargo_args} -p alpha-ta-ffi
        WORKING_DIRECTORY "${ALPHA_TA_RUST_DIR}"
        RESULT_VARIABLE ALPHA_TA_CARGO_RESULT
    )
    if(NOT ALPHA_TA_CARGO_RESULT EQUAL 0)
        message(FATAL_ERROR "alpha_ta: cargo build --release failed (${ALPHA_TA_CARGO_RESULT}).")
    endif()
endif()

# ----------------------------------------------------------------------------
# Imported target: the actual shared library
# ----------------------------------------------------------------------------
add_library(alpha_ta SHARED IMPORTED GLOBAL)
set_target_properties(alpha_ta PROPERTIES
    IMPORTED_LOCATION "${ALPHA_TA_SHARED_PATH}"
    INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
if(ALPHA_TA_IMPLIB_PATH AND EXISTS "${ALPHA_TA_IMPLIB_PATH}")
    set_target_properties(alpha_ta PROPERTIES IMPORTED_IMPLIB "${ALPHA_TA_IMPLIB_PATH}")
endif()

# Header-only target (so downstream `target_link_libraries(... alpha_ta::headers)`
# works without touching the .so / .dll).
add_library(alpha_ta_headers INTERFACE)
target_include_directories(alpha_ta_headers INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
)

add_library(alpha_ta::headers ALIAS alpha_ta_headers)

# Umbrella target: link this and you get both headers and the dylib.
add_library(alpha_ta::alpha_ta INTERFACE IMPORTED)
set_target_properties(alpha_ta::alpha_ta PROPERTIES
    INTERFACE_LINK_LIBRARIES "alpha_ta;alpha_ta_headers"
)

# ----------------------------------------------------------------------------
# install() rules — cross-platform
#
# On Windows a DLL is a RUNTIME; the import lib (`.dll.lib`) is an ARCHIVE.
# On POSIX `.so` / `.dylib` are LIBRARY (developer) artifacts and must also be
# installed so that dynamically loaded executables can find them.
# ----------------------------------------------------------------------------
include(CMakePackageConfigHelpers)

if(WIN32)
    install(IMPORTED_RUNTIME_ARTIFACTS alpha_ta
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}  # alpha_ta_ffi.dll
    )
    # On Windows, the .dll.lib import lib is not a runtime artifact, install it
    # alongside the .dll so that downstream targets that link against alpha_ta
    # have a `.lib` to consume.
    if(ALPHA_TA_IMPLIB_PATH AND EXISTS "${ALPHA_TA_IMPLIB_PATH}")
        install(FILES "${ALPHA_TA_IMPLIB_PATH}"
            DESTINATION ${CMAKE_INSTALL_LIBDIR}
        )
    endif()
else()
    install(IMPORTED_RUNTIME_ARTIFACTS alpha_ta
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}  # .so / .dylib
    )
endif()

install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/include/
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    FILES_MATCHING PATTERN "*.h" PATTERN "*.hpp"
)

# CMake package config so downstream projects can do
#   find_package(alpha_ta CONFIG REQUIRED)
#   target_link_libraries(my_app PRIVATE alpha_ta::alpha_ta)
if(ALPHA_TA_INSTALL_CMAKECONFIG)
    set(ALPHA_TA_CMAKE_CONFIG_DEST "${CMAKE_INSTALL_LIBDIR}/cmake/alpha_ta")

    # Note: we don't install(EXPORT ...) here because the alpha_ta target is
    # an IMPORTED SHARED target, not a real one with an associated .lib/.a.
    # Instead, alpha_ta-config.cmake constructs the imported target on demand.

    configure_package_config_file(
        "${CMAKE_CURRENT_SOURCE_DIR}/cmake/alpha_ta-config.cmake.in"
        "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta-config.cmake"
        INSTALL_DESTINATION ${ALPHA_TA_CMAKE_CONFIG_DEST}
    )

    write_basic_package_version_file(
        "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta-config-version.cmake"
        VERSION ${PROJECT_VERSION}
        COMPATIBILITY SameMajorVersion
    )

    install(FILES
        "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta-config.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta-config-version.cmake"
        DESTINATION ${ALPHA_TA_CMAKE_CONFIG_DEST}
    )
endif()

# pkg-config
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/alpha_ta.pc.in"
    "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta.pc"
    @ONLY
)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/alpha_ta.pc"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig
)

if(ALPHA_TA_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(ALPHA_TA_BUILD_EXAMPLES)
    add_subdirectory(examples)
endif()
