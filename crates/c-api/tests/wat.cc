#include <wasmtime/wat.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(wat2wasm, Simple) {
  EXPECT_TRUE(wat2wasm("(module)"));
  EXPECT_FALSE(wat2wasm("not a module"));
}
