cmake_minimum_required(VERSION 3.0)
project(SignalHandlers CXX)

if( CMAKE_SIZEOF_VOID_P EQUAL 8 )
  set(CMAKE_CXX_FLAGS "-std=c++11 -fno-exceptions -fno-rtti -fPIC")
else( CMAKE_SIZEOF_VOID_P EQUAL 8 )
  set(CMAKE_CXX_FLAGS "-m32 -std=c++11 -fno-exceptions -fno-rtti -fPIC")
endif( CMAKE_SIZEOF_VOID_P EQUAL 8 )

add_library(SignalHandlers STATIC SignalHandlers.cpp)

install(TARGETS SignalHandlers DESTINATION .)
