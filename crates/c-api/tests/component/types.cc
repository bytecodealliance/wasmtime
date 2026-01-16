#include <gtest/gtest.h>
#include <wasmtime/component.hh>

using namespace wasmtime::component;
using wasmtime::Config;
using wasmtime::Engine;
using wasmtime::ExternType;
using wasmtime::Result;
using wasmtime::Span;
using wasmtime::Store;

TEST(types, empty_component) {
  Engine engine;

  auto component = Component::compile(engine, R"(
(component)
      )")
                       .unwrap()
                       .type();

  EXPECT_EQ(component.import_count(engine), 0);
  EXPECT_EQ(component.export_count(engine), 0);
}

TEST(types, component_resource) {
  Engine engine;

  auto component = Component::compile(engine, R"(
(component
  (import "x" (type $t (sub resource)))
  (export "x" (type $t))
)
      )")
                       .unwrap()
                       .type();

  EXPECT_EQ(component.import_count(engine), 1);
  auto ty = component.import_get(engine, "x")->resource();
  auto i = *component.import_nth(engine, 0);
  EXPECT_EQ(i.first, "x");
  EXPECT_EQ(ty, i.second.resource());

  EXPECT_EQ(component.export_count(engine), 1);

  EXPECT_EQ(component.export_get(engine, "x")->resource(), ty);
  auto e = *component.import_nth(engine, 0);
  EXPECT_EQ(e.first, "x");
  EXPECT_EQ(ty, e.second.resource());
}

TEST(types, component_instance) {
  Engine engine;

  auto component = Component::compile(engine, R"(
(component
  (import "x" (instance $a (export "t" (type (sub resource)))))
  (export "x" (instance $a))
)
      )")
                       .unwrap()
                       .type();

  auto ty = component.import_get(engine, "x")->component_instance();
  EXPECT_EQ(ty.export_count(engine), 1);
  auto resource_ty = ty.export_get(engine, "t")->resource();
  EXPECT_EQ(resource_ty, ty.export_nth(engine, 0)->second.resource());
  EXPECT_EQ("t", ty.export_nth(engine, 0)->first);

  auto ty2 = component.export_get(engine, "x")->component_instance();
  EXPECT_EQ(resource_ty, ty2.export_get(engine, "t")->resource());
}

TEST(types, component_func) {
  Engine engine;

  auto component = Component::compile(engine, R"(
(component
  (import "x" (func))
)
      )")
                       .unwrap()
                       .type();

  auto ty = component.import_get(engine, "x")->component_func();
  EXPECT_EQ(ty.param_count(), 0);
  EXPECT_FALSE(ty.result());

  component = Component::compile(engine, R"(
(component
  (import "x" (func (param "x" u32) (result string)))
)
      )")
                  .unwrap()
                  .type();

  ty = component.import_get(engine, "x")->component_func();
  EXPECT_EQ(ty.param_count(), 1);
  EXPECT_EQ(ty.param_nth(0)->first, "x");
  EXPECT_TRUE(ty.param_nth(0)->second.is_u32());
  EXPECT_EQ(ty.param_nth(0)->second, ValType::new_u32());

  EXPECT_TRUE(ty.result()->is_string());
  EXPECT_EQ(*ty.result(), ValType::new_string());
}

