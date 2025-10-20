#include <array>
#include <gtest/gtest.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

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

  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();
  auto component = Component::compile(engine, component_text).unwrap();
  auto f = *component.export_index(nullptr, "f");

  Linker linker(engine);

  auto instance = linker.instantiate(context, component).unwrap();
  auto func = *instance.get_func(context, f);

  auto params = std::array<Val, 2>{
      uint32_t(34),
      uint32_t(35),
  };

  auto results = std::array<Val, 1>{false};

  func.call(context, params, results).unwrap();

  func.post_return(context).unwrap();

  EXPECT_TRUE(results[0].is_u32());
  EXPECT_EQ(results[0].get_u32(), 69);
}
