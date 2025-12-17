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
