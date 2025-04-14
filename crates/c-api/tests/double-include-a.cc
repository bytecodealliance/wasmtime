#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Store, WorksInFileA) {
  Engine engine;
  Store store(engine);
}
