#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>

using namespace wasmtime::component;

TEST(Linker, allow_shadowing) {
  wasmtime::Engine engine;
  Linker linker(engine);
  auto m = wasmtime::Module::compile(engine, "(module)").unwrap();

  linker.root().add_module("x", m).unwrap();
  linker.root().add_module("x", m).err();
  linker.allow_shadowing(true);
  linker.root().add_module("x", m).unwrap();
}

TEST(Linker, unknown_imports_trap) {
  wasmtime::Engine engine;
  Linker linker(engine);
  wasmtime::Store store(engine);

  auto c = Component::compile(engine, R"(
    (component
      (import "a" (func))
    )
  )")
               .unwrap();

  EXPECT_FALSE(linker.instantiate(store, c));
  EXPECT_TRUE(linker.define_unknown_imports_as_traps(c));
  EXPECT_TRUE(linker.instantiate(store, c));
}
