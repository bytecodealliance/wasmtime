#include <wasmtime/wasi.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(WasiConfig, Smoke) {
  WasiConfig config;
  config.argv({"x"});
  config.inherit_argv();
  config.env({{"x", "y"}});
  config.inherit_env();
  EXPECT_FALSE(config.stdin_file("nonexistent"));
  config.inherit_stdin();
  EXPECT_FALSE(config.stdout_file("path/to/nonexistent"));
  config.inherit_stdout();
  EXPECT_FALSE(config.stderr_file("path/to/nonexistent"));
  config.inherit_stderr();

  WasiConfig config2;
  if (config2.preopen_dir("nonexistent", "nonexistent", 0, 0)) {
    Engine engine;
    Store store(engine);
    EXPECT_FALSE(store.context().set_wasi(std::move(config2)));
  }
}
