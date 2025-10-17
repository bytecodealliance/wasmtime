#include "utils.h"

#include <array>
#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime;
using namespace wasmtime::component;

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

  Engine engine;
  Store store(engine);
  auto context = store.context();
  auto component = Component::compile(engine, component_text).unwrap();
  auto f = *component.export_index(nullptr, "f");

  Linker linker(engine);

  auto instance = linker.instantiate(context, component).unwrap();
  auto func = *instance.get_func(context, f);

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

  auto err = wasmtime_component_func_call(func.capi(), context.capi(),
                                          params.data(), params.size(),
                                          results.data(), results.size());
  CHECK_ERR(err);

  func.post_return(context).unwrap();

  EXPECT_EQ(results[0].kind, WASMTIME_COMPONENT_U32);
  EXPECT_EQ(results[0].of.u32, 69);
}
