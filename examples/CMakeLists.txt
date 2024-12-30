cmake_minimum_required(VERSION 3.10)
project(wasmtime-examples)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_STANDARD 11)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../crates/c-api ${CMAKE_CURRENT_BINARY_DIR}/wasmtime)

function(CREATE_TARGET TARGET TARGET_PATH)
	add_executable(wasmtime-${TARGET} ${TARGET_PATH})

	if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
		target_compile_options(wasmtime-${TARGET} PRIVATE -Wall -Wextra -Wno-deprecated-declarations)
	elseif(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
		target_compile_options(wasmtime-${TARGET} PRIVATE /W3)
	endif()

        target_compile_definitions(wasmtime-${TARGET} PRIVATE WASMTIME_TEST_ONLY)

	set_target_properties(wasmtime-${TARGET} PROPERTIES
		OUTPUT_NAME wasmtime-${TARGET}
		RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/$<0:>
		CXX_VISIBILITY_PRESET hidden
		POSITION_INDEPENDENT_CODE ON)

	target_include_directories(wasmtime-${TARGET} PUBLIC wasmtime)
	target_link_libraries(wasmtime-${TARGET} PUBLIC wasmtime)
	if(APPLE)
		target_link_libraries(wasmtime-${TARGET} PRIVATE "-framework CoreFoundation")
	endif()
	add_test(NAME ${TARGET}-c COMMAND $<TARGET_FILE:wasmtime-${TARGET}> WORKING_DIRECTORY ../..)
endfunction()

function(CREATE_RUST_TEST EXAMPLE)
	if(ARGC GREATER 1)
		add_test(NAME ${EXAMPLE}-rust COMMAND cargo run --example ${EXAMPLE} --features ${ARGV1} WORKING_DIRECTORY ../..)
	else()
		add_test(NAME ${EXAMPLE}-rust COMMAND cargo run --example ${EXAMPLE} WORKING_DIRECTORY ../..)
	endif()
endfunction()
function(CREATE_RUST_WASM EXAMPLE TARGET)
	execute_process(COMMAND cargo build -p example-${EXAMPLE}-wasm --target ${TARGET})
endfunction()

# Enable testing
enable_testing()

# Add all examples
create_target(anyref anyref.c)
create_target(async async.cpp)
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
create_target(wasi wasi/main.c)

# Add rust tests
create_rust_test(anyref)
create_rust_wasm(fib-debug wasm32-unknown-unknown)
create_rust_wasm(tokio wasm32-wasip1)
create_rust_wasm(wasi wasm32-wasip1)
create_rust_wasm(component wasm32-unknown-unknown)
create_rust_test(epochs)
create_rust_test(externref)
create_rust_test(fib-debug)
create_rust_test(fuel)
create_rust_test(gcd)
create_rust_test(hello)
create_rust_test(interrupt)
create_rust_test(linking)
create_rust_test(memory)
create_rust_test(multi)
create_rust_test(multimemory)
create_rust_test(serialize)
create_rust_test(threads)
create_rust_test(wasi)
create_rust_test(tokio wasi-common/tokio)
create_rust_test(component)
