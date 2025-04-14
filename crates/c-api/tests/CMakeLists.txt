include(FetchContent)
FetchContent_Declare(
  googletest
  URL https://github.com/google/googletest/archive/refs/tags/release-1.12.1.zip
)
# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

include(GoogleTest)

function(add_capi_test name)
  add_executable(test-${name} ${name}.cc)
  target_link_libraries(test-${name} PRIVATE wasmtime-cpp gtest_main)
  gtest_discover_tests(test-${name})
endfunction()

add_capi_test(simple)
add_capi_test(types)
add_capi_test(func)

# Add a custom test where two files include `wasmtime.hh` and are compiled into
# the same executable (basically makes sure any defined functions in the header
# are tagged with `inline`).
add_executable(test-double-include double-include-a.cc double-include-b.cc)
target_link_libraries(test-double-include PRIVATE wasmtime-cpp gtest_main)
gtest_discover_tests(test-double-include)
