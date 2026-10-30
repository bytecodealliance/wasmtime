#include <wasmtime/types/func.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(FuncType, Smoke) {
  FuncType t({}, {});
  EXPECT_EQ(t->params().size(), 0);
  EXPECT_EQ(t->results().size(), 0);

  auto other = t;
  other = t;

  FuncType t2({ValType::i32()}, {ValType::i64()});
  EXPECT_EQ(t2->params().size(), 1);
  for (auto ty : t2->params()) {
    EXPECT_EQ(ty, ValType::i32());
  }
  EXPECT_EQ(t2->results().size(), 1);
  for (auto ty : t2->results()) {
    EXPECT_EQ(ty, ValType::i64());
  }
}

TEST(FuncType, CopyAssign) {
  FuncType a({ValType::i32(), ValType::i64()}, {ValType::f32()});
  FuncType b({}, {});
  b = a;
  EXPECT_EQ(b->params().size(), 2);
  EXPECT_EQ(b->results().size(), 1);
}
