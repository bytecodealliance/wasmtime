#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Store, WorksInFileB) {
  Engine engine;
  Store store(engine);
}
