#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime;
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

  Engine engine;
  Store store(engine);
  auto context = store.context();
  Component component = Component::compile(engine, component_text).unwrap();
  auto f = component.export_index(nullptr, "ff");

  EXPECT_FALSE(f);

  f = component.export_index(nullptr, "f");

  EXPECT_TRUE(f);

  Linker linker(engine);

  wasmtime_component_instance_t instance = {};
  auto err = wasmtime_component_linker_instantiate(
      linker.capi(), context.capi(), component.capi(), &instance);
  CHECK_ERR(err);

  wasmtime_component_func_t func = {};
  const auto found = wasmtime_component_instance_get_func(
      &instance, context.capi(), f->capi(), &func);
  EXPECT_TRUE(found);
  EXPECT_NE(func.store_id, 0);

  auto f2 = wasmtime_component_instance_get_export_index(
      &instance, context.capi(), nullptr, "f", strlen("f"));
  EXPECT_NE(f2, nullptr);

  wasmtime_component_export_index_delete(f2);
}
