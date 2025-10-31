#include <gtest/gtest.h>
#include <wasmtime/types/memory.hh>

using namespace wasmtime;

TEST(MemoryType, Simple) {
  MemoryType ty(1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);
}

TEST(MemoryType, WithMax) {
  MemoryType ty(1, 2);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), 2);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);
}

TEST(MemoryType, Mem64) {
  MemoryType ty = MemoryType::New64(1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_TRUE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);

  ty = MemoryType::New64(1, 2);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), 2);
  EXPECT_TRUE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);
}

TEST(MemoryType, Builder) {
  MemoryType ty = MemoryType::Builder().build().unwrap();
  EXPECT_EQ(ty->min(), 0);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);

  ty =
      MemoryType::Builder().max(4).shared(true).memory64(true).build().unwrap();
  EXPECT_EQ(ty->min(), 0);
  EXPECT_EQ(ty->max(), 4);
  EXPECT_TRUE(ty->is_64());
  EXPECT_TRUE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);

  ty = MemoryType::Builder()
           .min(5)
           .max(500)
           .shared(true)
           .memory64(false)
           .build()
           .unwrap();
  EXPECT_EQ(ty->min(), 5);
  EXPECT_EQ(ty->max(), 500);
  EXPECT_FALSE(ty->is_64());
  EXPECT_TRUE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 16);
  EXPECT_EQ(ty->page_size(), 1 << 16);

  // 1-byte custom page size.
  ty = MemoryType::Builder()
           .min(1 << 16)
           .max(1 << 17)
           .page_size_log2(0)
           .build()
           .unwrap();
  EXPECT_EQ(ty->min(), 1 << 16);
  EXPECT_EQ(ty->max(), 1 << 17);
  EXPECT_FALSE(ty->is_64());
  EXPECT_FALSE(ty->is_shared());
  EXPECT_EQ(ty->page_size_log2(), 0);
  EXPECT_EQ(ty->page_size(), 1);

  // Invalid custom page size.
  EXPECT_FALSE(MemoryType::Builder().min(1).page_size_log2(4).build());
}
