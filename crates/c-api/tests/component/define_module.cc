#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component/component.hh>
#include <wasmtime/module.hh>
#include <wasmtime/store.hh>

using namespace wasmtime;
using namespace wasmtime::component;

TEST(component, define_module) {
  static constexpr auto module_wat = std::string_view{
      R"END(
(module
    (func $function (param $x i32) (result i32)
        local.get $x)
    (export "function" (func $function))
)
      )END",
  };

  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (import "x:y/z" (instance
        (export "mod" (core module
            (export "function" (func (param i32) (result i32)))
        ))
    ))
)
      )END",
  };

  Engine engine;
  Module module = Module::compile(engine, module_wat).unwrap();
  Store store(engine);
  auto context = store.context();

  Component component = Component::compile(engine, component_text).unwrap();

  const auto linker = wasmtime_component_linker_new(engine.capi());

  const auto root = wasmtime_component_linker_root(linker);

  wasmtime_component_linker_instance_t *x_y_z = nullptr;
  auto err = wasmtime_component_linker_instance_add_instance(
      root, "x:y/z", strlen("x:y/z"), &x_y_z);
  CHECK_ERR(err);

  err = wasmtime_component_linker_instance_add_module(
      x_y_z, "mod", strlen("mod"), module.capi());
  CHECK_ERR(err);

  wasmtime_component_linker_instance_delete(x_y_z);
  wasmtime_component_linker_instance_delete(root);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context.capi(),
                                              component.capi(), &instance);
  CHECK_ERR(err);

  wasmtime_component_linker_delete(linker);
}
