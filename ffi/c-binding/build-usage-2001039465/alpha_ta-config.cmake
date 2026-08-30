
####### Expanded from @PACKAGE_INIT@ by configure_package_config_file() #######
####### Any changes to this file will be overwritten by the next CMake run ####
####### The input file was alpha_ta-config.cmake.in                            ########

get_filename_component(PACKAGE_PREFIX_DIR "${CMAKE_CURRENT_LIST_DIR}/../../../" ABSOLUTE)

macro(set_and_check _var _file)
  set(${_var} "${_file}")
  if(NOT EXISTS "${_file}")
    message(FATAL_ERROR "File or directory ${_file} referenced by variable ${_var} does not exist !")
  endif()
endmacro()

macro(check_required_components _NAME)
  foreach(comp ${${_NAME}_FIND_COMPONENTS})
    if(NOT ${_NAME}_${comp}_FOUND)
      if(${_NAME}_FIND_REQUIRED_${comp})
        set(${_NAME}_FOUND FALSE)
      endif()
    endif()
  endforeach()
endmacro()

####################################################################################

# ----------------------------------------------------------------------------
# alpha_ta CMake package config
#
# Provided by `find_package(alpha_ta CONFIG)` from a downstream project. After
# this file is included, the following imported targets are available:
#
#   alpha_ta::alpha_ta   - umbrella target: alpha_ta dylib + public C/C++ headers
#   alpha_ta::headers  - headers-only target (no library)
# ----------------------------------------------------------------------------

include(CMakeFindDependencyMacro)
include(GNUInstallDirs)

# Re-construct the IMPORTED target at consumer-config time so we don't have to
# ship a separate alpha_ta-targets.cmake (the alpha_ta dylib is itself IMPORTED,
# so we have nothing to export).
if(NOT TARGET alpha_ta)
    if(WIN32)
        set(_alpha_ta_runtime "bin/alpha_ta_ffi.dll")
        set(_alpha_ta_archive "lib/alpha_ta_ffi.dll.lib")
    elseif(APPLE)
        set(_alpha_ta_library "lib/libalpha_ta_ffi.dylib")
    else()
        set(_alpha_ta_library "lib/libalpha_ta_ffi.so")
    endif()

    add_library(alpha_ta SHARED IMPORTED GLOBAL)
    if(WIN32)
        set_target_properties(alpha_ta PROPERTIES
            IMPORTED_LOCATION "${PACKAGE_PREFIX_DIR}/${_alpha_ta_runtime}"
            IMPORTED_IMPLIB   "${PACKAGE_PREFIX_DIR}/${_alpha_ta_archive}"
            INTERFACE_INCLUDE_DIRECTORIES "${PACKAGE_PREFIX_DIR}/include"
        )
    else()
        set_target_properties(alpha_ta PROPERTIES
            IMPORTED_LOCATION "${PACKAGE_PREFIX_DIR}/${_alpha_ta_library}"
            INTERFACE_INCLUDE_DIRECTORIES "${PACKAGE_PREFIX_DIR}/include"
        )
    endif()
endif()

if(NOT TARGET alpha_ta_headers)
    add_library(alpha_ta_headers INTERFACE)
    target_include_directories(alpha_ta_headers INTERFACE
        $<BUILD_INTERFACE:${PACKAGE_PREFIX_DIR}/include>
    )
endif()

if(NOT TARGET alpha_ta::alpha_ta)
    add_library(alpha_ta::alpha_ta INTERFACE IMPORTED)
    set_target_properties(alpha_ta::alpha_ta PROPERTIES
        INTERFACE_LINK_LIBRARIES "alpha_ta;alpha_ta_headers"
    )
endif()

if(NOT TARGET alpha_ta::headers)
    add_library(alpha_ta::headers ALIAS alpha_ta_headers)
endif()

check_required_components(alpha_ta)
