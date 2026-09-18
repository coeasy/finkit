cmake_minimum_required(VERSION 3.12)

project(finkit_tests C CXX)

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

add_executable(test_c_api test_c_api.c)
target_link_libraries(test_c_api finkit finkit_headers)

add_test(NAME test_c_api COMMAND test_c_api)

add_executable(test_c_numeric_contract test_c_numeric_contract.c)
target_link_libraries(test_c_numeric_contract finkit finkit_headers)

add_test(NAME test_c_numeric_contract COMMAND test_c_numeric_contract)
