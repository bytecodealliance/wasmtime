#include <wasmtime.hh>
#include <wasmtime/types/table.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(TableType, Simple) {
  TableType ty = TableType::create(ValType::funcref(), 1).unwrap();
  EXPECT_EQ(ty->min(), 1);
  EXPECT_EQ(ty->max(), std::nullopt);
  EXPECT_FALSE(ty->is_64());
  EXPECT_EQ(ty->element(), ValType::funcref());

  ty = TableType::create(ValType::externref(), 2, 3).unwrap();
  EXPECT_EQ(ty->min(), 2);
  EXPECT_EQ(ty->max(), 3);
  EXPECT_EQ(ty->element(), ValType::externref());

  TableType other = TableType::create(ValType::funcref(), 1).unwrap();
  other = ty;
  EXPECT_EQ(other->min(), 2);
}

TEST(TableType, Invalid) {
  EXPECT_FALSE(TableType::create(ValType::funcref(), 2, 1));
  EXPECT_FALSE(TableType::create(ValType::i32(), 1));
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
  auto imports = module.imports();
  auto import = *imports.begin();
  auto ty = std::get<TableType::Ref>(ExternType::from_import(import));
  EXPECT_EQ(ty.min(), 0x100000000);
  EXPECT_EQ(ty.max(), 0x200000000);
  EXPECT_TRUE(ty.is_64());
}
