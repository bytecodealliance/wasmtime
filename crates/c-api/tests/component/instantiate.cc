#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime;
using namespace wasmtime::component;

TEST(component, instantiate) {
  static constexpr auto bytes = std::string_view{
      R"END(
      (component
          (core module)
      )
      )END",
  };

  Engine engine;
  Store store(engine);
  auto context = store.context();
  Component component = Component::compile(engine, bytes).unwrap();

  const auto linker = wasmtime_component_linker_new(engine.capi());
  EXPECT_NE(linker, nullptr);

  wasmtime_component_instance_t instance = {};
  auto error = wasmtime_component_linker_instantiate(
      linker, context.capi(), component.capi(), &instance);

  CHECK_ERR(error);

  wasmtime_component_linker_delete(linker);
}
