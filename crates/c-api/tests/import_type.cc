#include <wasmtime/types/import.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(ImportType, Smoke) {
  Engine engine;
  Module module = Module::compile(engine, "(module)").unwrap();
  EXPECT_EQ(module.imports().size(), 0);

  module = Module::compile(engine, "(module"
                                   "(import \"a\" \"b\" (func))"
                                   ")").unwrap();

  auto imports = module.imports();
  EXPECT_EQ(imports.size(), 1);
  auto i = *imports.begin();
  EXPECT_EQ(i.module(), "a");
  EXPECT_EQ(i.name(), "b");
  auto import_ty = std::get<FuncType::Ref>(ExternType::from_import(i));
  EXPECT_EQ(import_ty.params().size(), 0);
  EXPECT_EQ(import_ty.results().size(), 0);

  for (auto &imp : imports) {
  }

  auto other_imports = module.imports();
  other_imports = std::move(imports);
  ImportType::List last_imports(std::move(other_imports));
}
