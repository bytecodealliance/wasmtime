#include <gtest/gtest.h>
#include <wasmtime/component.hh>
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
  Linker linker(engine);

  linker.instantiate(context, component).unwrap();
}
