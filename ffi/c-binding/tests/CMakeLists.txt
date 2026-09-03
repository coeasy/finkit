cmake_minimum_required(VERSION 3.12)

project(finkit_tests CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Reuse the parent build targets when included from the source tree.
# Only resolve an installed SDK when this directory is configured standalone.
if(NOT TARGET finkit OR NOT TARGET finkit_headers)
    find_package(finkit CONFIG REQUIRED)
endif()

add_executable(test_indicators test_indicators.cpp)
target_link_libraries(test_indicators finkit finkit_headers)

add_test(NAME test_indicators COMMAND test_indicators)
