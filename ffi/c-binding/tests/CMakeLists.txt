cmake_minimum_required(VERSION 3.12)

project(finkit_tests CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Find parent project
find_package(finkit REQUIRED PATHS "${CMAKE_CURRENT_SOURCE_DIR}/..")

add_executable(test_indicators test_indicators.cpp)
target_link_libraries(test_indicators finkit finkit_headers)

add_test(NAME test_indicators COMMAND test_indicators)
