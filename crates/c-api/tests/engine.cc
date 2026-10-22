#include <wasmtime/engine.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(Engine, Simple) {
  Engine engine;
  engine = Engine(Config());
  engine.increment_epoch();

  Engine engine2 = engine;
  engine2 = Engine();
  engine.is_pulley();
}

TEST(Engine, InvalidConfig) {
  wasm_config_t *config = wasm_config_new();
  wasmtime_config_cranelift_flag_set(config, "no_such_flag", "true");
  EXPECT_EQ(wasm_engine_new_with_config(config), nullptr);
}
