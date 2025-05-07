#include "utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>

TEST(component, instantiate) {
  static constexpr auto bytes = std::string_view{
      R"END(
      (component
          (core module)
      )
      )END",
  };

  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  EXPECT_NE(store, nullptr);
  const auto context = wasmtime_store_context(store);
  EXPECT_NE(context, nullptr);

  wasmtime_component_t *component = nullptr;

  auto error = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(bytes.data()), bytes.size(),
      &component);

  CHECK_ERR(error);

  const auto linker = wasmtime_component_linker_new(engine);
  EXPECT_NE(linker, nullptr);

  wasmtime_component_instance_t instance = {};
  error = wasmtime_component_linker_instantiate(linker, context, component,
                                                &instance);

  CHECK_ERR(error);

  wasmtime_component_linker_delete(linker);

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
}
