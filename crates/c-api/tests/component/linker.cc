#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>

using namespace wasmtime;
using namespace wasmtime::component;

TEST(Linker, allow_shadowing) {
  Engine engine;
  Linker linker(engine);
  Module m = Module::compile(engine, "(module)").unwrap();

  linker.root().add_module("x", m).unwrap();
  linker.root().add_module("x", m).err();
  linker.allow_shadowing(true);
  linker.root().add_module("x", m).unwrap();
}
