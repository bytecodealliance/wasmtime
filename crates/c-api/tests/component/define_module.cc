#include <gtest/gtest.h>
#include <wasmtime/component.hh>
#include <wasmtime/module.hh>
#include <wasmtime/store.hh>

using namespace wasmtime::component;

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

  wasmtime::Engine engine;
  wasmtime::Module module =
      wasmtime::Module::compile(engine, module_wat).unwrap();
  wasmtime::Store store(engine);
  auto context = store.context();

  Component component = Component::compile(engine, component_text).unwrap();
  Linker linker(engine);

  {
    auto root = linker.root();
    auto xyz = root.add_instance("x:y/z").unwrap();
    xyz.add_module("mod", module).unwrap();
  }

  linker.instantiate(context, component).unwrap();
}
