#include <wasmtime/types/table.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(TableType, Simple) {
  TableType ty(ValType::funcref(), 1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_EQ(ty->element(), ValType::funcref());

  ty = TableType(ValType::externref(), 2, 3);
  EXPECT_EQ(ty->min(), 2);
  EXPECT_EQ(ty->max(), 3);
  EXPECT_EQ(ty->element(), ValType::externref());
}
