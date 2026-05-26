#include <wasmtime/types/val.hh>

#include <gtest/gtest.h>
#include <sstream>

using namespace wasmtime;

TEST(ValType, Simple) {
  ValType ty = ValType::i32();
  EXPECT_EQ(ty, ValType::i32());
  ty = ValType::i64();
  EXPECT_EQ(ty, ValType::i64());
  ty = ValType::f32();
  EXPECT_EQ(ty, ValType::f32());
  ty = ValType::f64();
  EXPECT_EQ(ty, ValType::f64());
  ty = ValType::externref();
  EXPECT_EQ(ty, ValType::externref());
  ty = ValType::funcref();
  EXPECT_EQ(ty, ValType::funcref());
  ty = ValType::v128();
  EXPECT_EQ(ty, ValType::v128());
}

TEST(ValKind, String) {
  std::stringstream x;
  x << ValType::i32();
  EXPECT_EQ(x.str(), "i32");
  x << ValType::f32();
  EXPECT_EQ(x.str(), "i32f32");
}

TEST(HeapType, Smoke) {
  EXPECT_TRUE(HeapType::extern_().is_extern());
  EXPECT_TRUE(HeapType::noextern().is_noextern());
  EXPECT_TRUE(HeapType::func().is_func());
  EXPECT_TRUE(HeapType(FuncType({}, {})).as_concrete_func());
  EXPECT_TRUE(HeapType::nofunc().is_nofunc());
  EXPECT_TRUE(HeapType::any().is_any());
  EXPECT_TRUE(HeapType::none().is_none());
  EXPECT_TRUE(HeapType::eq().is_eq());
  EXPECT_TRUE(HeapType::i31().is_i31());
  EXPECT_TRUE(HeapType::array().is_array());
  EXPECT_TRUE(HeapType::struct_().is_struct());
  EXPECT_TRUE(HeapType::exn().is_exn());
  EXPECT_TRUE(HeapType::noexn().is_noexn());

  HeapType hty(FuncType({ValType::i32()}, {ValType::i64()}));
  auto func_ty = hty.as_concrete_func();
  EXPECT_TRUE(func_ty);
  EXPECT_EQ(func_ty->params().size(), 1);
  EXPECT_EQ(*func_ty->params().begin(), ValType::i32());
  EXPECT_EQ(func_ty->results().size(), 1);
  EXPECT_EQ(*func_ty->results().begin(), ValType::i64());

  EXPECT_TRUE(!HeapType::extern_().is_concrete());
  EXPECT_TRUE(HeapType(FuncType({}, {})).is_concrete());
}

TEST(ArrayType, Smoke) {
  Engine engine;
  ArrayType aty(engine, FieldType::mut_(ValType::i32()));
  HeapType hty(aty);
  auto array_ty = hty.as_concrete_array();
  ASSERT_NE(array_ty, nullptr);
  EXPECT_TRUE(array_ty->element_type().is_mutable());

  StorageType sty = array_ty->element_type().storage_type();
  EXPECT_TRUE(sty.as_valtype());
  EXPECT_EQ(*sty.as_valtype(), ValType::i32());

  RefType rty(true, hty);
  rty = RefType(false, hty);
  rty = RefType(false, aty);
  ValType vty(engine, rty);
}

TEST(ExnType, Smoke) {
  Engine engine;
  ExnType ety =
      ExnType::create(engine, {ValType::i32(), ValType::i64()}).unwrap();
  HeapType hty(ety);
  auto exn_ty = hty.as_concrete_exn();
  ASSERT_NE(exn_ty, nullptr);
  EXPECT_TRUE(hty.is_concrete());
  EXPECT_FALSE(HeapType::exn().is_concrete());

  TagType tt = exn_ty->tag_type();
  auto ft = tt->functype();
  EXPECT_EQ(ft->params().size(), 2);
  EXPECT_EQ(*ft->params().begin(), ValType::i32());

  RefType rty(true, hty);
  rty = RefType(false, hty);
  rty = RefType(false, ety);
  ValType vty(engine, rty);

  ety = ExnType::create(engine, {}).unwrap();
  HeapType hty2(ety);
  auto exn_ty2 = hty2.as_concrete_exn();
  ASSERT_NE(exn_ty2, nullptr);
  EXPECT_EQ(exn_ty2->tag_type()->functype()->params().size(), 0);
}

TEST(StructType, Smoke) {
  Engine engine;
  StructType sty(engine, {});
  HeapType hty(sty);
  auto struct_ty = hty.as_concrete_struct();
  ASSERT_NE(struct_ty, nullptr);
  EXPECT_EQ(struct_ty->field(0), std::nullopt);
  EXPECT_EQ(struct_ty->num_fields(), 0);

  RefType rty(true, hty);
  rty = RefType(false, hty);
  rty = RefType(false, sty);
  ValType vty(engine, rty);

  sty = StructType(engine, {FieldType::mut_(StorageType::i8())});
  EXPECT_EQ(sty.field(0)->is_mutable(), true);
  EXPECT_TRUE(sty.field(0)->storage_type().is_i8());
  EXPECT_FALSE(sty.field(0)->storage_type().as_valtype());
  EXPECT_EQ(sty.num_fields(), 1);
}
