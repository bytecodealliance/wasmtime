cmake_minimum_required(VERSION 3.0)
project(SignalHandlers CXX)

set(CMAKE_CXX_FLAGS "-std=c++11 -fno-exceptions -fno-rtti -fPIC")

add_library(SignalHandlers STATIC SignalHandlers.cpp)

install(TARGETS SignalHandlers DESTINATION .)
