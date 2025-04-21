#include <wasmtime/types/global.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(GlobalType, Simple) {
  GlobalType ty(ValKind::I32, false);
  EXPECT_FALSE(ty->is_mutable());
  EXPECT_EQ(ty->content().kind(), ValKind::I32);

  ty = GlobalType(ValKind::V128, true);
  EXPECT_TRUE(ty->is_mutable());
  EXPECT_EQ(ty->content().kind(), ValKind::V128);
}
