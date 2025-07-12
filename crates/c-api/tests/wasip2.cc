#include "component/utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>

TEST(wasip2, smoke) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component)
      )END",
  };
  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  const auto context = wasmtime_store_context(store);

  const auto cfg = wasmtime_wasip2_config_new();
  wasmtime_wasip2_config_inherit_stdin(cfg);
  wasmtime_wasip2_config_inherit_stdout(cfg);
  wasmtime_wasip2_config_inherit_stderr(cfg);
  wasmtime_wasip2_config_arg(cfg, "hello", strlen("hello"));
  wasmtime_context_set_wasip2(context, cfg);

  wasmtime_component_t *component = nullptr;

  auto err = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(component_text.data()),
      component_text.size(), &component);

  CHECK_ERR(err);

  const auto linker = wasmtime_component_linker_new(engine);

  wasmtime_component_linker_add_wasip2(linker);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context, component,
                                              &instance);
  CHECK_ERR(err);

  wasmtime_component_linker_delete(linker);

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
}
