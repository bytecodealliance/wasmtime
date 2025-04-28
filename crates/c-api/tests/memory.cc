#include <wasmtime/memory.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Memory, Smoke) {
  Engine engine;
  Store store(engine);
  Memory m = Memory::create(store, MemoryType(1)).unwrap();
  EXPECT_EQ(m.size(store), 1);
  EXPECT_EQ(m.grow(store, 1).unwrap(), 1);
  EXPECT_EQ(m.data(store).size(), 2 << 16);
  EXPECT_EQ(m.type(store)->min(), 1);
}
