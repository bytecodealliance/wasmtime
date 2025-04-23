#include <wasmtime/val.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(Val, Smoke) {
  Val val(1);
  EXPECT_EQ(val.kind(), ValKind::I32);
  EXPECT_EQ(val.i32(), 1);

  val = (int32_t)3;
  EXPECT_EQ(val.kind(), ValKind::I32);
  EXPECT_EQ(val.i32(), 3);

  val = (int64_t)4;
  EXPECT_EQ(val.kind(), ValKind::I64);
  EXPECT_EQ(val.i64(), 4);

  val = (float)5;
  EXPECT_EQ(val.kind(), ValKind::F32);
  EXPECT_EQ(val.f32(), 5);

  val = (double)6;
  EXPECT_EQ(val.kind(), ValKind::F64);
  EXPECT_EQ(val.f64(), 6);

  val = V128();
  EXPECT_EQ(val.kind(), ValKind::V128);
  for (int i = 0; i < 16; i++) {
    EXPECT_EQ(val.v128().v128[i], 0);
  }

  Engine engine;
  Store store(engine);
  val = std::optional<ExternRef>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(val.externref(store), std::nullopt);

  val = std::optional<ExternRef>(ExternRef(store, 5));
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(std::any_cast<int>(val.externref(store)->data(store)), 5);

  val = ExternRef(store, 5);
  EXPECT_EQ(val.kind(), ValKind::ExternRef);
  EXPECT_EQ(std::any_cast<int>(val.externref(store)->data(store)), 5);

  val = std::optional<AnyRef>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref(store), std::nullopt);

  val = std::optional<AnyRef>(AnyRef::i31(store, 5));
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref(store)->i31(store), 5);
  EXPECT_EQ(val.anyref(store)->u31(store), 5);

  val = AnyRef::i31(store, -5);
  EXPECT_EQ(val.kind(), ValKind::AnyRef);
  EXPECT_EQ(val.anyref(store)->i31(store), -5);
  EXPECT_EQ(val.anyref(store)->u31(store), 0x7ffffffb);

  val = std::optional<Func>(std::nullopt);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
  EXPECT_EQ(val.funcref(), std::nullopt);

  Func func(
      store, FuncType({}, {}),
      [](auto caller, auto params, auto results) -> auto{
        return std::monostate();
      });

  val = std::optional<Func>(func);
  EXPECT_EQ(val.kind(), ValKind::FuncRef);

  val = func;
  EXPECT_EQ(val.kind(), ValKind::FuncRef);
}
