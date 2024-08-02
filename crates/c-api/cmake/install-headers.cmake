cmake_minimum_required(VERSION 3.12)

include(${CMAKE_CURRENT_LIST_DIR}/features.cmake)

set(dst "${CMAKE_INSTALL_PREFIX}/include")
message(STATUS "dst: ${dst}")
set(include_src "${CMAKE_CURRENT_LIST_DIR}/../include")

message(STATUS "Installing: ${dst}/wasmtime/conf.h")
file(READ "${include_src}/wasmtime/conf.h.in" conf_h)
file(CONFIGURE OUTPUT "${dst}/wasmtime/conf.h" CONTENT "${conf_h}")
file(INSTALL "${include_src}/"
     DESTINATION "${dst}"
     FILES_MATCHING REGEX "\\.hh?$")