TEST(types, module_type) {
  Engine engine;

  auto component = Component::compile(engine, R"(
(component
  (import "x" (core module))
)
      )")
                       .unwrap();
  auto cty = component.type();

  auto ty = cty.import_get(engine, "x")->module();
  EXPECT_EQ(ty.import_count(engine), 0);
  EXPECT_EQ(ty.export_count(engine), 0);

  component = Component::compile(engine, R"(
(component
  (import "x" (core module
    (import "" "" (func))
    (export "x" (global (mut i32)))
  ))
)
      )")
                  .unwrap();
  cty = component.type();

  ty = cty.import_get(engine, "x")->module();
  EXPECT_EQ(ty.import_count(engine), 1);
  auto import = *ty.import_nth(engine, 0);
  EXPECT_EQ(import.ref().module(), "");
  EXPECT_EQ(import.ref().name(), "");

  auto import_item_ty = ExternType::from_import(import.ref());
  auto func_ty = std::get<wasmtime::FuncType::Ref>(import_item_ty);
  EXPECT_EQ(func_ty.params().size(), 0);
  EXPECT_EQ(func_ty.results().size(), 0);

  auto export_ty = ty.export_nth(engine, 0);
  EXPECT_EQ(export_ty->ref().name(), "x");
  auto export_item_ty = ExternType::from_export(export_ty->ref());
  auto global_ty = std::get<wasmtime::GlobalType::Ref>(export_item_ty);
  EXPECT_EQ(global_ty.content().kind(), wasmtime::ValKind::I32);
  EXPECT_TRUE(global_ty.is_mutable());
}

static ValType result(const char *wat) {
  Engine engine;
  auto component = Component::compile(engine, wat).unwrap();
  return *component.type().import_get(engine, "f")->component_func().result();
}

TEST(types, valtype_primitives) {
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result bool))))").is_bool());
  EXPECT_TRUE(result("(component (import \"f\" (func (result u8))))").is_u8());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result u16))))").is_u16());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result u32))))").is_u32());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result u64))))").is_u64());
  EXPECT_TRUE(result("(component (import \"f\" (func (result s8))))").is_s8());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result s16))))").is_s16());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result s32))))").is_s32());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result s64))))").is_s64());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result f32))))").is_f32());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result f64))))").is_f64());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result char))))").is_char());
  EXPECT_TRUE(
      result("(component (import \"f\" (func (result string))))").is_string());
}

TEST(types, valtype_list) {
  auto ty = result("(component (import \"f\" (func (result (list u8)))))");
  EXPECT_TRUE(ty.is_list());
  auto elem = ty.list().element();
  EXPECT_TRUE(elem.is_u8());
}

TEST(types, valtype_map) {
  Config config;
  config.wasm_component_model_map(true);
  Engine engine(std::move(config));
  auto component =
      Component::compile(engine,
                         "(component (import \"f\" (func (result (map u32 string)))))")
          .unwrap();
  auto ty = *component.type().import_get(engine, "f")->component_func().result();
  EXPECT_TRUE(ty.is_map());
  auto map_ty = ty.map();
  EXPECT_TRUE(map_ty.key().is_u32());
  EXPECT_TRUE(map_ty.value().is_string());
}

