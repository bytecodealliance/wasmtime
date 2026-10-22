#include <wasmtime.hh>
#include <wasmtime/types/table.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(TableType, Simple) {
  TableType ty(ValType::funcref(), 1);
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_EQ(ty->element(), ValType::funcref());

  ty = TableType(ValType::externref(), 2, 3);
  EXPECT_EQ(ty->min(), 2);
  EXPECT_EQ(ty->max(), 3);
  EXPECT_EQ(ty->element(), ValType::externref());
}

TEST(TableType, Table64) {
  Engine engine;
  Module module =
      Module::compile(
          engine,
          "(module"
          "(import \"\" \"t\" (table i64 0x100000000 0x200000000 funcref))"
          ")")
          .unwrap();
  auto import = *module.imports().begin();
  auto ty = std::get<TableType::Ref>(ExternType::from_import(import));
  EXPECT_EQ(ty.min(), 0x100000000);
  EXPECT_EQ(ty.max(), 0x200000000);
  EXPECT_TRUE(ty.is_64());
}
