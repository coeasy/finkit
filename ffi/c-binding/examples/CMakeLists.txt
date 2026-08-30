cmake_minimum_required(VERSION 3.12)

project(alpha_ta_examples CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Find the parent project
find_package(alpha_ta REQUIRED PATHS "${CMAKE_CURRENT_SOURCE_DIR}/..")

add_executable(example example.cpp)
target_link_libraries(example alpha_ta alpha_ta_headers)
