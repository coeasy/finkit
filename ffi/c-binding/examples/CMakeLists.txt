cmake_minimum_required(VERSION 3.12)

project(finkit_examples CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Reuse the parent build targets when included from the source tree.
# Only resolve an installed SDK when this directory is configured standalone.
if(NOT TARGET finkit OR NOT TARGET finkit_headers)
    find_package(finkit CONFIG REQUIRED)
endif()

add_executable(example example.cpp)
target_link_libraries(example finkit finkit_headers)
