#include <wasmtime/types/global.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(GlobalType, Simple) {
  GlobalType ty(ValType::i32(), false);
  EXPECT_FALSE(ty->is_mutable());
  EXPECT_EQ(ty->content(), ValType::i32());

  ty = GlobalType(ValType::v128(), true);
  EXPECT_TRUE(ty->is_mutable());
  EXPECT_EQ(ty->content(), ValType::v128());
}

TEST(GlobalType, CopyAssign) {
  GlobalType a(ValType::i64(), true);
  GlobalType b(ValType::i32(), false);
  b = a;
  EXPECT_TRUE(b->is_mutable());
  EXPECT_EQ(b->content(), ValType::i64());
}
