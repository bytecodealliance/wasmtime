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
  cmake_parse_arguments(PARSE_ARGV 1 arg "" "" "FILES")
  add_executable(test-${name} ${arg_FILES})
  target_link_libraries(test-${name} PRIVATE wasmtime-cpp gtest_main)
  gtest_discover_tests(test-${name}
    # GitHub Actions on Windows is pretty slow, let's give it lots more time
    # than the default 5 seconds.
    DISCOVERY_TIMEOUT 60)
endfunction()

add_capi_test(tests FILES
  simple.cc
  types.cc
  memory_type.cc
  val_type.cc
  global_type.cc
  table_type.cc
  func_type.cc
  import_type.cc
  export_type.cc
  extern_type.cc
  func.cc
  component/instantiate.cc
  error.cc
  config.cc
  wat.cc
  module.cc
  engine.cc
  trap.cc
)

# Add a custom test where two files include `wasmtime.hh` and are compiled into
# the same executable (basically makes sure any defined functions in the header
# are tagged with `inline`).
add_capi_test(test-double-include FILES double-include-a.cc double-include-b.cc)
