#include <gtest/gtest.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime::component;

TEST(component, lookup_func) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module $m
        (func (export "f"))
    )
    (core instance $i (instantiate $m))
    (func (export "f")
        (canon lift (core func $i "f")))
)
      )END",
  };

  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();
  Component component = Component::compile(engine, component_text).unwrap();
  auto f = component.export_index(nullptr, "ff");

  EXPECT_FALSE(f);

  f = component.export_index(nullptr, "f");

  EXPECT_TRUE(f);

  Linker linker(engine);

  auto instance = linker.instantiate(context, component).unwrap();

  *instance.get_func(context, *f);

  auto f2 = instance.get_export_index(context, nullptr, "f");
  EXPECT_TRUE(f2);
}

TEST(component, lookup_empty_name) {
  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();
  Component component = Component::compile(engine, "(component)").unwrap();

  // A default `std::string_view` has a null data pointer.
  EXPECT_FALSE(component.export_index(nullptr, std::string_view{}));

  Linker linker(engine);
  auto instance = linker.instantiate(context, component).unwrap();
  EXPECT_FALSE(instance.get_export_index(context, nullptr, std::string_view{}));
}

TEST(component, lookup_export_extern) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module $m
        (func (export "f"))
    )
    (core instance $i (instantiate $m))
    (func $f (canon lift (core func $i "f")))
    (instance $inner
        (export "f" (external-id "inner-f") (func $f))
    )
    (export "a" (implements "a:b/c") (external-id "id-a") (instance $inner))
)
      )END",
  };

  wasmtime::Config config;
  config.wasm_component_model_implements(true);
  wasmtime::Engine engine(std::move(config));
  Component component = Component::compile(engine, component_text).unwrap();

  auto a = component.export_extern(nullptr, "a");
  ASSERT_TRUE(a);
  EXPECT_EQ(a->first.implements(), "a:b/c");
  EXPECT_EQ(a->first.external_id(), "id-a");
  auto f = component.export_extern(&a->second, "f");
  ASSERT_TRUE(f);
  EXPECT_EQ(f->first.external_id(), "inner-f");

  wasmtime::Store store(engine);
  auto context = store.context();
  Linker linker(engine);
  auto instance = linker.instantiate(context, component).unwrap();

  auto a2 = instance.get_export_extern(context, nullptr, "a");
  ASSERT_TRUE(a2);
  EXPECT_EQ(a2->first.implements(), "a:b/c");
  auto f2 = instance.get_export_extern(context, &a2->second, "f");
  ASSERT_TRUE(f2);
  EXPECT_EQ(f2->first.external_id(), "inner-f");
  EXPECT_TRUE(instance.get_func(context, f2->second));
}
