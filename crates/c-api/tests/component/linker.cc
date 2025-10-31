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
