#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

template <typename T, typename E> T unwrap(Result<T, E> result) {
  if (result) {
    return result.ok();
  }
  std::cerr << "error: " << result.err().message() << "\n";
  std::abort();
}

TEST(ValType, Smoke) {
  EXPECT_EQ(ValType(ValKind::I32)->kind(), ValKind::I32);
  EXPECT_EQ(ValType(ValKind::I64)->kind(), ValKind::I64);
  EXPECT_EQ(ValType(ValKind::F32)->kind(), ValKind::F32);
  EXPECT_EQ(ValType(ValKind::F64)->kind(), ValKind::F64);
  EXPECT_EQ(ValType(ValKind::V128)->kind(), ValKind::V128);
  EXPECT_EQ(ValType(ValKind::FuncRef)->kind(), ValKind::FuncRef);
  EXPECT_EQ(ValType(ValKind::ExternRef)->kind(), ValKind::ExternRef);

  ValType t(ValKind::I32);
  t = ValKind::I64;
  ValType t2(ValKind::F32);
  t = t2;
  ValType t3(t2);

  ValType t4(**t);
  ValType::Ref r(t4);
}

TEST(MemoryType, Smoke) {
  MemoryType t(1);

  EXPECT_EQ(t->min(), 1);
  EXPECT_EQ(t->max(), std::nullopt);
  MemoryType t2 = t;
  t2 = t;
}

TEST(TableType, Smoke) {
  TableType t(ValKind::FuncRef, 1);

  EXPECT_EQ(t->min(), 1);
  EXPECT_EQ(t->max(), std::nullopt);
  EXPECT_EQ(t->element().kind(), ValKind::FuncRef);

  TableType t2 = t;
  t2 = t;
}

TEST(GlobalType, Smoke) {
  GlobalType t(ValKind::FuncRef, true);

  EXPECT_EQ(t->content().kind(), ValKind::FuncRef);
  EXPECT_TRUE(t->is_mutable());

  GlobalType t2 = t;
  t2 = t;
}

TEST(FuncType, Smoke) {
  FuncType t({}, {});
  EXPECT_EQ(t->params().size(), 0);
  EXPECT_EQ(t->results().size(), 0);

  auto other = t;
  other = t;

  FuncType t2({ValKind::I32}, {ValKind::I64});
  EXPECT_EQ(t2->params().size(), 1);
  for (auto ty : t2->params()) {
    EXPECT_EQ(ty.kind(), ValKind::I32);
  }
  EXPECT_EQ(t2->results().size(), 1);
  for (auto ty : t2->results()) {
    EXPECT_EQ(ty.kind(), ValKind::I64);
  }
}

TEST(ModuleType, Smoke) {
  Engine engine;
  Module module = unwrap(Module::compile(engine, "(module)"));
  EXPECT_EQ(module.imports().size(), 0);
  EXPECT_EQ(module.exports().size(), 0);

  module =
      unwrap(Module::compile(engine, "(module"
                                     "(import \"a\" \"b\" (func))"
                                     "(global (export \"x\") i32 (i32.const 0))"
                                     ")"));

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

  auto exports = module.exports();
  EXPECT_EQ(exports.size(), 1);
  auto e = *exports.begin();
  EXPECT_EQ(e.name(), "x");
  auto export_ty = std::get<GlobalType::Ref>(ExternType::from_export(e));
  EXPECT_EQ(export_ty.content().kind(), ValKind::I32);
  EXPECT_FALSE(export_ty.is_mutable());

  for (auto &exp : exports) {
  }

  auto other_imports = module.imports();
  other_imports = std::move(imports);
  ImportType::List last_imports(std::move(other_imports));

  auto other_exports = module.exports();
  other_exports = std::move(exports);
  ExportType::List last_exports(std::move(other_exports));
}

TEST(MemoryType, SixtyFour) {
  MemoryType t(1);
  EXPECT_FALSE(t->is_64());
  t = MemoryType::New64(1);
  EXPECT_TRUE(t->is_64());
  EXPECT_EQ(t->min(), 1);
  EXPECT_EQ(t->max(), std::nullopt);

  t = MemoryType::New64(0x100000000, 0x100000001);
  EXPECT_TRUE(t->is_64());
  EXPECT_EQ(t->min(), 0x100000000);
  EXPECT_EQ(t->max(), 0x100000001);
}
