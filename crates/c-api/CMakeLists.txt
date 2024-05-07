cmake_minimum_required(VERSION 3.10)
project(wasmtime C)

set(WASMTIME_USER_CARGO_BUILD_OPTIONS "" CACHE STRING "Additional cargo flags (such as --features) to apply to the build command")
option(BUILD_SHARED_LIBS "Build using shared libraries" OFF)
option(WASMTIME_ALWAYS_BUILD "If cmake should always invoke cargo to build wasmtime" ON)
option(WASMTIME_FASTEST_RUNTIME "Set flags designed to optimize runtime performance" OFF)

if(WASMTIME_FASTEST_RUNTIME)
	set(WASMTIME_BUILD_TYPE_FLAG "--profile=fastest-runtime")
	set(WASMTIME_BUILD_TYPE "fastest-runtime")
	set(CARGO_PROFILE_PANIC CARGO_PROFILE_RELEASE_PANIC)
else()
	if(CMAKE_BUILD_TYPE STREQUAL "Release" OR
	    CMAKE_BUILD_TYPE STREQUAL "MinSizeRel" OR
	    CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
		set(WASMTIME_BUILD_TYPE_FLAG "--release")
		set(WASMTIME_BUILD_TYPE "release")
		set(CARGO_PROFILE_PANIC CARGO_PROFILE_RELEASE_PANIC)
	else()
		set(WASMTIME_BUILD_TYPE "debug")
		set(CARGO_PROFILE_PANIC CARGO_PROFILE_DEBUG_PANIC)
    endif()
endif()

if(ANDROID)
	# TODO wasmtime only supports arm64-v8a right now
	if(ANDROID_ABI STREQUAL "armeabi-v7a")
		set(ANDROID_TARGET "armv7-linux-androideabi")
		set(ANDROID_ARCH_SHORT "arm")
	elseif(ANDROID_ABI STREQUAL "arm64-v8a")
		set(ANDROID_TARGET "aarch64-linux-android")
		set(ANDROID_ARCH_SHORT "aarch64")
	elseif(ANDROID_ABI STREQUAL "x86")
		set(ANDROID_TARGET "i686-linux-android")
		set(ANDROID_ARCH_SHORT "i386")
	elseif(ANDROID_ABI STREQUAL "x86_64")
		set(ANDROID_TARGET "x86_64-linux-android")
		set(ANDROID_ARCH_SHORT "x86_64")
	endif()

	set(WASMTIME_BUILD_TARGET "--target=${ANDROID_TARGET}")
endif()

if (BUILD_SHARED_LIBS AND ANDROID)
	message(FATAL_ERROR "Wasmtime cannot be built with BUILD_SHARED_LIBS on Android")
endif()

if(BUILD_SHARED_LIBS)
	if(WIN32)
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${WASMTIME_BUILD_TYPE}/wasmtime.dll.lib)
	elseif(APPLE)
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${WASMTIME_BUILD_TYPE}/libwasmtime.dylib)
	else()
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${WASMTIME_BUILD_TYPE}/libwasmtime.so)
	endif()
else()
	if(WIN32)
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${WASMTIME_BUILD_TYPE}/wasmtime.lib)
	elseif(ANDROID)
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${ANDROID_TARGET}/${WASMTIME_BUILD_TYPE}/libwasmtime.a)
	else()
		set(WASMTIME_BUILD_PRODUCT
			${CMAKE_CURRENT_SOURCE_DIR}/../../target/${WASMTIME_BUILD_TYPE}/libwasmtime.a)
	endif()
endif()