TEST(types, valtype_record) {
  auto ty = result(R"(
  (component
    (type $t' (record (field "a" u8) (field "b" u16)))
    (import "t" (type $t (eq $t')))
    (import "f" (func (result $t)))
  )
  )");
  EXPECT_TRUE(ty.is_record());
  EXPECT_EQ(ty.record().field_count(), 2);
  auto [name, field_ty] = *ty.record().field_nth(0);
  EXPECT_EQ(name, "a");
  EXPECT_TRUE(field_ty.is_u8());
  auto [name2, field_ty2] = *ty.record().field_nth(1);
  EXPECT_EQ(name2, "b");
  EXPECT_TRUE(field_ty2.is_u16());
}

TEST(types, valtype_tuple) {
  auto ty = result("(component (import \"f\" (func (result (tuple u16 u8)))))");
  EXPECT_TRUE(ty.is_tuple());
  EXPECT_EQ(ty.tuple().types_count(), 2);
  EXPECT_TRUE(ty.tuple().types_nth(0)->is_u16());
  EXPECT_TRUE(ty.tuple().types_nth(1)->is_u8());
}

TEST(types, valtype_variant) {
  auto ty = result(R"(
  (component
    (type $t' (variant (case "a") (case "b" u16)))
    (import "t" (type $t (eq $t')))
    (import "f" (func (result $t)))
  )
  )");
  EXPECT_TRUE(ty.is_variant());
  EXPECT_EQ(ty.variant().case_count(), 2);
  auto [name, case_ty] = *ty.variant().case_nth(0);
  EXPECT_EQ(name, "a");
  EXPECT_FALSE(case_ty.has_value());
  auto [name2, case_ty2] = *ty.variant().case_nth(1);
  EXPECT_EQ(name2, "b");
  EXPECT_TRUE(case_ty2->is_u16());
}

TEST(types, valtype_enum) {
  auto ty = result(R"(
  (component
    (type $t' (enum "a" "b" "c"))
    (import "t" (type $t (eq $t')))
    (import "f" (func (result $t)))
  )
  )");
  EXPECT_TRUE(ty.is_enum());
  auto enum_ = ty.enum_();
  EXPECT_EQ(enum_.names_count(), 3);
  EXPECT_EQ(enum_.names_nth(0), "a");
  EXPECT_EQ(enum_.names_nth(1), "b");
  EXPECT_EQ(enum_.names_nth(2), "c");
}

TEST(types, valtype_flags) {
  auto ty = result(R"(
  (component
    (type $t' (flags "a" "b" "c"))
    (import "t" (type $t (eq $t')))
    (import "f" (func (result $t)))
  )
  )");
  EXPECT_TRUE(ty.is_flags());
  auto flags = ty.flags();
  EXPECT_EQ(flags.names_count(), 3);
  EXPECT_EQ(flags.names_nth(0), "a");
  EXPECT_EQ(flags.names_nth(1), "b");
  EXPECT_EQ(flags.names_nth(2), "c");
}

TEST(types, valtype_option) {
  auto ty = result(R"(
  (component (import "f" (func (result (option u8)))))
  )");
  EXPECT_TRUE(ty.is_option());
  EXPECT_TRUE(ty.option().ty().is_u8());
}

TEST(types, valtype_result) {
  auto ty = result(R"(
  (component (import "f" (func (result (result u8)))))
  )");
  EXPECT_TRUE(ty.is_result());
  EXPECT_TRUE(ty.result().ok()->is_u8());
  EXPECT_FALSE(ty.result().err().has_value());

  ty = result(R"(
  (component (import "f" (func (result (result (error u8))))))
  )");
  EXPECT_TRUE(ty.is_result());
  EXPECT_FALSE(ty.result().ok().has_value());
  EXPECT_TRUE(ty.result().err()->is_u8());

  ty = result(R"(
  (component (import "f" (func (result (result)))))
  )");
  EXPECT_TRUE(ty.is_result());
  EXPECT_FALSE(ty.result().ok().has_value());
  EXPECT_FALSE(ty.result().err().has_value());
}

TEST(types, func_result) {
  Engine engine;
  auto wat = R"(
(component
  (core module $m
    (func (export "f1"))
    (func (export "f2") (param i32) (result i32) unreachable)
  )
  (core instance $i (instantiate $m))
  (func (export "f1") (canon lift (core func $i "f1")))
  (func (export "f2") (param "x" u32) (result u32)
    (canon lift (core func $i "f2")))
)
)";
  auto component = Component::compile(engine, wat).unwrap();
  Store store(engine);
  auto instance = Linker(engine).instantiate(store, component).unwrap();
  auto f1_index = *instance.get_export_index(store, nullptr, "f1");
  auto f2_index = *instance.get_export_index(store, nullptr, "f2");
  auto f1 = *instance.get_func(store, f1_index);
  auto f2 = *instance.get_func(store, f2_index);

  EXPECT_FALSE(f1.type(store).async());
  EXPECT_EQ(f1.type(store).param_count(), 0);
  EXPECT_FALSE(f1.type(store).result().has_value());

  const auto &ty = f2.type(store);
  EXPECT_EQ(ty.param_count(), 1);
  const auto [name, param_ty] = *ty.param_nth(0);
  EXPECT_EQ(name, "x");
  EXPECT_TRUE(param_ty.is_u32());
  auto result = ty.result();
  ASSERT_TRUE(result.has_value());
  EXPECT_TRUE(result->is_u32());
}
