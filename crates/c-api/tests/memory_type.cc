#include <wasmtime/types/memory.hh>
#include <gtest/gtest.h>

using namespace wasmtime;

TEST(MemoryType, Simple) {
  MemoryType ty(1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
}

TEST(MemoryType, WithMax) {
  MemoryType ty(1, 2);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), 2);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
}

TEST(MemoryType, Mem64) {
  MemoryType ty = MemoryType::New64(1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_TRUE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());

  ty = MemoryType::New64(1, 2);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), 2);
  EXPECT_TRUE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
}

TEST(MemoryType, Builder) {
  MemoryType ty = MemoryType::Builder().build();
  EXPECT_EQ(ty->min(), 0);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());

  ty = MemoryType::Builder().max(4).shared(true).memory64(true).build();
  EXPECT_EQ(ty->min(), 0);
  EXPECT_EQ(ty->max(), 4);
  EXPECT_TRUE(ty->is_64());
  EXPECT_TRUE(ty->is_shared());

  ty = MemoryType::Builder().min(5).max(500).shared(true).memory64(false).build();
  EXPECT_EQ(ty->min(), 5);
  EXPECT_EQ(ty->max(), 500);
  EXPECT_FALSE(ty->is_64());
  EXPECT_TRUE(ty->is_shared());

}
