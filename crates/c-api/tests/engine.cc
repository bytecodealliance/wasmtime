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
