#include "component/utils.h"

#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime;
using namespace wasmtime::component;

TEST(wasip2, smoke) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component)
      )END",
  };

  Engine engine;
  Store store(engine);
  auto context = store.context();

  const auto cfg = wasmtime_wasip2_config_new();
  wasmtime_wasip2_config_inherit_stdin(cfg);
  wasmtime_wasip2_config_inherit_stdout(cfg);
  wasmtime_wasip2_config_inherit_stderr(cfg);
  wasmtime_wasip2_config_arg(cfg, "hello", strlen("hello"));
  wasmtime_context_set_wasip2(context.capi(), cfg);

  Component component = Component::compile(engine, component_text).unwrap();

  const auto linker = wasmtime_component_linker_new(engine.capi());

  wasmtime_component_linker_add_wasip2(linker);

  wasmtime_component_instance_t instance = {};
  auto err = wasmtime_component_linker_instantiate(linker, context.capi(),
                                                   component.capi(), &instance);
  CHECK_ERR(err);

  wasmtime_component_linker_delete(linker);
}
