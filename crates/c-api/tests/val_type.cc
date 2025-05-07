#include <wasmtime/types/val.hh>

#include <gtest/gtest.h>
#include <sstream>

using namespace wasmtime;

TEST(ValType, Simple) {
  ValType ty(ValKind::I32);
  EXPECT_EQ(ty->kind(), ValKind::I32);
  ty = ValKind::I64;
  EXPECT_EQ(ty->kind(), ValKind::I64);
  ty = ValKind::F32;
  EXPECT_EQ(ty->kind(), ValKind::F32);
  ty = ValKind::F64;
  EXPECT_EQ(ty->kind(), ValKind::F64);
  ty = ValKind::ExternRef;
  EXPECT_EQ(ty->kind(), ValKind::ExternRef);
  ty = ValKind::FuncRef;
  EXPECT_EQ(ty->kind(), ValKind::FuncRef);
  ty = ValKind::V128;
  EXPECT_EQ(ty->kind(), ValKind::V128);
}

TEST(ValKind, String) {
  std::stringstream x;
  x << ValKind::I32;
  EXPECT_EQ(x.str(), "i32");
  x << ValKind::F32;
  EXPECT_EQ(x.str(), "i32f32");
}
