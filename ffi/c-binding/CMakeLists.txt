cmake_minimum_required(VERSION 3.16)

project(finkit
    VERSION 1.0.0
    DESCRIPTION "Finkit - Financial Technical Analysis Library C/C++ Bindings"
    LANGUAGES CXX C
)

# ----------------------------------------------------------------------------
# Options
# ----------------------------------------------------------------------------
option(FINKIT_BUILD_TESTS    "Build finkit C/C++ binding tests"    ON)
option(FINKIT_BUILD_EXAMPLES "Build finkit C/C++ binding examples" ON)
option(FINKIT_AUTO_BUILD_RS  "Auto-invoke `cargo build` when the prebuilt FFI library is missing" ON)
option(FINKIT_INSTALL_CMAKECONFIG "Generate and install finkit-config.cmake" ON)

find_package(PkgConfig REQUIRED)
include(GNUInstallDirs)

# ----------------------------------------------------------------------------
# Cross-platform shared library naming
#
# Rust produces a `cdylib` whose filename follows the platform's convention:
#   * Windows : finkit_ffi.dll (+ .dll.lib import lib)
#   * Linux   : libfinkit_ffi.so
#   * macOS   : libfinkit_ffi.dylib
# ----------------------------------------------------------------------------
set(FINKIT_RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(FINKIT_TARGET_DIR "${FINKIT_RUST_DIR}/target/release")

if(WIN32)
    set(FINKIT_SHARED_NAME  "finkit_ffi.dll")
    set(FINKIT_IMPLIB_NAME  "finkit_ffi.dll.lib")
    set(FINKIT_CARGO_TARGET_TRIPLE "")
elseif(APPLE)
    set(FINKIT_SHARED_NAME  "libfinkit_ffi.dylib")
    set(FINKIT_IMPLIB_NAME  "")
    # Apple Silicon vs Intel — pick the same triple Rust would produce by default
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
        set(FINKIT_CARGO_TARGET_TRIPLE "aarch64-apple-darwin")
    else()
        set(FINKIT_CARGO_TARGET_TRIPLE "x86_64-apple-darwin")
    endif()
elseif(UNIX)
    set(FINKIT_SHARED_NAME  "libfinkit_ffi.so")
    set(FINKIT_IMPLIB_NAME  "")
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64|arm64")
        set(FINKIT_CARGO_TARGET_TRIPLE "aarch64-unknown-linux-gnu")
    else()
        set(FINKIT_CARGO_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
    endif()
else()
    message(FATAL_ERROR "finkit: unsupported platform (WIN32/APPLE/UNIX only).")
endif()

set(FINKIT_SHARED_PATH "${FINKIT_TARGET_DIR}/${FINKIT_SHARED_NAME}")
if(FINKIT_IMPLIB_NAME)
    set(FINKIT_IMPLIB_PATH "${FINKIT_TARGET_DIR}/${FINKIT_IMPLIB_NAME}")
else()
    set(FINKIT_IMPLIB_PATH "")
endif()

# ----------------------------------------------------------------------------
# Auto-build the Rust library when the prebuilt artifact is missing.
#
# This is intentionally a *best effort* convenience for downstream consumers:
# when they clone the repo and configure with cmake, the FFI dylib is produced
# for them in the workspace's `target/release` directory.
# ----------------------------------------------------------------------------
if(FINKIT_AUTO_BUILD_RS AND NOT EXISTS "${FINKIT_SHARED_PATH}")
    message(STATUS "finkit: prebuilt '${FINKIT_SHARED_NAME}' not found, invoking `cargo build --release`")
    set(_cargo_args "build" "--release")
    if(FINKIT_CARGO_TARGET_TRIPLE)
        list(APPEND _cargo_args "--target" "${FINKIT_CARGO_TARGET_TRIPLE}")
    endif()
    execute_process(
        COMMAND cargo ${_cargo_args} -p finkit-ffi
        WORKING_DIRECTORY "${FINKIT_RUST_DIR}"
        RESULT_VARIABLE FINKIT_CARGO_RESULT
    )
    if(NOT FINKIT_CARGO_RESULT EQUAL 0)
        message(FATAL_ERROR "finkit: cargo build --release failed (${FINKIT_CARGO_RESULT}).")
    endif()
endif()

# ----------------------------------------------------------------------------
# Imported target: the actual shared library
# ----------------------------------------------------------------------------
add_library(finkit SHARED IMPORTED GLOBAL)
set_target_properties(finkit PROPERTIES
    IMPORTED_LOCATION "${FINKIT_SHARED_PATH}"
    INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
if(FINKIT_IMPLIB_PATH AND EXISTS "${FINKIT_IMPLIB_PATH}")
    set_target_properties(finkit PROPERTIES IMPORTED_IMPLIB "${FINKIT_IMPLIB_PATH}")
endif()

# Header-only target (so downstream `target_link_libraries(... finkit::headers)`
# works without touching the .so / .dll).
add_library(finkit_headers INTERFACE)
target_include_directories(finkit_headers INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
)

add_library(finkit::headers ALIAS finkit_headers)

# Umbrella target: link this and you get both headers and the dylib.
add_library(finkit::finkit INTERFACE IMPORTED)
set_target_properties(finkit::finkit PROPERTIES
    INTERFACE_LINK_LIBRARIES "finkit;finkit_headers"
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
    install(IMPORTED_RUNTIME_ARTIFACTS finkit
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}  # finkit_ffi.dll
    )
    # On Windows, the .dll.lib import lib is not a runtime artifact, install it
    # alongside the .dll so that downstream targets that link against finkit
    # have a `.lib` to consume.
    if(FINKIT_IMPLIB_PATH AND EXISTS "${FINKIT_IMPLIB_PATH}")
        install(FILES "${FINKIT_IMPLIB_PATH}"
            DESTINATION ${CMAKE_INSTALL_LIBDIR}
        )
    endif()
else()
    install(IMPORTED_RUNTIME_ARTIFACTS finkit
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}  # .so / .dylib
    )
endif()

install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/include/
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    FILES_MATCHING PATTERN "*.h" PATTERN "*.hpp"
)

# CMake package config so downstream projects can do
#   find_package(finkit CONFIG REQUIRED)
#   target_link_libraries(my_app PRIVATE finkit::finkit)
if(FINKIT_INSTALL_CMAKECONFIG)
    set(FINKIT_CMAKE_CONFIG_DEST "${CMAKE_INSTALL_LIBDIR}/cmake/finkit")

    # Note: we don't install(EXPORT ...) here because the finkit target is
    # an IMPORTED SHARED target, not a real one with an associated .lib/.a.
    # Instead, finkit-config.cmake constructs the imported target on demand.

    configure_package_config_file(
        "${CMAKE_CURRENT_SOURCE_DIR}/cmake/finkit-config.cmake.in"
        "${CMAKE_CURRENT_BINARY_DIR}/finkit-config.cmake"
        INSTALL_DESTINATION ${FINKIT_CMAKE_CONFIG_DEST}
    )

    write_basic_package_version_file(
        "${CMAKE_CURRENT_BINARY_DIR}/finkit-config-version.cmake"
        VERSION ${PROJECT_VERSION}
        COMPATIBILITY SameMajorVersion
    )

    install(FILES
        "${CMAKE_CURRENT_BINARY_DIR}/finkit-config.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/finkit-config-version.cmake"
        DESTINATION ${FINKIT_CMAKE_CONFIG_DEST}
    )
endif()

# pkg-config
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/finkit.pc.in"
    "${CMAKE_CURRENT_BINARY_DIR}/finkit.pc"
    @ONLY
)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/finkit.pc"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig
)

if(FINKIT_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(FINKIT_BUILD_EXAMPLES)
    add_subdirectory(examples)
endif()
