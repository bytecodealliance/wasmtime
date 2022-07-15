cmake_minimum_required(VERSION 3.10)
project(wasmtime-examples)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../crates/c-api ${CMAKE_CURRENT_BINARY_DIR}/wasmtime)

function(CREATE_TARGET TARGET TARGET_PATH)
	add_executable(wasmtime-${TARGET} ${TARGET_PATH})

	if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
		target_compile_options(wasmtime-${TARGET} PRIVATE -Wall -Wextra -Wno-deprecated-declarations)
	elseif(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
		target_compile_options(wasmtime-${TARGET} PRIVATE /W4)
	endif()

	set_target_properties(wasmtime-${TARGET} PROPERTIES
		OUTPUT_NAME wasmtime-${TARGET}
		RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/$<0:>
		CXX_VISIBILITY_PRESET hidden
		POSITION_INDEPENDENT_CODE ON)

	target_include_directories(wasmtime-${TARGET} PUBLIC wasmtime)
	target_link_libraries(wasmtime-${TARGET} PUBLIC wasmtime)
endfunction()

# Add all examples
create_target(externref externref.c)
create_target(fib-debug fib-debug/main.c)
create_target(fuel fuel.c)
create_target(gcd gcd.c)
create_target(hello hello.c)
create_target(interrupt interrupt.c)
create_target(linking linking.c)
create_target(memory memory.c)
create_target(multi multi.c)
create_target(multimemory multimemory.c)
create_target(serialize serialize.c)
create_target(threads threads.c)
create_target(tokio tokio/main.c)
create_target(wasi wasi/main.c)
