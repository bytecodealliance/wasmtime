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
  EXPECT_EQ(ValType::i32(), ValType::i32());
  EXPECT_EQ(ValType::i64(), ValType::i64());
  EXPECT_EQ(ValType::f32(), ValType::f32());
  EXPECT_EQ(ValType::f64(), ValType::f64());
  EXPECT_EQ(ValType::v128(), ValType::v128());
  EXPECT_EQ(ValType::funcref(), ValType::funcref());
  EXPECT_EQ(ValType::externref(), ValType::externref());
  EXPECT_EQ(ValType::exnref(), ValType::exnref());
  EXPECT_EQ(ValType::anyref(), ValType::anyref());

  ValType t(ValType::i32());
  t = ValType::i64();
  ValType t2(ValType::f32());
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
  TableType t(ValType::funcref(), 1);

  EXPECT_EQ(t->min(), 1);
  EXPECT_EQ(t->max(), std::nullopt);
  EXPECT_EQ(t->element(), ValType::funcref());

  TableType t2 = t;
  t2 = t;
}

TEST(GlobalType, Smoke) {
  GlobalType t(ValType::funcref(), true);

  EXPECT_EQ(t->content(), ValType::funcref());
  EXPECT_TRUE(t->is_mutable());

  GlobalType t2 = t;
  t2 = t;
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
  EXPECT_EQ(export_ty.content(), ValType::i32());
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
