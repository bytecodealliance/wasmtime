#include "utils.h"

#include <array>
#include <gtest/gtest.h>
#include <wasmtime.h>

TEST(component, call_func) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module $m
        (func (export "f") (param $x i32) (param $y i32) (result i32)
            (local.get $x)
            (local.get $y)
            (i32.add)
        )
    )
    (core instance $i (instantiate $m))
    (func $f (param "x" u32) (param "y" u32) (result u32) (canon lift (core func $i "f")))
    (export "f" (func $f))
)
      )END",
  };
  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  const auto context = wasmtime_store_context(store);

  wasmtime_component_t *component = nullptr;

  auto err = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(component_text.data()),
      component_text.size(), &component);

  CHECK_ERR(err);

  const auto f =
      wasmtime_component_get_export_index(component, nullptr, "f", 1);

  EXPECT_NE(f, nullptr);

  const auto linker = wasmtime_component_linker_new(engine);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context, component,
                                              &instance);
  CHECK_ERR(err);

  wasmtime_component_func_t func = {};
  const auto found =
      wasmtime_component_instance_get_func(&instance, context, f, &func);
  EXPECT_TRUE(found);

  auto params = std::array<wasmtime_component_val_t, 2>{
      wasmtime_component_val_t{
          .kind = WASMTIME_COMPONENT_U32,
          .of = {.u32 = 34},
      },
      wasmtime_component_val_t{
          .kind = WASMTIME_COMPONENT_U32,
          .of = {.u32 = 35},
      },
  };

  auto results = std::array<wasmtime_component_val_t, 1>{};

  err =
      wasmtime_component_func_call(&func, context, params.data(), params.size(),
                                   results.data(), results.size());
  CHECK_ERR(err);

  err = wasmtime_component_func_post_return(&func, context);
  CHECK_ERR(err);

  EXPECT_EQ(results[0].kind, WASMTIME_COMPONENT_U32);
  EXPECT_EQ(results[0].of.u32, 69);

  wasmtime_component_export_index_delete(f);
  wasmtime_component_linker_delete(linker);

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
}
