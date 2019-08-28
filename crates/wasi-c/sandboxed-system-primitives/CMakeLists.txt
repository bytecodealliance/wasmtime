cmake_minimum_required(VERSION 3.0)
project(SandboxedSystemPrimitives C)

include_directories(include)

add_library(SandboxedSystemPrimitives STATIC src/posix.c src/random.c src/str.c)

install(TARGETS SandboxedSystemPrimitives DESTINATION .)
