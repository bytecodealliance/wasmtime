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
  EXPECT_EQ(m.page_size_log2(store), 16);
  EXPECT_EQ(m.page_size(store), 1 << 16);
}

TEST(Memory, Discard) {
  Engine engine;
  Store store(engine);
  Memory m = Memory::create(store, MemoryType(2)).unwrap();
  for (auto &byte : m.data(store)) {
    byte = 0xa5;
  }

  m.discard(store, 1, 1).unwrap();
  auto data = m.data(store);
  EXPECT_EQ(data[0], 0xa5);
  EXPECT_EQ(data[(1 << 16) - 1], 0xa5);
  EXPECT_EQ(data[1 << 16], 0);
  EXPECT_EQ(data[(2 << 16) - 1], 0);
  EXPECT_EQ(m.size(store), 2);

  // Out of bounds, and the memory is unchanged.
  EXPECT_FALSE(m.discard(store, 1, 2));
  EXPECT_EQ(m.data(store)[0], 0xa5);
}

TEST(Memory, OneBytePageSize) {
  Engine engine;
  Store store(engine);
  MemoryType mem_ty =
      MemoryType::Builder().min(1).max(2).page_size_log2(0).build().unwrap();
  Memory mem = Memory::create(store, mem_ty).unwrap();

  // Has expected page size and limits.
  EXPECT_EQ(mem.page_size_log2(store), 0);
  EXPECT_EQ(mem.page_size(store), 1);
  EXPECT_EQ(mem.type(store)->min(), 1);
  EXPECT_EQ(*mem.type(store)->max(), 2);

  // Has expected initial size.
  EXPECT_EQ(mem.size(store), 1);

  // Can grow to max and has expected new size.
  EXPECT_EQ(mem.grow(store, 1).unwrap(), 1);
  EXPECT_EQ(mem.size(store), 2);
  EXPECT_EQ(mem.data(store).size(), 2);

  // Fails to grow beyond max, size remains unchanged.
  EXPECT_FALSE(mem.grow(store, 1));
  EXPECT_EQ(mem.size(store), 2);
  EXPECT_EQ(mem.data(store).size(), 2);
}
