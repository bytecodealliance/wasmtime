#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>

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
  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  wasm_byte_vec_t wasm;
  auto err = wasmtime_wat2wasm(module_wat.data(), module_wat.size(), &wasm);
  CHECK_ERR(err);

  wasmtime_module_t *module = nullptr;
  err = wasmtime_module_new(
      engine, reinterpret_cast<const uint8_t *>(wasm.data), wasm.size, &module);
  CHECK_ERR(err);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  const auto context = wasmtime_store_context(store);

  wasmtime_component_t *component = nullptr;

  err = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(component_text.data()),
      component_text.size(), &component);

  CHECK_ERR(err);

  const auto linker = wasmtime_component_linker_new(engine);

  const auto root = wasmtime_component_linker_root(linker);

  wasmtime_component_linker_instance_t *x_y_z = nullptr;
  err = wasmtime_component_linker_instance_add_instance(
      root, "x:y/z", strlen("x:y/z"), &x_y_z);
  CHECK_ERR(err);

  err = wasmtime_component_linker_instance_add_module(x_y_z, "mod",
                                                      strlen("mod"), module);
  CHECK_ERR(err);

  wasmtime_component_linker_instance_delete(x_y_z);
  wasmtime_component_linker_instance_delete(root);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context, component,
                                              &instance);
  CHECK_ERR(err);

  wasmtime_component_linker_delete(linker);

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
}
