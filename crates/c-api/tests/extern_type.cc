#include <wasmtime/types/extern.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(ExternType, Smoke) {
  Engine engine;

  Module module = Module::compile(engine, "(module"
                                   "(import \"a\" \"b\" (func))"
                                   "(global (export \"x\") i32 (i32.const 0))"
                                   ")").unwrap();

  auto imports = module.imports();
  EXPECT_EQ(imports.size(), 1);
  auto i = *imports.begin();
  auto import_ty = std::get<FuncType::Ref>(ExternType::from_import(i));
  EXPECT_EQ(import_ty.params().size(), 0);
  EXPECT_EQ(import_ty.results().size(), 0);

  auto exports = module.exports();
  EXPECT_EQ(exports.size(), 1);
  auto e = *exports.begin();
  auto export_ty = std::get<GlobalType::Ref>(ExternType::from_export(e));
  EXPECT_EQ(export_ty.content().kind(), ValKind::I32);
  EXPECT_FALSE(export_ty.is_mutable());
}
