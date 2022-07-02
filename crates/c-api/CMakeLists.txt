cmake_minimum_required(VERSION 3.10)

option(BUILD_SHARED_LIBS "Build using shared libraries" OFF)

if (BUILD_SHARED_LIBS)
	# Copy shared library into build directory
	if(WIN32)
		set(WASMTIME_INSTALL_COMMAND ${CMAKE_COMMAND} -E copy_if_different
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/wasmtime.dll
			${CMAKE_BINARY_DIR})
	elseif(APPLE)
		set(WASMTIME_INSTALL_COMMAND ${CMAKE_COMMAND} -E copy_if_different
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.dylib
			${CMAKE_BINARY_DIR})
	else()
		set(WASMTIME_INSTALL_COMMAND ${CMAKE_COMMAND} -E copy_if_different
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.so
			${CMAKE_BINARY_DIR})
	endif()
endif()

include(ExternalProject)
ExternalProject_Add(
	wasmtime-crate
	DOWNLOAD_COMMAND ""
	CONFIGURE_COMMAND ""
	INSTALL_COMMAND "${WASMTIME_INSTALL_COMMAND}"
	BUILD_COMMAND cargo build --release
	BINARY_DIR ${CMAKE_CURRENT_SOURCE_DIR}
	BUILD_ALWAYS ON)
add_library(wasmtime INTERFACE)
add_dependencies(wasmtime wasmtime-crate)

if (BUILD_SHARED_LIBS)
	if(WIN32)
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/wasmtime.dll.lib)
	elseif(APPLE)
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.dylib)
		set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,-rpath='$ORIGIN'")
	else()
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.so)
		set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,-rpath='$ORIGIN'")
	endif()
else()
	if(WIN32)
		target_compile_options(wasmtime INTERFACE -DWASM_API_EXTERN= -DWASI_API_EXTERN=)
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/wasmtime.lib
			ws2_32 advapi32 userenv ntdll shell32 ole32 bcrypt)
	elseif(APPLE)
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.a)
	else()
		target_link_libraries(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwasmtime.a
			pthread dl m)
	endif()
endif()

target_include_directories(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/wasm-c-api/include)