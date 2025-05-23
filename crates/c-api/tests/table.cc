#include <wasmtime/table.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Table, Smoke) {
  Engine engine;
  Store store(engine);
  Table::create(store, TableType(ValKind::FuncRef, 1), 3.0).err();

  Val null = std::optional<Func>();
  Table t = Table::create(store, TableType(ValKind::FuncRef, 1), null).unwrap();
  EXPECT_FALSE(t.get(store, 1));
  EXPECT_TRUE(t.get(store, 0));
  Val val = *t.get(store, 0);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
  EXPECT_FALSE(val.funcref());
  EXPECT_EQ(t.grow(store, 4, null).unwrap(), 1);
  t.set(store, 3, null).unwrap();
  t.set(store, 3, 3).err();
  EXPECT_EQ(t.size(store), 5);
  EXPECT_EQ(t.type(store)->element().kind(), ValKind::FuncRef);
}
