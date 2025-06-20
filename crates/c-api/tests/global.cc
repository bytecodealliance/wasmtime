#include <wasmtime/global.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Global, Smoke) {
  Engine engine;
  Store store(engine);
  Global::create(store, GlobalType(ValKind::I32, true), 3.0).err();
  Global::create(store, GlobalType(ValKind::I32, true), 3).unwrap();
  Global::create(store, GlobalType(ValKind::I32, false), 3).unwrap();

  Global g = Global::create(store, GlobalType(ValKind::I32, true), 4).unwrap();
  EXPECT_EQ(g.get(store).i32(), 4);
  g.set(store, 10).unwrap();
  EXPECT_EQ(g.get(store).i32(), 10);
  g.set(store, 10.23).err();
  EXPECT_EQ(g.get(store).i32(), 10);

  EXPECT_EQ(g.type(store)->content().kind(), ValKind::I32);
  EXPECT_TRUE(g.type(store)->is_mutable());
}
