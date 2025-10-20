#include <gtest/gtest.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime::component;

TEST(wasip2, smoke) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
  (import "wasi:cli/environment@0.2.0" (instance
    (export "get-arguments" (func (result (list string))))
  ))
)
      )END",
  };

  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();

  wasmtime::WasiConfig config;
  context.set_wasi(std::move(config)).unwrap();
  Component component = Component::compile(engine, component_text).unwrap();

  Linker linker(engine);
  linker.add_wasip2().unwrap();
  linker.instantiate(context, component).unwrap();
}
