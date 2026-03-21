#include <wasmtime/types/tag.hh>

#include <gtest/gtest.h>
#include <wasmtime.hh>

using namespace wasmtime;

// Basic TagType construction and param inspection via functype.
TEST(TagType, Simple) {
  Config config;
  config.wasm_exceptions(true);
  Engine engine(std::move(config));

  // Tag with no payload: empty functype.
  FuncType empty_ft({}, {});
  TagType empty(empty_ft);
  auto empty_func = empty->functype();
  EXPECT_EQ(empty_func->params().size(), 0u);

  // Tag with i32 and i64 payload types.
  FuncType ft({ValKind::I32, ValKind::I64}, {});
  TagType t(ft);
  auto func = t->functype();
  auto params = func->params();
  EXPECT_EQ(params.size(), 2u);
  auto it = params.begin();
  EXPECT_EQ(it->kind(), ValKind::I32);
  ++it;
  EXPECT_EQ(it->kind(), ValKind::I64);

  // Copy.
  auto t2 = t;
  EXPECT_EQ(t2->functype()->params().size(), 2u);
}

// Verify that a module exporting an exception tag can have its exports
// enumerated without panicking (regression test for wasmtime issue #10252).
// Previously, CExternType::new() hit todo!() for ExternType::Tag.
TEST(TagType, ModuleExportEnumeration) {
  Config config;
  config.wasm_exceptions(true);
  Engine engine(std::move(config));

  // Compile a module that exports a tag.  The tag has an i32 payload.
  Module module = Module::compile(engine, "(module"
                                          "  (tag $t (param i32))"
                                          "  (export \"t\" (tag $t))"
                                          ")")
                      .unwrap();

  auto exports = module.exports();
  ASSERT_EQ(exports.size(), 1);

  auto e = *exports.begin();
  EXPECT_EQ(e.name(), "t");

  auto extern_ty = ExternType::from_export(e);
  ASSERT_TRUE(std::holds_alternative<TagType::Ref>(extern_ty));

  auto tag_ref = std::get<TagType::Ref>(extern_ty);
  auto func = tag_ref.functype();
  auto params = func->params();
  ASSERT_EQ(params.size(), 1u);
  EXPECT_EQ(params.begin()->kind(), ValKind::I32);
}

// Verify that wasm_externtype_kind returns WASM_EXTERN_TAG for tag exports
// and that the C-level cast functions work correctly.
TEST(TagType, ExternTypeKindAndCast) {
  Config config;
  config.wasm_exceptions(true);
  Engine engine(std::move(config));

  Module module = Module::compile(engine, "(module"
                                          "  (tag $t)"
                                          "  (export \"t\" (tag $t))"
                                          ")")
                      .unwrap();

  auto exports = module.exports();
  ASSERT_EQ(exports.size(), 1);

  auto e = *exports.begin();

  // Access the raw C export type to verify kind and cast functions.
  const wasm_exporttype_t *raw_et =
      *reinterpret_cast<const wasm_exporttype_t *const *>(&e);
  const wasm_externtype_t *ext = wasm_exporttype_type(raw_et);

  EXPECT_EQ(wasm_externtype_kind(ext), WASM_EXTERN_TAG);
  EXPECT_NE(wasm_externtype_as_tagtype_const(ext), nullptr);
  EXPECT_EQ(wasm_externtype_as_functype_const(ext), nullptr);
  EXPECT_EQ(wasm_externtype_as_globaltype_const(ext), nullptr);
  EXPECT_EQ(wasm_externtype_as_memorytype_const(ext), nullptr);
  EXPECT_EQ(wasm_externtype_as_tabletype_const(ext), nullptr);
}
