#include <wasmtime/types/export.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

TEST(ExportType, Smoke) {
  Engine engine;
  Module module = Module::compile(engine, "(module)").unwrap();
  EXPECT_EQ(module.exports().size(), 0);

  module = Module::compile(engine, "(module"
                                   "(global (export \"x\") i32 (i32.const 0))"
                                   ")").unwrap();

  auto exports = module.exports();
  EXPECT_EQ(exports.size(), 1);
  auto e = *exports.begin();
  EXPECT_EQ(e.name(), "x");
  auto export_ty = std::get<GlobalType::Ref>(ExternType::from_export(e));
  EXPECT_EQ(export_ty.content().kind(), ValKind::I32);
  EXPECT_FALSE(export_ty.is_mutable());

  for (auto &exp : exports) {
  }

  auto other_exports = module.exports();
  other_exports = std::move(exports);
  ExportType::List last_exports(std::move(other_exports));
}