if(ANDROID)
	# Rust attempts to use libgcc.a on NDK versions r23-beta3 and up
	# but it has been replaced with libunwind.a (rust-lang/rust#85806)
	file(WRITE ${CMAKE_BINARY_DIR}/libgcc.a "INPUT(-lunwind)")
	# The version of the clang compiler is part of the libunwind.a path
	file(STRINGS ${ANDROID_TOOLCHAIN_ROOT}/AndroidVersion.txt CLANG_VERSION_FILE)
	list(GET CLANG_VERSION_FILE 0 CLANG_VERSION)

	# Some crates use the compiler directly, environment variables
	# are set to make them use the Android compiler
	set(WASMTIME_PREBUILD_COMMAND ${CMAKE_COMMAND} -E env
	CC=${ANDROID_TOOLCHAIN_ROOT}/bin/clang
	AR=${ANDROID_TOOLCHAIN_ROOT}/bin/llvm-ar
	"RUSTFLAGS=-L ${CMAKE_SYSROOT}/usr/lib/${ANDROID_TARGET}/${ANDROID_NATIVE_API_LEVEL} \
		-L ${ANDROID_TOOLCHAIN_ROOT}/lib64/clang/${CLANG_VERSION}/lib/linux/${ANDROID_ARCH_SHORT} \
		-L ${CMAKE_BINARY_DIR} -C linker=${ANDROID_TOOLCHAIN_ROOT}/bin/ld")
endif()
include(ExternalProject)
find_program(WASMTIME_CARGO_BINARY cargo)
if(NOT WASMTIME_CARGO_BINARY)
    message(FATAL_ERROR [["cargo" was not found. Ensure "cargo" is in PATH. Aborting...]])
endif()
ExternalProject_Add(
	wasmtime-crate
	DOWNLOAD_COMMAND ""
	CONFIGURE_COMMAND ""
	INSTALL_COMMAND "${WASMTIME_INSTALL_COMMAND}"
        BUILD_COMMAND
            ${CMAKE_COMMAND} -E env ${CARGO_PROFILE_PANIC}=abort
            ${WASMTIME_PREBUILD_COMMAND} ${WASMTIME_CARGO_BINARY} build ${WASMTIME_BUILD_TYPE_FLAG} ${WASMTIME_USER_CARGO_BUILD_OPTIONS} ${WASMTIME_BUILD_TARGET}
	USES_TERMINAL_BUILD TRUE
	BINARY_DIR ${CMAKE_CURRENT_SOURCE_DIR}/artifact
        BUILD_ALWAYS ${WASMTIME_ALWAYS_BUILD}
	BUILD_BYPRODUCTS ${WASMTIME_BUILD_PRODUCT})
add_library(wasmtime INTERFACE)
add_dependencies(wasmtime wasmtime-crate)

if(BUILD_SHARED_LIBS)
	if(NOT WIN32)
		set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,-rpath='$ORIGIN'")
	endif()
	target_link_libraries(wasmtime INTERFACE ${WASMTIME_BUILD_PRODUCT})
else()
	if(WIN32)
		target_compile_options(wasmtime INTERFACE -DWASM_API_EXTERN= -DWASI_API_EXTERN=)
		target_link_libraries(wasmtime INTERFACE ${WASMTIME_BUILD_PRODUCT}
			ws2_32 advapi32 userenv ntdll shell32 ole32 bcrypt)
	elseif(APPLE OR ANDROID)
		target_link_libraries(wasmtime INTERFACE ${WASMTIME_BUILD_PRODUCT})
	else()
		target_link_libraries(wasmtime INTERFACE ${WASMTIME_BUILD_PRODUCT}
			pthread dl m)
	endif()
endif()

target_include_directories(wasmtime INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)

include(GNUInstallDirs)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/include/wasmtime.h
	${CMAKE_CURRENT_SOURCE_DIR}/include/wasi.h
	${CMAKE_CURRENT_SOURCE_DIR}/include/doc-wasm.h
	${CMAKE_CURRENT_SOURCE_DIR}/include/wasm.h
	${CMAKE_CURRENT_SOURCE_DIR}/include/wasm.hh
	DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/include/wasmtime
	DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES ${WASMTIME_BUILD_PRODUCT}
	DESTINATION ${CMAKE_INSTALL_LIBDIR})
