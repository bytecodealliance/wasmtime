#include <wasmtime/types/func.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(FuncType, Smoke) {
  FuncType t({}, {});
  EXPECT_EQ(t->params().size(), 0);
  EXPECT_EQ(t->results().size(), 0);

  auto other = t;
  other = t;

  FuncType t2({ValKind::I32}, {ValKind::I64});
  EXPECT_EQ(t2->params().size(), 1);
  for (auto ty : t2->params()) {
    EXPECT_EQ(ty.kind(), ValKind::I32);
  }
  EXPECT_EQ(t2->results().size(), 1);
  for (auto ty : t2->results()) {
    EXPECT_EQ(ty.kind(), ValKind::I64);
  }
}
